//! Configuration storage for tunnel-rs client configs

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

/// Iroh transport tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransportConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub congestion_controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receive_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_window: Option<u64>,
}

/// Iroh-specific client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrohConfig {
    pub server_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relay_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks5_proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<TransportConfig>,
}

/// Full tunnel-rs client configuration (matching client.toml format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelClientConfig {
    pub role: String,
    pub mode: String,
    pub iroh: IrohConfig,
}

impl TunnelClientConfig {
    /// Create a new config with default values
    pub fn new(server_node_id: String) -> Self {
        Self {
            role: "client".to_string(),
            mode: "iroh".to_string(),
            iroh: IrohConfig {
                server_node_id,
                request_source: None,
                target: None,
                relay_urls: vec![],
                dns_server: None,
                socks5_proxy: None,
                auth_token: None,
                auth_token_file: None,
                transport: None,
            },
        }
    }

    /// Convert to TOML string for writing config file
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}

/// Stored config entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredConfig {
    pub id: Uuid,
    pub name: String,
    pub config: TunnelClientConfig,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

/// Config store managing all saved configurations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigStore {
    pub configs: HashMap<Uuid, StoredConfig>,
}

/// Get the application data directory path
fn app_data_dir() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not find data directory".to_string())?;
    Ok(data_dir.join("tunnel-rs-manager"))
}

/// App-wide settings (persisted)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub binary_path: Option<String>,
}

impl AppSettings {
    /// Get the settings file path
    fn settings_path() -> Result<PathBuf, String> {
        Ok(app_data_dir()?.join("settings.json"))
    }

    /// Load settings from disk
    pub fn load() -> Result<Self, String> {
        let path = Self::settings_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read settings: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse settings: {}", e))
    }

    /// Save settings to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::settings_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create settings directory: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write settings: {}", e))
    }
}

impl ConfigStore {
    /// Get the config store file path
    fn store_path() -> Result<PathBuf, String> {
        Ok(app_data_dir()?.join("configs.json"))
    }

    /// Load config store from disk
    pub fn load() -> Result<Self, String> {
        let path = Self::store_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config store: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config store: {}", e))
    }

    /// Save config store to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config store: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write config store: {}", e))
    }

    /// Get all configs as a list
    pub fn list(&self) -> Vec<StoredConfig> {
        let mut configs: Vec<_> = self.configs.values().cloned().collect();
        configs.sort_by(|a, b| a.name.cmp(&b.name));
        configs
    }

    /// Get a config by ID
    pub fn get(&self, id: Uuid) -> Option<&StoredConfig> {
        self.configs.get(&id)
    }

    /// Add or update a config
    pub fn upsert(&mut self, mut config: StoredConfig) -> Result<Uuid, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.configs.contains_key(&config.id) {
            config.updated_at = now;
        } else {
            config.created_at = now;
            config.updated_at = now;
        }

        let id = config.id;
        self.configs.insert(id, config);
        self.save()?;
        Ok(id)
    }

    /// Delete a config by ID
    pub fn delete(&mut self, id: Uuid) -> Result<(), String> {
        self.configs.remove(&id);
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_to_toml() {
        let mut config = TunnelClientConfig::new("test123".to_string());
        config.iroh.request_source = Some("tcp://127.0.0.1:22".to_string());
        config.iroh.target = Some("127.0.0.1:2222".to_string());
        config.iroh.auth_token = Some("iXXXXXXXXXXXXXXXXX".to_string());
        
        let toml = config.to_toml().unwrap();
        assert!(toml.contains("role = \"client\""));
        assert!(toml.contains("mode = \"iroh\""));
        assert!(toml.contains("server_node_id = \"test123\""));
    }
}
