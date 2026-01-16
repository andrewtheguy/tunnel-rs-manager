//! Process management for tunnel-rs client instances

use crate::config::StoredConfig;
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
    pub config_id: Uuid,
    pub config_name: String,
    pub status: TunnelStatus,
    pub child: Option<ChildProcess>,
    pub logs: VecDeque<LogEntry>,
    pub temp_config_path: Option<std::path::PathBuf>,
}

impl TunnelInstance {
    fn new(config_id: Uuid, config_name: String) -> Self {
        Self {
            config_id,
            config_name,
            status: TunnelStatus::Stopped,
            child: None,
            logs: VecDeque::new(),
            temp_config_path: None,
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
    pub config_id: Uuid,
    pub config_name: String,
    pub status: TunnelStatus,
    pub logs: VecDeque<LogEntry>,
}

impl From<&TunnelInstance> for TunnelInstanceView {
    fn from(instance: &TunnelInstance) -> Self {
        Self {
            config_id: instance.config_id,
            config_name: instance.config_name.clone(),
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

    /// Get custom binary path
    pub async fn get_custom_binary_path(&self) -> Option<String> {
        self.custom_binary_path.read().await.clone()
    }

    /// Check if using bundled binary
    pub async fn is_using_bundled(&self) -> bool {
        self.custom_binary_path.read().await.is_none()
    }

    /// Get all running instances
    pub async fn list_instances(&self) -> Vec<TunnelInstanceView> {
        let instances = self.instances.read().await;
        let mut views = Vec::new();
        for instance in instances.values() {
            let guard = instance.lock().await;
            views.push(TunnelInstanceView::from(&*guard));
        }
        views.sort_by(|a, b| a.config_name.cmp(&b.config_name));
        views
    }

    /// Get a specific instance
    pub async fn get_instance(&self, id: Uuid) -> Option<TunnelInstanceView> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&id) {
            let guard = instance.lock().await;
            Some(TunnelInstanceView::from(&*guard))
        } else {
            None
        }
    }

    /// Start a tunnel with the given config
    pub async fn start(&self, config: &StoredConfig) -> Result<(), String> {
        let id = config.id;

        // Create instance and insert atomically under write lock to prevent TOCTOU
        let instance = {
            let mut instances = self.instances.write().await;

            // Check if already running while holding write lock
            if let Some(existing) = instances.get(&id) {
                let guard = existing.lock().await;
                match guard.status {
                    TunnelStatus::Running | TunnelStatus::Starting => {
                        return Err("Tunnel is already running".to_string());
                    }
                    _ => {}
                }
            }

            // Create and insert instance while still holding write lock
            let instance = Arc::new(Mutex::new(TunnelInstance::new(id, config.name.clone())));
            {
                let mut guard = instance.lock().await;
                guard.status = TunnelStatus::Starting;
                guard.add_log("Starting tunnel...".to_string(), false);
            }
            instances.insert(id, instance.clone());

            instance
            // Write lock released here
        };

        // Write temp config file
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("tunnel-rs-{}.toml", id));

        let toml_content = match config.config.to_toml() {
            Ok(content) => content,
            Err(e) => {
                // Clean up: set error status, remove from instances
                {
                    let mut guard = instance.lock().await;
                    guard.status = TunnelStatus::Error;
                    guard.add_log(format!("Failed to serialize config: {}", e), true);
                }
                let mut instances = self.instances.write().await;
                instances.remove(&id);
                return Err(format!("Failed to serialize config: {}", e));
            }
        };

        if let Err(e) = std::fs::write(&config_path, &toml_content) {
            // Clean up: set error status, remove from instances
            {
                let mut guard = instance.lock().await;
                guard.status = TunnelStatus::Error;
                guard.add_log(format!("Failed to write temp config: {}", e), true);
            }
            let mut instances = self.instances.write().await;
            instances.remove(&id);
            return Err(format!("Failed to write temp config: {}", e));
        }

        {
            let mut guard = instance.lock().await;
            guard.temp_config_path = Some(config_path.clone());
        }

        // Check if we should use custom binary or sidecar
        let custom_path = self.custom_binary_path.read().await.clone();

        if let Some(binary_path) = custom_path {
            // Use custom binary path with tokio::process::Command
            self.start_with_custom_binary(&instance, id, &binary_path, &config_path)
                .await
        } else {
            // Use bundled sidecar
            self.start_with_sidecar(&instance, id, &config_path).await
        }
    }

    /// Start tunnel using custom binary path
    async fn start_with_custom_binary(
        &self,
        instance: &Arc<Mutex<TunnelInstance>>,
        id: Uuid,
        binary_path: &str,
        config_path: &std::path::Path,
    ) -> Result<(), String> {
        // Verify binary exists
        if !std::path::Path::new(binary_path).exists() {
            let error_msg = format!("Custom binary path '{}' does not exist", binary_path);
            {
                let mut guard = instance.lock().await;
                guard.status = TunnelStatus::Error;
                guard.add_log(error_msg.clone(), true);
                if let Err(del_err) = std::fs::remove_file(config_path) {
                    guard.add_log(format!("Failed to delete temp config: {}", del_err), true);
                }
                guard.temp_config_path = None;
            }
            let mut instances = self.instances.write().await;
            instances.remove(&id);
            return Err(error_msg);
        }

        let child = tokio::process::Command::new(binary_path)
            .arg("client")
            .arg("-c")
            .arg(config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                {
                    let mut guard = instance.lock().await;
                    guard.status = TunnelStatus::Error;
                    guard.add_log(format!("Failed to spawn tunnel-rs: {}", e), true);
                    if let Err(del_err) = std::fs::remove_file(config_path) {
                        guard.add_log(format!("Failed to delete temp config: {}", del_err), true);
                    }
                    guard.temp_config_path = None;
                }
                let mut instances = self.instances.write().await;
                instances.remove(&id);
                return Err(format!("Failed to spawn tunnel-rs: {}", e));
            }
        };

        // Take stdout/stderr for log capture
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        {
            let mut guard = instance.lock().await;
            guard.child = Some(ChildProcess::Tokio(child));
            guard.status = TunnelStatus::Running;
            guard.add_log(
                format!(
                    "Started with custom binary: {} (config: {})",
                    binary_path,
                    config_path.display()
                ),
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
                            guard.status = if status.success() {
                                TunnelStatus::Stopped
                            } else {
                                TunnelStatus::Error
                            };
                            guard.add_log(
                                format!("Process exited with status: {}", status),
                                !status.success(),
                            );
                            guard.child = None;

                            // Clean up temp config
                            if let Some(ref path) = guard.temp_config_path {
                                let _ = std::fs::remove_file(path);
                            }
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
        _id: Uuid,
        config_path: &std::path::Path,
    ) -> Result<(), String> {
        let app_handle = {
            let handle = self.app_handle.read().await;
            handle.clone().ok_or_else(|| "App handle not set".to_string())?
        };

        let sidecar_command = app_handle
            .shell()
            .sidecar("tunnel-rs")
            .map_err(|e| format!("Failed to create sidecar command: {}", e))?
            .args(["client", "-c", &config_path.to_string_lossy()]);

        let (mut rx, child) = sidecar_command
            .spawn()
            .map_err(|e| {
                // Clean up temp config on spawn failure
                let _ = std::fs::remove_file(config_path);
                format!("Failed to spawn sidecar: {}", e)
            })?;

        {
            let mut guard = instance.lock().await;
            guard.child = Some(ChildProcess::Sidecar(child));
            guard.status = TunnelStatus::Running;
            guard.add_log(
                format!(
                    "Started with bundled sidecar (config: {})",
                    config_path.display()
                ),
                false,
            );
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
                        let success = payload.code == Some(0);
                        guard.status = if success {
                            TunnelStatus::Stopped
                        } else {
                            TunnelStatus::Error
                        };
                        let exit_msg = match payload.code {
                            Some(code) => format!("Process exited with code: {}", code),
                            None => "Process terminated by signal".to_string(),
                        };
                        guard.add_log(exit_msg, !success);
                        guard.child = None;

                        // Clean up temp config
                        if let Some(ref path) = guard.temp_config_path {
                            let _ = std::fs::remove_file(path);
                        }
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
    pub async fn stop(&self, id: Uuid) -> Result<(), String> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&id) {
            let mut guard = instance.lock().await;

            match guard.child.take() {
                Some(ChildProcess::Tokio(mut child)) => {
                    child
                        .kill()
                        .await
                        .map_err(|e| format!("Failed to kill process: {}", e))?;
                    guard.add_log("Tunnel stopped by user".to_string(), false);
                }
                Some(ChildProcess::Sidecar(child)) => {
                    child
                        .kill()
                        .map_err(|e| format!("Failed to kill process: {}", e))?;
                    guard.add_log("Tunnel stopped by user".to_string(), false);
                }
                None => {
                    // No child to kill
                }
            }

            guard.status = TunnelStatus::Stopped;

            // Clean up temp config
            if let Some(ref path) = guard.temp_config_path {
                let _ = std::fs::remove_file(path);
                guard.temp_config_path = None;
            }

            Ok(())
        } else {
            Err("Tunnel not found".to_string())
        }
    }

    /// Stop all running tunnels and wait for them to finish
    pub async fn stop_all(&self) {
        // Collect IDs first to avoid holding lock while stopping
        let ids: Vec<Uuid> = {
            let instances = self.instances.read().await;
            instances.keys().copied().collect()
        };

        for id in ids {
            let _ = self.stop(id).await;
        }
    }
}
