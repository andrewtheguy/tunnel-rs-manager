//! Tauri backend for tunnel-rs-manager

mod config;
mod process;

use config::{ConfigStore, StoredConfig, TunnelClientConfig};
use process::{ProcessManager, TunnelInstanceView};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Guard to prevent multiple shutdown handlers from running
static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Application state shared across commands
pub struct AppState {
    config_store: Mutex<ConfigStore>,
    process_manager: Arc<ProcessManager>,
    config_load_error: Option<String>,
}

impl AppState {
    fn new() -> Self {
        let (config_store, config_load_error) = match ConfigStore::load() {
            Ok(store) => (store, None),
            Err(e) => {
                let error_msg = format!("Failed to load config store: {}. Using default.", e);
                tracing::error!("{}", error_msg);
                (ConfigStore::default(), Some(error_msg))
            }
        };
        Self {
            config_store: Mutex::new(config_store),
            process_manager: Arc::new(ProcessManager::new()),
            config_load_error,
        }
    }
}

// ============================================================================
// Config Commands
// ============================================================================

#[tauri::command]
async fn list_configs(state: State<'_, AppState>) -> Result<Vec<StoredConfig>, String> {
    let store = state.config_store.lock().await;
    Ok(store.list())
}

#[tauri::command]
async fn get_config(state: State<'_, AppState>, id: String) -> Result<StoredConfig, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let store = state.config_store.lock().await;
    store.get(uuid).cloned().ok_or_else(|| "Config not found".to_string())
}

#[tauri::command]
async fn create_config(
    state: State<'_, AppState>,
    name: String,
    server_node_id: String,
    source: Option<String>,
    target: Option<String>,
    auth_token: Option<String>,
    relay_urls: Option<Vec<String>>,
) -> Result<StoredConfig, String> {
    let mut config = TunnelClientConfig::new(server_node_id);
    config.iroh.request_source = source;
    config.iroh.target = target;
    config.iroh.auth_token = auth_token;
    if let Some(urls) = relay_urls {
        config.iroh.relay_urls = urls;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let stored = StoredConfig {
        id: Uuid::new_v4(),
        name,
        config,
        created_at: now,
        updated_at: now,
    };

    let mut store = state.config_store.lock().await;
    store.upsert(stored.clone())?;
    Ok(stored)
}

#[tauri::command]
async fn update_config(
    state: State<'_, AppState>,
    id: String,
    name: String,
    server_node_id: String,
    source: Option<String>,
    target: Option<String>,
    auth_token: Option<String>,
    relay_urls: Option<Vec<String>>,
) -> Result<StoredConfig, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;

    let mut store = state.config_store.lock().await;
    let existing = store.get(uuid).ok_or_else(|| "Config not found".to_string())?;
    let created_at = existing.created_at;

    let mut config = TunnelClientConfig::new(server_node_id);
    config.iroh.request_source = source;
    config.iroh.target = target;
    config.iroh.auth_token = auth_token;
    if let Some(urls) = relay_urls {
        config.iroh.relay_urls = urls;
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let stored = StoredConfig {
        id: uuid,
        name,
        config,
        created_at,
        updated_at: now,
    };

    store.upsert(stored.clone())?;
    Ok(stored)
}

#[tauri::command]
async fn delete_config(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut store = state.config_store.lock().await;
    store.delete(uuid)
}

#[tauri::command]
fn get_config_load_error(state: State<'_, AppState>) -> Option<String> {
    state.config_load_error.clone()
}

// ============================================================================
// Process Commands
// ============================================================================

#[tauri::command]
async fn list_instances(state: State<'_, AppState>) -> Result<Vec<TunnelInstanceView>, String> {
    Ok(state.process_manager.list_instances().await)
}

#[tauri::command]
async fn get_instance(state: State<'_, AppState>, id: String) -> Result<Option<TunnelInstanceView>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    Ok(state.process_manager.get_instance(uuid).await)
}

#[tauri::command]
async fn start_tunnel(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;

    // Clone config and drop lock before awaiting to avoid holding lock across await
    let config = {
        let store = state.config_store.lock().await;
        store.get(uuid).cloned().ok_or_else(|| "Config not found".to_string())?
    };

    state.process_manager.start(&config).await
}

#[tauri::command]
async fn stop_tunnel(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    state.process_manager.stop(uuid).await
}

#[tauri::command]
async fn set_binary_path(state: State<'_, AppState>, path: Option<String>) -> Result<(), String> {
    state.process_manager.set_binary_path(path).await;
    Ok(())
}

// ============================================================================
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Config commands
            list_configs,
            get_config,
            create_config,
            update_config,
            delete_config,
            get_config_load_error,
            // Process commands
            list_instances,
            get_instance,
            start_tunnel,
            stop_tunnel,
            set_binary_path,
        ])
        .setup(|app| {
            // Emit config load error event if there was an error during startup
            if let Some(state) = app.try_state::<AppState>() {
                if let Some(ref error) = state.config_load_error {
                    let _ = app.emit("config-load-failure", error.clone());
                }
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            // Check if shutdown is already in progress (compare_exchange returns Ok if we set it)
            if SHUTDOWN_IN_PROGRESS
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                // Shutdown already in progress, ignore this event
                api.prevent_exit();
                return;
            }

            // Prevent immediate exit to allow graceful shutdown
            api.prevent_exit();

            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // Stop all running tunnels with a timeout
                if let Some(state) = app_handle.try_state::<AppState>() {
                    const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

                    match tokio::time::timeout(SHUTDOWN_TIMEOUT, state.process_manager.stop_all()).await {
                        Ok(()) => {
                            tracing::info!("All tunnels stopped gracefully");
                        }
                        Err(_) => {
                            tracing::error!(
                                "Shutdown timeout after {} seconds, forcing exit",
                                SHUTDOWN_TIMEOUT.as_secs()
                            );
                        }
                    }
                }
                // Now exit
                app_handle.exit(0);
            });
        }
    });
}
