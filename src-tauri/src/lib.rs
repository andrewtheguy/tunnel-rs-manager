//! Tauri backend for tunnel-rs-manager

mod age_export;
mod config;
mod crypto;
mod keychain;
mod passphrase;
mod process;

use config::{AppSettings, ConfigStore, ExportData, Forwarding, ImportResult, PassphraseMeta, ServerGroup};
use process::{ProcessManager, TunnelInstanceView};
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
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Guard to prevent multiple shutdown handlers from running
static SHUTDOWN_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Application state shared across commands
pub struct AppState {
    config_store: Mutex<ConfigStore>,
    app_settings: Mutex<AppSettings>,
    process_manager: Arc<ProcessManager>,
    config_load_error: Option<String>,
    cipher: Mutex<Option<crypto::Cipher>>,
    keychain_available: bool,
}

impl AppState {
    fn new() -> Self {
        let keychain_available = keychain::init_store();

        let (config_store, config_load_error) = match ConfigStore::load() {
            Ok(store) => (store, None),
            Err(e) => {
                let error_msg = format!("Failed to load config store: {}. Using default.", e);
                tracing::error!("{}", error_msg);
                (ConfigStore::default(), Some(error_msg))
            }
        };

        let app_settings = match AppSettings::load() {
            Ok(settings) => settings,
            Err(e) => {
                tracing::error!("Failed to load app settings: {}. Using default.", e);
                AppSettings::default()
            }
        };

        Self {
            config_store: Mutex::new(config_store),
            app_settings: Mutex::new(app_settings),
            process_manager: Arc::new(ProcessManager::new()),
            config_load_error,
            cipher: Mutex::new(None),
            keychain_available,
        }
    }
}

// ============================================================================
// Passphrase / Encryption Commands
// ============================================================================

#[tauri::command]
async fn get_encryption_status(state: State<'_, AppState>) -> Result<String, String> {
    let cipher = state.cipher.lock().await;
    if cipher.is_some() {
        return Ok("unlocked".to_string());
    }
    let store = state.config_store.lock().await;
    if store.passphrase_meta.is_some() {
        Ok("locked".to_string())
    } else {
        Ok("not_configured".to_string())
    }
}

#[tauri::command]
async fn setup_passphrase(
    state: State<'_, AppState>,
    instance: String,
    passphrase: String,
    save_to_keychain: bool,
) -> Result<(), String> {
    if instance.is_empty() || instance.len() > 32 {
        return Err("Instance name must be 1-32 characters.".to_string());
    }
    if !instance.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Instance name may only contain letters, digits, hyphens, and underscores.".to_string());
    }
    if let Some(err) = passphrase::passphrase_policy_error(&passphrase) {
        return Err(err.to_string());
    }
    {
        let store = state.config_store.lock().await;
        if store.passphrase_meta.is_some() {
            return Err("Passphrase is already configured. Use unlock_passphrase instead.".to_string());
        }
    }

    let salt = passphrase::generate_salt();
    let salt_b64 = BASE64.encode(salt);

    let key = tokio::task::spawn_blocking({
        let passphrase = passphrase.clone();
        let salt = salt;
        move || passphrase::derive_config_key(&passphrase, &salt)
    })
    .await
    .map_err(|e| format!("key derivation task failed: {e}"))??;

    let instance_sig = passphrase::compute_instance_sig(&instance, &key);

    let meta = PassphraseMeta {
        instance: instance.clone(),
        instance_sig: instance_sig.clone(),
        salt: salt_b64,
    };

    {
        let mut store = state.config_store.lock().await;
        store.passphrase_meta = Some(meta);
        store.save()?;
    }

    let cipher = crypto::Cipher::new(key, instance.clone(), &instance_sig);
    *state.cipher.lock().await = Some(cipher);

    if save_to_keychain && state.keychain_available {
        if let Err(e) = keychain::save_passphrase(&instance, &passphrase) {
            tracing::warn!("Failed to save passphrase to keychain: {e}");
        }
    }

    Ok(())
}

#[tauri::command]
async fn unlock_passphrase(
    state: State<'_, AppState>,
    passphrase: String,
    save_to_keychain: bool,
) -> Result<(), String> {
    let store = state.config_store.lock().await;
    let meta = store
        .passphrase_meta
        .as_ref()
        .ok_or_else(|| "No passphrase configured".to_string())?
        .clone();
    drop(store);

    let salt = BASE64
        .decode(&meta.salt)
        .map_err(|e| format!("invalid salt in config: {e}"))?;

    let key = tokio::task::spawn_blocking({
        let passphrase = passphrase.clone();
        move || passphrase::derive_config_key(&passphrase, &salt)
    })
    .await
    .map_err(|e| format!("key derivation task failed: {e}"))??;

    if !passphrase::verify_instance_sig(&meta.instance, &key, &meta.instance_sig) {
        return Err("Wrong passphrase".to_string());
    }

    let cipher = crypto::Cipher::new(key, meta.instance.clone(), &meta.instance_sig);
    *state.cipher.lock().await = Some(cipher);

    if save_to_keychain && state.keychain_available {
        if let Err(e) = keychain::save_passphrase(&meta.instance, &passphrase) {
            tracing::warn!("Failed to save passphrase to keychain: {e}");
        }
    }

    Ok(())
}

#[tauri::command]
async fn try_keychain_unlock(state: State<'_, AppState>) -> Result<bool, String> {
    if !state.keychain_available {
        return Ok(false);
    }

    let store = state.config_store.lock().await;
    let meta = match store.passphrase_meta.as_ref() {
        Some(m) => m.clone(),
        None => return Ok(false),
    };
    drop(store);

    let passphrase = match keychain::load_passphrase(&meta.instance) {
        Some(p) => p,
        None => return Ok(false),
    };

    let salt = BASE64
        .decode(&meta.salt)
        .map_err(|e| format!("invalid salt in config: {e}"))?;

    let key = tokio::task::spawn_blocking({
        let passphrase = passphrase;
        move || passphrase::derive_config_key(&passphrase, &salt)
    })
    .await
    .map_err(|e| format!("key derivation task failed: {e}"))??;

    if !passphrase::verify_instance_sig(&meta.instance, &key, &meta.instance_sig) {
        tracing::warn!(
            "Keychain passphrase for instance '{}' failed verification, removing stale entry",
            meta.instance
        );
        keychain::delete_passphrase(&meta.instance);
        return Ok(false);
    }

    let cipher = crypto::Cipher::new(key, meta.instance.clone(), &meta.instance_sig);
    *state.cipher.lock().await = Some(cipher);

    Ok(true)
}

// ============================================================================
// Server Group Commands
// ============================================================================

#[tauri::command]
async fn list_server_groups(state: State<'_, AppState>) -> Result<Vec<ServerGroup>, String> {
    let store = state.config_store.lock().await;
    Ok(store.list_server_groups())
}

#[tauri::command]
async fn get_server_group(state: State<'_, AppState>, id: String) -> Result<ServerGroup, String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let store = state.config_store.lock().await;
    store
        .get_server_group(uuid)
        .cloned()
        .ok_or_else(|| "Server group not found".to_string())
}

#[tauri::command]
async fn get_decrypted_tokens(
    state: State<'_, AppState>,
    id: String,
) -> Result<(String, String), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let cipher_guard = state.cipher.lock().await;
    let cipher = cipher_guard
        .as_ref()
        .ok_or_else(|| "Encryption not unlocked".to_string())?;

    let store = state.config_store.lock().await;
    let group = store
        .get_server_group(uuid)
        .ok_or_else(|| "Server group not found".to_string())?;

    let auth = match &group.auth_token {
        Some(token) if crypto::is_encrypted(token) => cipher.decrypt(token)?,
        Some(token) => token.clone(),
        None => String::new(),
    };

    let alpn = match &group.alpn_token {
        Some(token) if crypto::is_encrypted(token) => cipher.decrypt(token)?,
        Some(token) => token.clone(),
        None => String::new(),
    };

    Ok((auth, alpn))
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

    let cipher_guard = state.cipher.lock().await;
    let cipher = cipher_guard
        .as_ref()
        .ok_or_else(|| "Encryption not unlocked".to_string())?;

    let encrypted_auth = cipher.encrypt(&auth_token)?;
    let encrypted_alpn = cipher.encrypt(&alpn_token)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let group = ServerGroup {
        id: Uuid::new_v4(),
        name,
        server_node_id,
        auth_token: Some(encrypted_auth),
        alpn_token: Some(encrypted_alpn),
        relay_urls: relay_urls.unwrap_or_default(),
        created_at: now,
        updated_at: now,
    };

    let mut store = state.config_store.lock().await;
    store.upsert_server_group(group.clone())?;

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

    let cipher_guard = state.cipher.lock().await;
    let cipher = cipher_guard
        .as_ref()
        .ok_or_else(|| "Encryption not unlocked".to_string())?;

    let encrypted_auth = cipher.encrypt(&auth_token)?;
    let encrypted_alpn = cipher.encrypt(&alpn_token)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("System time error: {}", e))?
        .as_secs();

    let mut store = state.config_store.lock().await;
    let existing = store
        .get_server_group(uuid)
        .ok_or_else(|| "Server group not found".to_string())?;
    let created_at = existing.created_at;

    let group = ServerGroup {
        id: uuid,
        name,
        server_node_id,
        auth_token: Some(encrypted_auth),
        alpn_token: Some(encrypted_alpn),
        relay_urls: relay_urls.unwrap_or_default(),
        created_at,
        updated_at: now,
    };

    store.upsert_server_group(group.clone())?;

    Ok(group)
}

#[tauri::command]
async fn delete_server_group(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let mut store = state.config_store.lock().await;
    store.delete_server_group(uuid)
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
) -> Result<String, String> {
    let store = state.config_store.lock().await;
    let export_data = store.export();
    serde_json::to_string_pretty(&export_data)
        .map_err(|e| format!("Failed to serialize export data: {}", e))
}

#[tauri::command]
async fn import_configs(
    state: State<'_, AppState>,
    json: String,
) -> Result<ImportResult, String> {
    let export_data: ExportData = serde_json::from_str(&json)
        .map_err(|e| format!("Invalid import data: {}", e))?;

    let mut store = state.config_store.lock().await;
    Ok(store.import(export_data))
}

// ============================================================================
// Export Forwarding as TOML
// ============================================================================

/// Placeholder written for secret tokens when no age recipient is provided.
const TOKEN_PLACEHOLDER: &str = "REPLACE_WITH_ENCRYPTED_TOKEN";

#[tauri::command]
async fn export_forwarding_toml(
    state: State<'_, AppState>,
    forwarding_id: String,
    encryption_recipient: Option<String>,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&forwarding_id).map_err(|e| format!("Invalid UUID: {}", e))?;

    // Treat empty/whitespace-only recipient as "no recipient".
    let recipient = encryption_recipient
        .map(|r| r.trim().to_string())
        .filter(|r| !r.is_empty());

    let (forwarding_name, group_name, mut config) = {
        let store = state.config_store.lock().await;

        let forwarding = store
            .get_forwarding(uuid)
            .ok_or_else(|| "Forwarding not found".to_string())?;
        let forwarding_name = forwarding.name.clone();
        let group_id = forwarding.server_group_id;

        let group = store
            .get_server_group(group_id)
            .ok_or_else(|| "Server group not found".to_string())?;
        let group_name = group.name.clone();

        let config = store.build_tunnel_config(uuid)?;
        (forwarding_name, group_name, config)
    };

    match recipient {
        Some(r) => {
            // Validate before doing anything else so a bad key fails cleanly.
            age_export::validate_recipient(&r)?;

            // Re-encrypt the stored tokens to the age recipient. This needs the
            // passphrase cipher to first decrypt our own at-rest AES blobs.
            let cipher_guard = state.cipher.lock().await;
            let cipher = cipher_guard
                .as_ref()
                .ok_or_else(|| "Encryption not unlocked".to_string())?;

            for token in [&mut config.iroh.auth_token, &mut config.iroh.alpn_token] {
                if let Some(value) = token {
                    let plaintext = if crypto::is_encrypted(value) {
                        cipher.decrypt(value)?
                    } else {
                        value.clone()
                    };
                    *token = Some(age_export::encrypt_value(&plaintext, &r)?);
                }
            }
            drop(cipher_guard);

            config.iroh.encryption_recipient = Some(r.clone());

            let toml = config.to_commented_toml(&forwarding_name, &group_name);

            // Remember the recipient to prefill the dialog next time.
            {
                let mut settings = state.app_settings.lock().await;
                settings.age_recipient = Some(r);
                settings.save()?;
            }

            Ok(toml)
        }
        None => {
            // No recipient: never emit real secrets, only placeholders.
            for token in [&mut config.iroh.auth_token, &mut config.iroh.alpn_token] {
                if token.is_some() {
                    *token = Some(TOKEN_PLACEHOLDER.to_string());
                }
            }
            Ok(config.to_commented_toml(&forwarding_name, &group_name))
        }
    }
}

#[tauri::command]
async fn get_age_recipient(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let settings = state.app_settings.lock().await;
    Ok(settings.age_recipient.clone())
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

    let (forwarding_name, server_group_name, config) = {
        let cipher_guard = state.cipher.lock().await;
        let cipher = cipher_guard
            .as_ref()
            .ok_or_else(|| "Encryption not unlocked".to_string())?;

        let store = state.config_store.lock().await;

        let forwarding = store
            .get_forwarding(uuid)
            .ok_or_else(|| "Forwarding not found".to_string())?;
        let forwarding_name = forwarding.name.clone();

        let group = store
            .get_server_group(forwarding.server_group_id)
            .ok_or_else(|| "Server group not found".to_string())?;
        let server_group_name = group.name.clone();

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

        let mut config = store.build_tunnel_config(uuid)?;

        if let Some(ref auth) = config.iroh.auth_token {
            if crypto::is_encrypted(auth) {
                config.iroh.auth_token = Some(cipher.decrypt(auth)?);
            }
        }
        if let Some(ref alpn) = config.iroh.alpn_token {
            if crypto::is_encrypted(alpn) {
                config.iroh.alpn_token = Some(cipher.decrypt(alpn)?);
            }
        }

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
    {
        let mut settings = state.app_settings.lock().await;
        settings.binary_path = path.clone();
        settings.save()?;
    }
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

    let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray@2x.png"))?;

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .icon_as_template(true)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
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
                    let _ = window.show();
                    let _ = window.set_focus();
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
            // Passphrase commands
            get_encryption_status,
            setup_passphrase,
            unlock_passphrase,
            try_keychain_unlock,
            // Server Group commands
            list_server_groups,
            get_server_group,
            get_decrypted_tokens,
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
            export_forwarding_toml,
            get_age_recipient,
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
            setup_tray(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let window_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_clone.hide();
                    }
                });
            }

            if let Some(state) = app.try_state::<AppState>() {
                let app_handle = app.handle().clone();
                tauri::async_runtime::block_on(async {
                    state.process_manager.set_app_handle(app_handle).await;

                    let binary_path = state.app_settings.lock().await.binary_path.clone();
                    if let Some(path) = binary_path {
                        state
                            .process_manager
                            .set_custom_binary_path(Some(path))
                            .await;
                    }
                });

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
            if SHUTDOWN_IN_PROGRESS
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return;
            }

            api.prevent_exit();

            let app_handle = app_handle.clone();
            tauri::async_runtime::spawn(async move {
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

                    state.process_manager.force_kill_all().await;
                }
                std::process::exit(0);
            });
        }
    });
}
