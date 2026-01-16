//! Process management for tunnel-rs client instances

use crate::config::StoredConfig;
use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
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

/// A running tunnel instance
pub struct TunnelInstance {
    pub config_id: Uuid,
    pub config_name: String,
    pub status: TunnelStatus,
    pub child: Option<Child>,
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
    binary_path: RwLock<Option<String>>,
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
            binary_path: RwLock::new(None),
        }
    }

    /// Set custom binary path (default: "tunnel-rs" in PATH)
    pub async fn set_binary_path(&self, path: Option<String>) {
        *self.binary_path.write().await = path;
    }

    /// Get the binary path to use
    async fn get_binary(&self) -> String {
        self.binary_path
            .read()
            .await
            .clone()
            .unwrap_or_else(|| "tunnel-rs".to_string())
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

        // Spawn the process
        let binary = self.get_binary().await;
        let child = Command::new(&binary)
            .arg("client")
            .arg("-c")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn();

        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                // Clean up: delete temp config file, set error status, remove from instances
                {
                    let mut guard = instance.lock().await;
                    guard.status = TunnelStatus::Error;
                    guard.add_log(format!("Failed to spawn tunnel-rs: {}", e), true);

                    // Clean up temp config file
                    if let Err(del_err) = std::fs::remove_file(&config_path) {
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
            guard.child = Some(child);
            guard.status = TunnelStatus::Running;
            guard.add_log(format!("Started with config: {}", config_path.display()), false);
        }

        // Spawn log readers
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

        // Spawn process monitor
        let instance_clone = instance.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let mut guard = instance_clone.lock().await;
                if let Some(ref mut child) = guard.child {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            guard.status = if status.success() {
                                TunnelStatus::Stopped
                            } else {
                                TunnelStatus::Error
                            };
                            guard.add_log(format!("Process exited with status: {}", status), !status.success());
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

    /// Stop a running tunnel
    pub async fn stop(&self, id: Uuid) -> Result<(), String> {
        let instances = self.instances.read().await;
        if let Some(instance) = instances.get(&id) {
            let mut guard = instance.lock().await;
            
            if let Some(ref mut child) = guard.child {
                child.kill().await
                    .map_err(|e| format!("Failed to kill process: {}", e))?;
                guard.add_log("Tunnel stopped by user".to_string(), false);
            }
            
            guard.status = TunnelStatus::Stopped;
            guard.child = None;
            
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
