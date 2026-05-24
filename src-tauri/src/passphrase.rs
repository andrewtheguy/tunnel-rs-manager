use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, KeyInit, Mac};
use scrypt::Params as ScryptParams;
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

const SCRYPT_LOG_N: u8 = 16;
const SCRYPT_R: u32 = 8;
const SCRYPT_P: u32 = 1;

pub(crate) fn derive_config_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    let params = ScryptParams::new(SCRYPT_LOG_N, SCRYPT_R, SCRYPT_P)
        .map_err(|e| format!("invalid scrypt parameters: {e}"))?;
    scrypt::scrypt(passphrase.as_bytes(), salt, &params, &mut key)
        .map_err(|e| format!("scrypt failed: {e}"))?;
    Ok(key)
}

pub(crate) fn compute_instance_sig(instance: &str, key: &[u8; 32]) -> String {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(instance.as_bytes());
    BASE64.encode(mac.finalize().into_bytes())
}

pub(crate) fn verify_instance_sig(instance: &str, key: &[u8; 32], expected_sig_b64: &str) -> bool {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(instance.as_bytes());
    let computed = mac.finalize().into_bytes();
    let expected = match BASE64.decode(expected_sig_b64) {
        Ok(b) => b,
        Err(_) => return false,
    };
    computed.ct_eq(&expected).into()
}

pub(crate) fn generate_salt() -> [u8; 32] {
    rand::random()
}

pub(crate) fn passphrase_policy_error(passphrase: &str) -> Option<&'static str> {
    if passphrase.chars().count() < 12 {
        return Some("Must be at least 12 characters.");
    }
    if !passphrase.chars().any(|c| c.is_ascii_lowercase()) {
        return Some("Must contain a lowercase letter.");
    }
    if !passphrase.chars().any(|c| c.is_ascii_uppercase()) {
        return Some("Must contain an uppercase letter.");
    }
    if !passphrase.chars().any(|c| c.is_ascii_digit()) {
        return Some("Must contain a digit.");
    }
    if !passphrase.chars().any(|c| c.is_ascii_punctuation()) {
        return Some("Must contain a special character.");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [0x42u8; 32];
        let k1 = derive_config_key("test-passphrase", &salt).unwrap();
        let k2 = derive_config_key("test-passphrase", &salt).unwrap();
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_passphrase_different_key() {
        let salt = [0x42u8; 32];
        let k1 = derive_config_key("passphrase-one", &salt).unwrap();
        let k2 = derive_config_key("passphrase-two", &salt).unwrap();
        assert_ne!(k1, k2);
    }

    #[test]
    fn instance_sig_roundtrip() {
        let key = [0x42u8; 32];
        let sig = compute_instance_sig("mysite", &key);
        assert!(verify_instance_sig("mysite", &key, &sig));
    }

    #[test]
    fn instance_sig_wrong_key() {
        let key = [0x42u8; 32];
        let sig = compute_instance_sig("mysite", &key);
        let wrong_key = [0x43u8; 32];
        assert!(!verify_instance_sig("mysite", &wrong_key, &sig));
    }

    #[test]
    fn instance_sig_wrong_instance() {
        let key = [0x42u8; 32];
        let sig = compute_instance_sig("mysite", &key);
        assert!(!verify_instance_sig("other", &key, &sig));
    }
}
