//! Tauri backend for tunnel-rs-manager

mod config;
mod crypto;
mod process;

use config::{AppSettings, ConfigStore, ExportData, Forwarding, ImportResult, SecretsStore, ServerGroup};
use crypto::EncryptedToken;
use process::{ProcessManager, TunnelInstanceView};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{
    Emitter, Manager, State,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Guard to prevent multiple shutdown handlers from running
static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Application state shared across commands
pub struct AppState {
    config_store: Mutex<ConfigStore>,
    secrets_store: SecretsStore,
    app_settings: Mutex<AppSettings>,
    process_manager: Arc<ProcessManager>,
    config_load_error: Option<String>,
}

impl AppState {
    fn new() -> Self {
        let (mut config_store, config_load_error) = match ConfigStore::load() {
            Ok(store) => (store, None),
            Err(e) => {
                let error_msg = format!("Failed to load config store: {}. Using default.", e);
                tracing::error!("{}", error_msg);
                (ConfigStore::default(), Some(error_msg))
            }
        };

        let secrets_store = SecretsStore::new();

        // Restore auth tokens from secrets store (OS keyring)
        config_store.restore_secrets(&secrets_store);

        let app_settings = match AppSettings::load() {
            Ok(settings) => settings,
            Err(e) => {
                tracing::error!("Failed to load app settings: {}. Using default.", e);
                AppSettings::default()
            }
        };

        Self {
            config_store: Mutex::new(config_store),
            secrets_store,
            app_settings: Mutex::new(app_settings),
            process_manager: Arc::new(ProcessManager::new()),
            config_load_error,
        }
    }
}

// ============================================================================
// Server Group Commands
// ============================================================================

#[tauri::command]
async fn list_server_groups(state: State<'_, AppState>) -> Result<Vec<ServerGroup>, String> {
    let mut store = state.config_store.lock().await;
    store.restore_secrets(&state.secrets_store);
    Ok(store.list_server_groups())
}

#[tauri::command]
async fn get_server_group(state: State<'_, AppState>, id: String) -> Result<ServerGroup, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut store = state.config_store.lock().await;
    store.restore_secrets(&state.secrets_store);
    store
        .get_server_group(uuid)
        .cloned()
        .ok_or_else(|| "Server group not found".to_string())
}

#[tauri::command]
async fn create_server_group(
    state: State<'_, AppState>,
    name: String,
    server_node_id: String,
    auth_token: String,
    alpn_token: String,
    relay_urls: Option<Vec<String>>,
) -> Result<ServerGroup, String> {
    if auth_token.is_empty() {
        return Err("Auth token is required".to_string());
    }
    if alpn_token.is_empty() {
        return Err("ALPN token is required".to_string());
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let group = ServerGroup {
        id: Uuid::new_v4(),
        name,
        server_node_id: server_node_id.clone(),
        auth_token: Some(auth_token.clone()),
        alpn_token: Some(alpn_token.clone()),
        relay_urls: relay_urls.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    // Persist server group first to avoid orphan secrets if this fails
    {
        let mut store = state.config_store.lock().await;
        store.upsert_server_group(group.clone())?;
    }

    // Save tokens to secrets store (if this fails, user can re-edit to add tokens)
    state.secrets_store.set_token(&server_node_id, &auth_token)?;
    state.secrets_store.set_alpn_token(&server_node_id, &alpn_token)?;

    Ok(group)
}

#[tauri::command]
async fn update_server_group(
    state: State<'_, AppState>,
    id: String,
    name: String,
    server_node_id: String,
    auth_token: String,
    alpn_token: String,
    relay_urls: Option<Vec<String>>,
) -> Result<ServerGroup, String> {
    if auth_token.is_empty() {
        return Err("Auth token is required".to_string());
    }
    if alpn_token.is_empty() {
        return Err("ALPN token is required".to_string());
    }

    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;

    let (created_at, relay_urls_final, old_server_node_id) = {
        let store = state.config_store.lock().await;
        let existing = store
            .get_server_group(uuid)
            .ok_or_else(|| "Server group not found".to_string())?;
        (existing.created_at, relay_urls.unwrap_or_default(), existing.server_node_id.clone())
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let group = ServerGroup {
        id: uuid,
        name,
        server_node_id: server_node_id.clone(),
        auth_token: Some(auth_token.clone()),
        alpn_token: Some(alpn_token.clone()),
        relay_urls: relay_urls_final,
        created_at,
        updated_at: now,
    };

    // Persist server group first to avoid orphan secrets if this fails
    {
        let mut store = state.config_store.lock().await;
        store.upsert_server_group(group.clone())?;
    }

    // Update secrets store: set the new tokens first, then clean up old if node id changed
    state.secrets_store.set_token(&server_node_id, &auth_token)?;
    state.secrets_store.set_alpn_token(&server_node_id, &alpn_token)?;
    if old_server_node_id != server_node_id {
        let _ = state.secrets_store.remove_token(&old_server_node_id);
        let _ = state.secrets_store.remove_alpn_token(&old_server_node_id);
    }

    Ok(group)
}

#[tauri::command]
async fn delete_server_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;

    // Get the server_node_id before deleting so we can clean up secrets
    let server_node_id = {
        let store = state.config_store.lock().await;
        store
            .get_server_group(uuid)
            .map(|g| g.server_node_id.clone())
    };

    // Delete the server group
    {
        let mut store = state.config_store.lock().await;
        store.delete_server_group(uuid)?;
    }

    // Clean up auth token and ALPN token from secrets store
    if let Some(node_id) = server_node_id {
        let _ = state.secrets_store.remove_token(&node_id);
        let _ = state.secrets_store.remove_alpn_token(&node_id);
    }

    Ok(())
}

// ============================================================================
// Forwarding Commands
// ============================================================================

#[tauri::command]
async fn list_forwardings(state: State<'_, AppState>) -> Result<Vec<Forwarding>, String> {
    let store = state.config_store.lock().await;
    Ok(store.list_forwardings())
}

#[tauri::command]
async fn list_forwardings_by_group(
    state: State<'_, AppState>,
    server_group_id: String,
) -> Result<Vec<Forwarding>, String> {
    let uuid = Uuid::parse_str(&server_group_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let store = state.config_store.lock().await;
    Ok(store.list_forwardings_by_group(uuid))
}

#[tauri::command]
async fn get_forwarding(state: State<'_, AppState>, id: String) -> Result<Forwarding, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let store = state.config_store.lock().await;
    store
        .get_forwarding(uuid)
        .cloned()
        .ok_or_else(|| "Forwarding not found".to_string())
}

#[tauri::command]
async fn create_forwarding(
    state: State<'_, AppState>,
    server_group_id: String,
    name: String,
    source: Option<String>,
    target: Option<String>,
) -> Result<Forwarding, String> {
    let group_uuid =
        Uuid::parse_str(&server_group_id).map_err(|e| format!("Invalid UUID: {}", e))?;

    let mut store = state.config_store.lock().await;

    // Verify server group exists before creating forwarding
    if store.get_server_group(group_uuid).is_none() {
        return Err(format!("Server group '{}' not found", server_group_id));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let forwarding = Forwarding {
        id: Uuid::new_v4(),
        server_group_id: group_uuid,
        name,
        source,
        target,
        created_at: now,
        updated_at: now,
    };

    store.upsert_forwarding(forwarding.clone())?;
    Ok(forwarding)
}

#[tauri::command]
async fn update_forwarding(
    state: State<'_, AppState>,
    id: String,
    server_group_id: String,
    name: String,
    source: Option<String>,
    target: Option<String>,
) -> Result<Forwarding, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let group_uuid =
        Uuid::parse_str(&server_group_id).map_err(|e| format!("Invalid UUID: {}", e))?;

    let mut store = state.config_store.lock().await;

    // Verify server group exists before updating forwarding
    if store.get_server_group(group_uuid).is_none() {
        return Err(format!("Server group '{}' not found", server_group_id));
    }

    let existing = store
        .get_forwarding(uuid)
        .ok_or_else(|| "Forwarding not found".to_string())?;
    let created_at = existing.created_at;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let forwarding = Forwarding {
        id: uuid,
        server_group_id: group_uuid,
        name,
        source,
        target,
        created_at,
        updated_at: now,
    };

    store.upsert_forwarding(forwarding.clone())?;
    Ok(forwarding)
}

#[tauri::command]
async fn delete_forwarding(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut store = state.config_store.lock().await;
    store.delete_forwarding(uuid)
}

#[tauri::command]
fn get_config_load_error(state: State<'_, AppState>) -> Option<String> {
    state.config_load_error.clone()
}

// ============================================================================
// Export/Import Commands
// ============================================================================

#[tauri::command]
async fn export_configs(
    state: State<'_, AppState>,
    passphrase: Option<String>,
) -> Result<String, String> {
    if let Some(ref pass) = passphrase {
        crypto::validate_passphrase(pass)?;
    }
    let store = state.config_store.lock().await;
    let export_data = store.export();

    // Serialize to JSON Value so we can inject encrypted tokens per server group
    let mut value = serde_json::to_value(&export_data)
        .map_err(|e| format!("Failed to serialize export data: {}", e))?;

    if let Some(ref pass) = passphrase {
        if let Some(groups) = value
            .get_mut("config")
            .and_then(|c| c.get_mut("server_groups"))
            .and_then(|g| g.as_object_mut())
        {
            for (_, group) in groups.iter_mut() {
                let node_id = group
                    .get("server_node_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if let Some(auth) = state.secrets_store.get_token(&node_id) {
                    let encrypted = crypto::encrypt_token(&auth, pass)?;
                    group["auth_token"] = serde_json::to_value(&encrypted)
                        .map_err(|e| format!("Failed to serialize encrypted token: {}", e))?;
                }
                if let Some(alpn) = state.secrets_store.get_alpn_token(&node_id) {
                    let encrypted = crypto::encrypt_token(&alpn, pass)?;
                    group["alpn_token"] = serde_json::to_value(&encrypted)
                        .map_err(|e| format!("Failed to serialize encrypted token: {}", e))?;
                }
            }
        }
    }

    serde_json::to_string_pretty(&value)
        .map_err(|e| format!("Failed to serialize export data: {}", e))
}

#[tauri::command]
async fn import_configs(
    state: State<'_, AppState>,
    json: String,
    passphrase: Option<String>,
) -> Result<ImportResult, String> {
    // Parse as Value so we can extract encrypted tokens before deserializing
    let mut value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid import data: {}", e))?;

    // Extract and decrypt tokens from server groups
    let mut decrypted_tokens: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();

    if let Some(groups) = value
        .get_mut("config")
        .and_then(|c| c.get_mut("server_groups"))
        .and_then(|g| g.as_object_mut())
    {
        for (_, group) in groups.iter_mut() {
            let node_id = group
                .get("server_node_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut auth = None;
            let mut alpn = None;

            // Check if auth_token is an encrypted object (has "alg" field)
            if group.get("auth_token").map_or(false, |v| v.is_object()) {
                let envelope: EncryptedToken = serde_json::from_value(group["auth_token"].clone())
                    .map_err(|e| format!("Invalid encrypted auth_token: {}", e))?;
                let pass = passphrase.as_deref()
                    .ok_or_else(|| "Passphrase required to import encrypted credentials".to_string())?;
                auth = Some(crypto::decrypt_token(&envelope, pass)?);
                group["auth_token"] = serde_json::Value::Null;
            }

            if group.get("alpn_token").map_or(false, |v| v.is_object()) {
                let envelope: EncryptedToken = serde_json::from_value(group["alpn_token"].clone())
                    .map_err(|e| format!("Invalid encrypted alpn_token: {}", e))?;
                let pass = passphrase.as_deref()
                    .ok_or_else(|| "Passphrase required to import encrypted credentials".to_string())?;
                alpn = Some(crypto::decrypt_token(&envelope, pass)?);
                group["alpn_token"] = serde_json::Value::Null;
            }

            if auth.is_some() || alpn.is_some() {
                decrypted_tokens.insert(node_id, (auth, alpn));
            }
        }
    }

    // Now deserialize the cleaned JSON into ExportData
    let export_data: ExportData = serde_json::from_value(value)
        .map_err(|e| format!("Invalid import data: {}", e))?;

    let mut store = state.config_store.lock().await;
    let result = store.import(export_data);

    // Only persist tokens if the import succeeded (avoids orphan secrets on
    // version mismatch or save failure)
    if result.success {
        for (node_id, (auth, alpn)) in decrypted_tokens {
            if let Some(a) = auth {
                state.secrets_store.set_token(&node_id, &a)?;
            }
            if let Some(a) = alpn {
                state.secrets_store.set_alpn_token(&node_id, &a)?;
            }
        }

        // Rehydrate in-memory auth tokens from secrets store after import
        store.restore_secrets(&state.secrets_store);
    }

    Ok(result)
}

#[tauri::command]
fn check_import_has_credentials(json: String) -> Result<bool, String> {
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid JSON: {}", e))?;
    if let Some(groups) = value
        .get("config")
        .and_then(|c| c.get("server_groups"))
        .and_then(|g| g.as_object())
    {
        for (_, group) in groups {
            if group.get("auth_token").map_or(false, |v| v.is_object())
                || group.get("alpn_token").map_or(false, |v| v.is_object())
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ============================================================================
// Process Commands
// ============================================================================

#[tauri::command]
async fn list_instances(state: State<'_, AppState>) -> Result<Vec<TunnelInstanceView>, String> {
    Ok(state.process_manager.list_instances().await)
}

#[tauri::command]
async fn get_instance(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<TunnelInstanceView>, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    Ok(state.process_manager.get_instance(uuid).await)
}

#[tauri::command]
async fn start_tunnel(state: State<'_, AppState>, forwarding_id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&forwarding_id).map_err(|e| format!("Invalid UUID: {}", e))?;

    // Get forwarding, server group, and build config
    let (forwarding_name, server_group_name, config) = {
        let store = state.config_store.lock().await;

        let forwarding = store
            .get_forwarding(uuid)
            .ok_or_else(|| "Forwarding not found".to_string())?;
        let forwarding_name = forwarding.name.clone();

        let group = store
            .get_server_group(forwarding.server_group_id)
            .ok_or_else(|| "Server group not found".to_string())?;
        let server_group_name = group.name.clone();

        // Require auth token and ALPN token to start tunnel
        if group.auth_token.is_none() {
            return Err(format!(
                "Auth token is required. Please edit server group '{}' and add an auth token.",
                group.name
            ));
        }
        if group.alpn_token.is_none() {
            return Err(format!(
                "ALPN token is required. Please edit server group '{}' and add an ALPN token.",
                group.name
            ));
        }

        let config = store.build_tunnel_config(uuid)?;

        (forwarding_name, server_group_name, config)
    };

    state
        .process_manager
        .start(uuid, &forwarding_name, &server_group_name, &config)
        .await
}

#[tauri::command]
async fn stop_tunnel(state: State<'_, AppState>, forwarding_id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&forwarding_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    state.process_manager.stop(uuid).await
}

#[tauri::command]
async fn get_custom_binary_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let settings = state.app_settings.lock().await;
    Ok(settings.binary_path.clone())
}

#[tauri::command]
async fn set_custom_binary_path(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<(), String> {
    // Persist to settings first to ensure consistency
    {
        let mut settings = state.app_settings.lock().await;
        settings.binary_path = path.clone();
        settings.save()?;
    }
    // Only update process manager if persistence succeeded
    state.process_manager.set_custom_binary_path(path).await;
    Ok(())
}

#[tauri::command]
async fn is_using_bundled_binary(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.process_manager.is_using_bundled().await)
}

#[tauri::command]
async fn get_binary_version(state: State<'_, AppState>) -> Result<String, String> {
    state.process_manager.get_binary_version().await
}

// ============================================================================
// Tray Icon Setup
// ============================================================================

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;

    // Use high-contrast tray icon for macOS menu bar (44x44 for retina)
    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.show() {
                        tracing::trace!("failed to show main window: {}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        tracing::trace!("failed to set focus on main window: {}", e);
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if let Err(e) = window.show() {
                        tracing::trace!("failed to show main window: {}", e);
                    }
                    if let Err(e) = window.set_focus() {
                        tracing::trace!("failed to set focus on main window: {}", e);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================================
// App Entry
// ============================================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Server Group commands
            list_server_groups,
            get_server_group,
            create_server_group,
            update_server_group,
            delete_server_group,
            // Forwarding commands
            list_forwardings,
            list_forwardings_by_group,
            get_forwarding,
            create_forwarding,
            update_forwarding,
            delete_forwarding,
            get_config_load_error,
            // Export/Import commands
            export_configs,
            import_configs,
            check_import_has_credentials,
            // Process commands
            list_instances,
            get_instance,
            start_tunnel,
            stop_tunnel,
            get_custom_binary_path,
            set_custom_binary_path,
            is_using_bundled_binary,
            get_binary_version,
        ])
        .setup(|app| {
            // Set up system tray
            setup_tray(app)?;

            // Get main window and set up close handler to hide instead of quit
            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            // Apply saved settings and emit startup events
            if let Some(state) = app.try_state::<AppState>() {
                // Set app handle for sidecar spawning and apply saved custom binary path
                let app_handle = app.handle().clone();
                tauri::async_runtime::block_on(async {
                    state.process_manager.set_app_handle(app_handle).await;

                    // Apply saved custom binary path if set
                    let binary_path = state.app_settings.lock().await.binary_path.clone();
                    if let Some(path) = binary_path {
                        state
                            .process_manager
                            .set_custom_binary_path(Some(path))
                            .await;
                    }
                });

                // Emit config load error event if there was an error during startup
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
                // Shutdown already in progress — don't call prevent_exit() since the
                // channel may already be dropped (causes panic in tauri's unwrap).
                return;
            }

            // Prevent immediate exit to allow graceful shutdown
            api.prevent_exit();

            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                // Stop all running tunnels with a timeout
                if let Some(state) = app_handle.try_state::<AppState>() {
                    const SHUTDOWN_TIMEOUT: std::time::Duration =
                        std::time::Duration::from_secs(5);

                    match tokio::time::timeout(SHUTDOWN_TIMEOUT, state.process_manager.stop_all())
                        .await
                    {
                        Ok(()) => {
                            tracing::info!("All tunnels stopped gracefully");
                        }
                        Err(_) => {
                            tracing::error!(
                                "Shutdown timeout after {} seconds, force killing remaining processes",
                                SHUTDOWN_TIMEOUT.as_secs()
                            );
                        }
                    }

                    // Force kill any remaining processes to ensure no orphans
                    state.process_manager.force_kill_all().await;
                }
                // Force exit without re-triggering ExitRequested
                std::process::exit(0);
            });
        }
    });
}
