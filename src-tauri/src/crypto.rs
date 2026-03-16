use age::secrecy::ExposeSecret;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use std::path::{Path, PathBuf};

const AGEENC_PREFIX: &str = "ageenc:";

/// Get the default age key file path: `<app_data_dir>/age.key`
pub fn default_age_key_path() -> Result<PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Could not find data directory".to_string())?;
    Ok(data_dir.join("tunnel-rs-manager").join("age.key"))
}

/// Check if an age key file exists at the default path.
pub fn age_key_exists() -> Result<bool, String> {
    Ok(default_age_key_path()?.exists())
}

/// Generate a new age x25519 keypair, append to default key file, return public key.
pub fn generate_age_key() -> Result<String, String> {
    let identity = age::x25519::Identity::generate();
    let secret = identity.to_string();
    let public_key = identity.to_public().to_string();

    let path = default_age_key_path()?;
    write_identity_file(&path, secret.expose_secret(), &public_key, false)?;

    Ok(public_key)
}

/// Read all public keys from the age key file (lines matching `# public key: age1...`).
pub fn list_age_recipients() -> Result<Vec<String>, String> {
    let path = default_age_key_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read age key file: {}", e))?;
    let recipients: Vec<String> = contents
        .lines()
        .filter_map(|l| l.strip_prefix("# public key: "))
        .map(|s| s.trim().to_string())
        .filter(|s| s.starts_with("age1"))
        .collect();
    Ok(recipients)
}

/// Check if a string value is an age-encrypted value (has `ageenc:` prefix).
pub fn is_age_encrypted(value: &str) -> bool {
    value.trim().starts_with(AGEENC_PREFIX)
}

/// Encrypt a plaintext string for the given age recipient (public key).
/// Returns `ageenc:<base64>`.
pub fn encrypt_value(plaintext: &str, recipient_str: &str) -> Result<String, String> {
    let recipient: age::x25519::Recipient = recipient_str
        .parse()
        .map_err(|e| format!("Invalid age recipient '{}': {}", recipient_str, e))?;
    let ciphertext = age::encrypt(&recipient, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    Ok(format!("{}{}", AGEENC_PREFIX, BASE64.encode(&ciphertext)))
}

/// Decrypt an `ageenc:`-prefixed value using the identity file at `key_path`.
/// age automatically tries all identities in the file.
pub fn decrypt_value(value: &str, key_path: &Path) -> Result<String, String> {
    let encoded = value
        .trim()
        .strip_prefix(AGEENC_PREFIX)
        .ok_or_else(|| format!("Value does not start with '{}'", AGEENC_PREFIX))?;
    let ciphertext = BASE64
        .decode(encoded)
        .map_err(|e| format!("Invalid base64 in ageenc: value: {}", e))?;

    let contents = std::fs::read_to_string(key_path)
        .map_err(|e| format!("Failed to read age key file: {}", e))?;

    // Parse all identities from the file
    let identities: Vec<age::x25519::Identity> = contents
        .lines()
        .filter(|l| l.starts_with("AGE-SECRET-KEY-"))
        .filter_map(|l| l.parse::<age::x25519::Identity>().ok())
        .collect();

    if identities.is_empty() {
        return Err(format!(
            "No AGE-SECRET-KEY found in {}",
            key_path.display()
        ));
    }

    // Try each identity
    for identity in &identities {
        if let Ok(plaintext) = age::decrypt(identity, &ciphertext) {
            return String::from_utf8(plaintext)
                .map_err(|e| format!("Decrypted value is not valid UTF-8: {}", e));
        }
    }

    Err("Age decryption failed: no matching key found".to_string())
}

/// Write an age identity file with restricted permissions.
/// If the file exists and `force` is false, the new keypair is appended.
pub fn write_identity_file(
    path: &Path,
    secret_key: &str,
    public_key: &str,
    force: bool,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directory: {}", e))?;
    }

    let append = path.exists() && !force;
    let now = jiff::Zoned::now().strftime("%Y-%m-%dT%H:%M:%S%:z");
    let block = format!(
        "# created: {}\n# public key: {}\n{}\n",
        now, public_key, secret_key
    );
    let content = if append {
        format!("\n{}", block)
    } else {
        block
    };

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("Failed to open age key file: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write age key file: {}", e))?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set age key file permissions: {}", e))?;
    }

    #[cfg(not(unix))]
    {
        if append {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .map_err(|e| format!("Failed to open age key file: {}", e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to append to age key file: {}", e))?;
        } else {
            std::fs::write(path, &content)
                .map_err(|e| format!("Failed to write age key file: {}", e))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_age_encrypted() {
        assert!(is_age_encrypted("ageenc:YWdl..."));
        assert!(is_age_encrypted("  ageenc:YWdl...  "));
        assert!(!is_age_encrypted("plaintext_token"));
        assert!(!is_age_encrypted(""));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let identity = age::x25519::Identity::generate();
        let secret = identity.to_string();
        let public_key = identity.to_public().to_string();

        let plaintext = "my-secret-token-value";
        let encrypted = encrypt_value(plaintext, &public_key).unwrap();

        assert!(is_age_encrypted(&encrypted));
        assert!(encrypted.starts_with("ageenc:"));
        assert!(!encrypted.contains('\n'));

        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("age.key");
        write_identity_file(&key_path, secret.expose_secret(), &public_key, false).unwrap();

        let decrypted = decrypt_value(&encrypted, &key_path).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key() {
        let id1 = age::x25519::Identity::generate();
        let secret1 = id1.to_string();
        let public1 = id1.to_public().to_string();

        let id2 = age::x25519::Identity::generate();
        let public2 = id2.to_public().to_string();

        let encrypted = encrypt_value("secret", &public2).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("age.key");
        write_identity_file(&key_path, secret1.expose_secret(), &public1, false).unwrap();

        let result = decrypt_value(&encrypted, &key_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_and_list() {
        let identity = age::x25519::Identity::generate();
        let secret = identity.to_string();
        let public_key = identity.to_public().to_string();

        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("age.key");
        write_identity_file(&key_path, secret.expose_secret(), &public_key, false).unwrap();

        let contents = std::fs::read_to_string(&key_path).unwrap();
        let recipients: Vec<String> = contents
            .lines()
            .filter_map(|l| l.strip_prefix("# public key: "))
            .map(|s| s.trim().to_string())
            .filter(|s| s.starts_with("age1"))
            .collect();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0], public_key);
    }

    #[test]
    fn test_write_identity_file_appends() {
        let dir = tempfile::tempdir().unwrap();
        let key_path = dir.path().join("age.key");

        let id1 = age::x25519::Identity::generate();
        let s1 = id1.to_string();
        let p1 = id1.to_public().to_string();
        write_identity_file(&key_path, s1.expose_secret(), &p1, false).unwrap();

        let id2 = age::x25519::Identity::generate();
        let s2 = id2.to_string();
        let p2 = id2.to_public().to_string();
        write_identity_file(&key_path, s2.expose_secret(), &p2, false).unwrap();

        let contents = std::fs::read_to_string(&key_path).unwrap();
        let secret_lines: Vec<&str> = contents
            .lines()
            .filter(|l| l.starts_with("AGE-SECRET-KEY-"))
            .collect();
        assert_eq!(secret_lines.len(), 2);
    }
}
