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

/// Server Group: Named collection of shared connection settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerGroup {
    pub id: Uuid,
    pub name: String,
    pub server_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
    #[serde(default)]
    pub relay_urls: Vec<String>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

/// Forwarding: Individual named source/target pair within a server group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forwarding {
    pub id: Uuid,
    pub server_group_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
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

/// Config store managing server groups and forwardings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigStore {
    pub server_groups: HashMap<Uuid, ServerGroup>,
    pub forwardings: HashMap<Uuid, Forwarding>,
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

    // ============================================================================
    // Server Group CRUD
    // ============================================================================

    /// Get all server groups as a list
    pub fn list_server_groups(&self) -> Vec<ServerGroup> {
        let mut groups: Vec<_> = self.server_groups.values().cloned().collect();
        groups.sort_by(|a, b| a.name.cmp(&b.name));
        groups
    }

    /// Get a server group by ID
    pub fn get_server_group(&self, id: Uuid) -> Option<&ServerGroup> {
        self.server_groups.get(&id)
    }

    /// Add or update a server group
    pub fn upsert_server_group(&mut self, mut group: ServerGroup) -> Result<Uuid, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.server_groups.contains_key(&group.id) {
            group.updated_at = now;
        } else {
            group.created_at = now;
            group.updated_at = now;
        }

        let id = group.id;
        self.server_groups.insert(id, group);
        self.save()?;
        Ok(id)
    }

    /// Delete a server group by ID
    /// Returns error if the group has forwardings
    pub fn delete_server_group(&mut self, id: Uuid) -> Result<(), String> {
        // Check if any forwardings reference this group
        let has_forwardings = self.forwardings.values().any(|f| f.server_group_id == id);
        if has_forwardings {
            return Err("Cannot delete server group with existing forwardings. Delete all forwardings first.".to_string());
        }

        self.server_groups.remove(&id);
        self.save()
    }

    // ============================================================================
    // Forwarding CRUD
    // ============================================================================

    /// Get all forwardings as a list
    pub fn list_forwardings(&self) -> Vec<Forwarding> {
        let mut forwardings: Vec<_> = self.forwardings.values().cloned().collect();
        forwardings.sort_by(|a, b| a.name.cmp(&b.name));
        forwardings
    }

    /// Get all forwardings for a specific server group
    pub fn list_forwardings_by_group(&self, server_group_id: Uuid) -> Vec<Forwarding> {
        let mut forwardings: Vec<_> = self.forwardings.values()
            .filter(|f| f.server_group_id == server_group_id)
            .cloned()
            .collect();
        forwardings.sort_by(|a, b| a.name.cmp(&b.name));
        forwardings
    }

    /// Get a forwarding by ID
    pub fn get_forwarding(&self, id: Uuid) -> Option<&Forwarding> {
        self.forwardings.get(&id)
    }

    /// Add or update a forwarding
    pub fn upsert_forwarding(&mut self, mut forwarding: Forwarding) -> Result<Uuid, String> {
        // Verify server group exists
        if !self.server_groups.contains_key(&forwarding.server_group_id) {
            return Err("Server group not found".to_string());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if self.forwardings.contains_key(&forwarding.id) {
            forwarding.updated_at = now;
        } else {
            forwarding.created_at = now;
            forwarding.updated_at = now;
        }

        let id = forwarding.id;
        self.forwardings.insert(id, forwarding);
        self.save()?;
        Ok(id)
    }

    /// Delete a forwarding by ID
    pub fn delete_forwarding(&mut self, id: Uuid) -> Result<(), String> {
        self.forwardings.remove(&id);
        self.save()
    }

    // ============================================================================
    // Config Building
    // ============================================================================

    /// Build a TunnelClientConfig from a forwarding ID
    /// Combines the server group settings with the forwarding's source/target
    pub fn build_tunnel_config(&self, forwarding_id: Uuid) -> Result<TunnelClientConfig, String> {
        let forwarding = self.get_forwarding(forwarding_id)
            .ok_or_else(|| "Forwarding not found".to_string())?;

        let group = self.get_server_group(forwarding.server_group_id)
            .ok_or_else(|| "Server group not found".to_string())?;

        let mut config = TunnelClientConfig::new(group.server_node_id.clone());
        config.iroh.request_source = forwarding.source.clone();
        config.iroh.target = forwarding.target.clone();
        config.iroh.auth_token = group.auth_token.clone();
        config.iroh.relay_urls = group.relay_urls.clone();

        Ok(config)
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

    #[test]
    fn test_build_tunnel_config() {
        let mut store = ConfigStore::default();

        // Create a server group
        let group_id = Uuid::new_v4();
        let group = ServerGroup {
            id: group_id,
            name: "Test Group".to_string(),
            server_node_id: "test_node_123".to_string(),
            auth_token: Some("iXXXXXXXXXXXXXXXXX".to_string()),
            relay_urls: vec!["https://relay.example.com".to_string()],
            created_at: 0,
            updated_at: 0,
        };
        store.server_groups.insert(group_id, group);

        // Create a forwarding
        let fwd_id = Uuid::new_v4();
        let forwarding = Forwarding {
            id: fwd_id,
            server_group_id: group_id,
            name: "SSH".to_string(),
            source: Some("tcp://127.0.0.1:22".to_string()),
            target: Some("127.0.0.1:2222".to_string()),
            created_at: 0,
            updated_at: 0,
        };
        store.forwardings.insert(fwd_id, forwarding);

        // Build config
        let config = store.build_tunnel_config(fwd_id).unwrap();
        assert_eq!(config.iroh.server_node_id, "test_node_123");
        assert_eq!(config.iroh.request_source, Some("tcp://127.0.0.1:22".to_string()));
        assert_eq!(config.iroh.target, Some("127.0.0.1:2222".to_string()));
        assert_eq!(config.iroh.auth_token, Some("iXXXXXXXXXXXXXXXXX".to_string()));
        assert_eq!(config.iroh.relay_urls.len(), 1);
    }
}
