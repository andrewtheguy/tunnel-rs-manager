//! Age encryption for exporting tunnel-rs-compatible config files.
//!
//! tunnel-rs reads inline secrets in the form `auth_token = "ageenc:<base64>"`,
//! where the payload is binary age ciphertext encrypted to an age recipient
//! (public key). This module mirrors `../tunnel-rs/src/encryption.rs` so that
//! configs exported here can be decrypted by tunnel-rs at runtime via its
//! `[iroh].encryption_key_file` identity.

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

const AGEENC_PREFIX: &str = "ageenc:";

/// Validate that `s` is a parseable age x25519 recipient (public key, `age1…`).
pub fn validate_recipient(s: &str) -> Result<(), String> {
    s.trim()
        .parse::<age::x25519::Recipient>()
        .map(|_| ())
        .map_err(|e| format!("Invalid age recipient '{s}': {e}"))
}

/// Encrypt `plaintext` for the given age recipient (public key).
///
/// Returns `ageenc:<base64>` — a single-line value suitable for TOML, byte-for-byte
/// compatible with tunnel-rs's `encrypt_value`.
pub fn encrypt_value(plaintext: &str, recipient: &str) -> Result<String, String> {
    let recipient: age::x25519::Recipient = recipient
        .trim()
        .parse()
        .map_err(|e| format!("Invalid age recipient '{recipient}': {e}"))?;
    let ciphertext = age::encrypt(&recipient, plaintext.as_bytes())
        .map_err(|e| format!("age encryption failed: {e}"))?;
    Ok(format!("{AGEENC_PREFIX}{}", BASE64.encode(&ciphertext)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_recipient() {
        assert!(validate_recipient("not-an-age-key").is_err());
        assert!(validate_recipient("").is_err());
    }

    #[test]
    fn encrypt_round_trips_with_age() {
        let identity = age::x25519::Identity::generate();
        let recipient = identity.to_public().to_string();
        assert!(recipient.starts_with("age1"));
        assert!(validate_recipient(&recipient).is_ok());

        let encrypted = encrypt_value("my-secret-token", &recipient).unwrap();
        assert!(encrypted.starts_with(AGEENC_PREFIX));
        assert!(!encrypted.contains('\n'));

        let raw = BASE64
            .decode(encrypted.strip_prefix(AGEENC_PREFIX).unwrap())
            .unwrap();
        let plaintext = age::decrypt(&identity, &raw).unwrap();
        assert_eq!(plaintext, b"my-secret-token");
    }
}
