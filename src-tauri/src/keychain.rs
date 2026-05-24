const SERVICE: &str = "tunnel-rs-manager";

pub(crate) fn init_store() -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Ok(store) = apple_native_keyring_store::keychain::Store::new() {
            keyring_core::set_default_store(store);
            return true;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(store) = dbus_secret_service_keyring_store::Store::new() {
            keyring_core::set_default_store(store);
            return true;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(store) = windows_native_keyring_store::Store::new() {
            keyring_core::set_default_store(store);
            return true;
        }
    }
    false
}

pub(crate) fn save_passphrase(instance: &str, passphrase: &str) -> Result<(), String> {
    let entry =
        keyring_core::Entry::new(SERVICE, instance).map_err(|e| format!("keychain entry: {e}"))?;
    entry
        .set_password(passphrase)
        .map_err(|e| format!("keychain save: {e}"))
}

pub(crate) fn load_passphrase(instance: &str) -> Option<String> {
    let entry = match keyring_core::Entry::new(SERVICE, instance) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("keychain entry error for instance '{instance}': {e}");
            return None;
        }
    };
    match entry.get_password() {
        Ok(pw) => Some(pw),
        Err(keyring_core::Error::NoEntry) => None,
        Err(e) => {
            tracing::warn!("keychain load error for instance '{instance}': {e}");
            None
        }
    }
}

pub(crate) fn delete_passphrase(instance: &str) {
    if let Ok(entry) = keyring_core::Entry::new(SERVICE, instance) {
        if let Err(e) = entry.delete_credential() {
            tracing::warn!("keychain delete error for instance '{instance}': {e}");
        }
    }
}
