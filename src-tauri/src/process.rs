//! Process management for tunnel-rs client instances

use crate::config::TunnelClientConfig;
use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/// Status of a tunnel instance
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelStatus {
    Stopped,
    Starting,
    Running,
    Error,
}

/// Log entry from a tunnel process
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub message: String,
    pub is_error: bool,
}

/// Enum to hold either a tokio Child or a sidecar CommandChild
pub enum ChildProcess {
    Tokio(Child),
    Sidecar(CommandChild),
}

/// A running tunnel instance
pub struct TunnelInstance {
    pub forwarding_id: Uuid,
    pub forwarding_name: String,
    pub server_group_name: String,
    pub status: TunnelStatus,
    pub child: Option<ChildProcess>,
    pub logs: VecDeque<LogEntry>,
}

impl TunnelInstance {
    fn new(forwarding_id: Uuid, forwarding_name: String, server_group_name: String) -> Self {
        Self {
            forwarding_id,
            forwarding_name,
            server_group_name,
            status: TunnelStatus::Stopped,
            child: None,
            logs: VecDeque::new(),
        }
    }

    fn add_log(&mut self, message: String, is_error: bool) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Keep last 500 log entries (O(1) pop_front with VecDeque)
        if self.logs.len() >= 500 {
            self.logs.pop_front();
        }

        self.logs.push_back(LogEntry {
            timestamp,
            message,
            is_error,
        });
    }
}

/// Serializable view of a tunnel instance
#[derive(Debug, Clone, serde::Serialize)]
pub struct TunnelInstanceView {
    pub forwarding_id: Uuid,
    pub forwarding_name: String,
    pub server_group_name: String,
    pub status: TunnelStatus,
    pub logs: VecDeque<LogEntry>,
}

impl From<&TunnelInstance> for TunnelInstanceView {
    fn from(instance: &TunnelInstance) -> Self {
        Self {
            forwarding_id: instance.forwarding_id,
            forwarding_name: instance.forwarding_name.clone(),
            server_group_name: instance.server_group_name.clone(),
            status: instance.status.clone(),
            logs: instance.logs.clone(),
        }
    }
}

/// Manager for all tunnel processes
pub struct ProcessManager {
    instances: RwLock<HashMap<Uuid, Arc<Mutex<TunnelInstance>>>>,
    custom_binary_path: RwLock<Option<String>>,
    app_handle: RwLock<Option<AppHandle>>,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            custom_binary_path: RwLock::new(None),
            app_handle: RwLock::new(None),
        }
    }

    /// Set the app handle for sidecar spawning
    pub async fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.write().await = Some(handle);
    }

    /// Set custom binary path (None means use bundled sidecar)
    pub async fn set_custom_binary_path(&self, path: Option<String>) {
        *self.custom_binary_path.write().await = path;
    }

    /// Check if using bundled binary
    pub async fn is_using_bundled(&self) -> bool {
        self.custom_binary_path.read().await.is_none()
    }

    /// Get the version of the currently configured binary
    pub async fn get_binary_version(&self) -> Result<String, String> {
        let custom_path = self.custom_binary_path.read().await.clone();

        if let Some(binary_path) = custom_path {
            let output = tokio::process::Command::new(&binary_path)
                .arg("--version")
                .output()
                .await
                .map_err(|e| format!("Failed to run binary: {}", e))?;

            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(version_str)
        } else {
            let app_handle = self
                .app_handle
                .read()
                .await
                .clone()
                .ok_or_else(|| "App handle not set".to_string())?;

            let sidecar_command = app_handle
                .shell()
                .sidecar("tunnel-rs")
                .map_err(|e| format!("Failed to create sidecar command: {}", e))?
                .args(["--version"]);

            let output = sidecar_command
                .output()
                .await
                .map_err(|e| format!("Failed to run sidecar: {}", e))?;

            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(version_str)
        }
    }

    /// Get all running instances
    pub async fn list_instances(&self) -> Vec<TunnelInstanceView> {
        let instances = self.instances.read().await;
        let mut views = Vec::new();
        for instance in instances.values() {
            let guard = instance.lock().await;
            views.push(TunnelInstanceView::from(&*guard));
        }
        views.sort_by(|a, b| a.forwarding_name.cmp(&b.forwarding_name));
        views
    }

    /// Get a specific instance by forwarding_id
    pub async fn get_instance(&self, forwarding_id: Uuid) -> Option<TunnelInstanceView> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&forwarding_id) {
            let guard = instance.lock().await;
            Some(TunnelInstanceView::from(&*guard))
        } else {
            None
        }
    }

    /// Start a tunnel with the given forwarding info and built config
    pub async fn start(
        &self,
        forwarding_id: Uuid,
        forwarding_name: &str,
        server_group_name: &str,
        config: &TunnelClientConfig,
    ) -> Result<(), String> {
        // Create instance and insert atomically under write lock to prevent TOCTOU
        let instance = {
            let mut instances = self.instances.write().await;

            // Check if already running while holding write lock
            if let Some(existing) = instances.get(&forwarding_id) {
                let guard = existing.lock().await;
                match guard.status {
                    TunnelStatus::Running | TunnelStatus::Starting => {
                        return Err("Tunnel is already running".to_string());
                    }
                    _ => {}
                }
            }

            // Create and insert instance while still holding write lock
            let instance = Arc::new(Mutex::new(TunnelInstance::new(
                forwarding_id,
                forwarding_name.to_string(),
                server_group_name.to_string(),
            )));
            {
                let mut guard = instance.lock().await;
                guard.status = TunnelStatus::Starting;
                guard.add_log("Starting tunnel...".to_string(), false);
            }
            instances.insert(forwarding_id, instance.clone());

            instance
            // Write lock released here
        };

        // Serialize config to JSON for piping via stdin
        let json_config = match config.to_json() {
            Ok(content) => content,
            Err(e) => {
                {
                    let mut guard = instance.lock().await;
                    guard.status = TunnelStatus::Error;
                    guard.add_log(format!("Failed to serialize config: {}", e), true);
                }
                let mut instances = self.instances.write().await;
                instances.remove(&forwarding_id);
                return Err(format!("Failed to serialize config: {}", e));
            }
        };

        // Check if we should use custom binary or sidecar
        let custom_path = self.custom_binary_path.read().await.clone();

        if let Some(binary_path) = custom_path {
            self.start_with_custom_binary(&instance, forwarding_id, &binary_path, &json_config)
                .await
        } else {
            self.start_with_sidecar(&instance, forwarding_id, &json_config)
                .await
        }
    }

    /// Clean up a failed instance: set error status, log message, remove from map
    async fn cleanup_failed_instance(
        &self,
        instance: &Arc<Mutex<TunnelInstance>>,
        forwarding_id: Uuid,
        error_msg: &str,
    ) {
        {
            let mut guard = instance.lock().await;
            guard.status = TunnelStatus::Error;
            guard.add_log(error_msg.to_string(), true);
        }
        let mut instances = self.instances.write().await;
        instances.remove(&forwarding_id);
    }

    /// Start tunnel using custom binary path
    async fn start_with_custom_binary(
        &self,
        instance: &Arc<Mutex<TunnelInstance>>,
        forwarding_id: Uuid,
        binary_path: &str,
        json_config: &str,
    ) -> Result<(), String> {
        // Verify binary exists (async)
        match tokio::fs::try_exists(binary_path).await {
            Ok(true) => {} // Binary exists, continue
            Ok(false) => {
                let error_msg = format!("Custom binary path '{}' does not exist", binary_path);
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
            Err(e) => {
                let error_msg = format!(
                    "Failed to check if custom binary path '{}' exists: {}",
                    binary_path, e
                );
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
        }

        let child = tokio::process::Command::new(binary_path)
            .args(["client", "--config-stdin"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                let error_msg = format!("Failed to spawn tunnel-rs: {}", e);
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
        };

        // Pipe JSON config via stdin, then close stdin
        {
            use tokio::io::AsyncWriteExt;
            let mut stdin = child.stdin.take().expect("stdin was piped");
            if let Err(e) = stdin.write_all(json_config.as_bytes()).await {
                let _ = child.kill().await;
                let error_msg = format!("Failed to write config to stdin: {}", e);
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
            // stdin is dropped here, closing the pipe
        }

        // Take stdout/stderr for log capture
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        {
            let mut guard = instance.lock().await;
            guard.child = Some(ChildProcess::Tokio(child));
            guard.status = TunnelStatus::Running;
            guard.add_log(
                format!("Started with custom binary: {}", binary_path),
                false,
            );
        }

        // Spawn log readers for tokio child
        let instance_clone = instance.clone();
        if let Some(stdout) = stdout {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut guard = instance_clone.lock().await;
                    guard.add_log(line, false);
                }
            });
        }

        let instance_clone = instance.clone();
        if let Some(stderr) = stderr {
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let mut guard = instance_clone.lock().await;
                    guard.add_log(line, true);
                }
            });
        }

        // Spawn process monitor for tokio child
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut guard = instance_clone.lock().await;
                if let Some(ChildProcess::Tokio(ref mut child)) = guard.child {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // Preserve Stopped status if already set by stop()
                            if !matches!(guard.status, TunnelStatus::Stopped) {
                                guard.status = if status.success() {
                                    TunnelStatus::Stopped
                                } else {
                                    TunnelStatus::Error
                                };
                            }
                            guard.add_log(
                                format!("Process exited with status: {}", status),
                                !status.success(),
                            );
                            guard.child = None;
                            break;
                        }
                        Ok(None) => {
                            // Still running
                        }
                        Err(e) => {
                            guard.status = TunnelStatus::Error;
                            guard.add_log(format!("Error checking process status: {}", e), true);
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        });

        Ok(())
    }

    /// Start tunnel using bundled sidecar
    async fn start_with_sidecar(
        &self,
        instance: &Arc<Mutex<TunnelInstance>>,
        forwarding_id: Uuid,
        json_config: &str,
    ) -> Result<(), String> {
        let app_handle = match self.app_handle.read().await.clone() {
            Some(handle) => handle,
            None => {
                let error_msg = "App handle not set";
                self.cleanup_failed_instance(instance, forwarding_id, error_msg).await;
                return Err(error_msg.to_string());
            }
        };

        let sidecar_command = match app_handle.shell().sidecar("tunnel-rs") {
            Ok(cmd) => cmd.args(["client", "--config-stdin"]),
            Err(e) => {
                let error_msg = format!("Failed to create sidecar command: {}", e);
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
        };

        let (mut rx, mut child) = match sidecar_command.spawn() {
            Ok(result) => result,
            Err(e) => {
                let error_msg = format!("Failed to spawn sidecar: {}", e);
                self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
                return Err(error_msg);
            }
        };

        // Pipe JSON config via stdin
        if let Err(e) = child.write(json_config.as_bytes()) {
            let _ = child.kill();
            let error_msg = format!("Failed to write config to sidecar stdin: {}", e);
            self.cleanup_failed_instance(instance, forwarding_id, &error_msg).await;
            return Err(error_msg);
        }

        {
            let mut guard = instance.lock().await;
            guard.child = Some(ChildProcess::Sidecar(child));
            guard.status = TunnelStatus::Running;
            guard.add_log("Started with bundled sidecar".to_string(), false);
        }

        // Spawn log reader for sidecar (uses event-based output)
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            use tauri_plugin_shell::process::CommandEvent;

            while let Some(event) = rx.recv().await {
                let mut guard = instance_clone.lock().await;
                match event {
                    CommandEvent::Stdout(line) => {
                        guard.add_log(String::from_utf8_lossy(&line).to_string(), false);
                    }
                    CommandEvent::Stderr(line) => {
                        guard.add_log(String::from_utf8_lossy(&line).to_string(), true);
                    }
                    CommandEvent::Terminated(payload) => {
                        // Preserve Stopped status if already set by stop()
                        if !matches!(guard.status, TunnelStatus::Stopped) {
                            let success = payload.code == Some(0);
                            guard.status = if success {
                                TunnelStatus::Stopped
                            } else {
                                TunnelStatus::Error
                            };
                        }
                        let exit_msg = match payload.code {
                            Some(code) => format!("Process exited with code: {}", code),
                            None => "Process terminated by signal".to_string(),
                        };
                        let is_error = payload.code != Some(0);
                        guard.add_log(exit_msg, is_error);
                        guard.child = None;
                        break;
                    }
                    CommandEvent::Error(err) => {
                        guard.status = TunnelStatus::Error;
                        guard.add_log(format!("Process error: {}", err), true);
                        guard.child = None;
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Stop a running tunnel
    pub async fn stop(&self, forwarding_id: Uuid) -> Result<(), String> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&forwarding_id) {
            let mut guard = instance.lock().await;

            match guard.child.take() {
                Some(ChildProcess::Tokio(mut child)) => {
                    // Kill and wait for the process to actually terminate
                    if let Err(e) = child.kill().await {
                        guard.add_log(format!("Failed to send kill signal: {}", e), true);
                    }
                    // Wait for process to exit (with timeout)
                    match tokio::time::timeout(
                        tokio::time::Duration::from_secs(2),
                        child.wait(),
                    )
                    .await
                    {
                        Ok(Ok(_)) => {
                            guard.add_log("Tunnel stopped by user".to_string(), false);
                        }
                        Ok(Err(e)) => {
                            guard.add_log(format!("Error waiting for process: {}", e), true);
                        }
                        Err(_) => {
                            guard.add_log("Timeout waiting for process to exit".to_string(), true);
                        }
                    }
                }
                Some(ChildProcess::Sidecar(child)) => {
                    if let Err(e) = child.kill() {
                        guard.add_log(format!("Failed to kill process: {}", e), true);
                    } else {
                        guard.add_log("Tunnel stopped by user".to_string(), false);
                    }
                }
                None => {
                    // No child to kill
                }
            }

            guard.status = TunnelStatus::Stopped;

            Ok(())
        } else {
            Err("Tunnel not found".to_string())
        }
    }

    /// Stop all running tunnels
    pub async fn stop_all(&self) {
        // Collect IDs first to avoid holding lock while stopping
        let ids: Vec<Uuid> = {
            let instances = self.instances.read().await;
            instances.keys().copied().collect()
        };

        // Stop all instances gracefully
        for id in ids {
            let _ = self.stop(id).await;
        }
    }

    /// Force kill all tracked child processes immediately (for emergency shutdown)
    pub async fn force_kill_all(&self) {
        let mut to_kill = Vec::new();
        {
            let instances = self.instances.read().await;
            for instance in instances.values() {
                let mut guard = instance.lock().await;

                if let Some(child) = guard.child.take() {
                    to_kill.push(child);
                }
            }
        }

        for child in to_kill {
            match child {
                ChildProcess::Tokio(mut child) => {
                    if let Err(e) = child.kill().await {
                        tracing::error!("Failed to kill process: {}", e);
                    }
                }
                ChildProcess::Sidecar(child) => {
                    if let Err(e) = child.kill() {
                        tracing::error!("Failed to kill process: {}", e);
                    }
                }
            }
        }

        // Clean up instance state after kill attempts to avoid Windows file locking issues.
        {
            let instances = self.instances.read().await;
            for instance in instances.values() {
                let mut guard = instance.lock().await;
                guard.status = TunnelStatus::Stopped;
            }
        }
    }
}
