use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, AeadCore, KeyInit, OsRng}};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

const HEADER: &str = "$TM;1.0;AES-256-GCM;";
const SIG_PREFIX_LEN: usize = 8;

pub fn is_encrypted(value: &str) -> bool {
    value.trim().starts_with(HEADER)
}

fn truncate_sig(instance_sig: &str) -> String {
    instance_sig.chars().take(SIG_PREFIX_LEN).collect()
}

pub struct Cipher {
    key: [u8; 32],
    instance: String,
    sig_prefix: String,
}

impl std::fmt::Debug for Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cipher")
            .field("instance", &self.instance)
            .field("sig_prefix", &self.sig_prefix)
            .finish()
    }
}

impl Cipher {
    pub fn new(key: [u8; 32], instance: String, instance_sig: &str) -> Self {
        Self {
            key,
            instance,
            sig_prefix: truncate_sig(instance_sig),
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let mut blob = cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|e| format!("aes-256-gcm encrypt: {e}"))?;
        let mut out = Vec::with_capacity(nonce.len() + blob.len());
        out.extend_from_slice(nonce.as_slice());
        out.append(&mut blob);
        Ok(format!(
            "{HEADER}{};{};{}",
            self.instance,
            self.sig_prefix,
            BASE64.encode(&out)
        ))
    }

    pub fn decrypt(&self, value: &str) -> Result<String, String> {
        let after_header = value
            .trim()
            .strip_prefix(HEADER)
            .ok_or_else(|| format!("value is not encrypted (missing header)"))?;
        let (_, rest) = after_header
            .split_once(';')
            .ok_or_else(|| "malformed encrypted value (missing instance separator)".to_string())?;
        let (_, encoded) = rest
            .split_once(';')
            .ok_or_else(|| "malformed encrypted value (missing sig separator)".to_string())?;
        let raw = BASE64
            .decode(encoded)
            .map_err(|e| format!("invalid base64 in encrypted value: {e}"))?;
        if raw.len() < 12 + 16 {
            return Err(format!("encrypted payload too short ({} bytes)", raw.len()));
        }
        let (nonce_bytes, ciphertext) = raw.split_at(12);
        let cipher = Aes256Gcm::new(aes_gcm::Key::<Aes256Gcm>::from_slice(&self.key));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|e| format!("aes-256-gcm decrypt: {e}"))?;
        String::from_utf8(plaintext).map_err(|e| format!("decrypted value is not valid UTF-8: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_detection() {
        assert!(is_encrypted(
            "$TM;1.0;AES-256-GCM;mysite;abcd1234;AAAA"
        ));
        assert!(!is_encrypted(""));
        assert!(!is_encrypted("plain"));
        assert!(!is_encrypted("ageenc:something"));
    }

    #[test]
    fn round_trip() {
        let cipher = Cipher::new([0x42u8; 32], "test".into(), "abcd12345678");
        let enc = cipher.encrypt("hunter2").unwrap();
        assert!(enc.starts_with("$TM;1.0;AES-256-GCM;test;abcd1234;"));
        assert!(!enc.contains('\n'));
        assert_eq!(cipher.decrypt(&enc).unwrap(), "hunter2");
    }

    #[test]
    fn wrong_key_fails() {
        let c1 = Cipher::new([0x01u8; 32], "inst".into(), "sig12345678");
        let c2 = Cipher::new([0x02u8; 32], "inst".into(), "sig12345678");
        let enc = c1.encrypt("secret").unwrap();
        assert!(c2.decrypt(&enc).is_err());
    }

    #[test]
    fn tampered_fails() {
        let cipher = Cipher::new([0x33u8; 32], "inst".into(), "sig12345678");
        let enc = cipher.encrypt("data").unwrap();
        let semi_pos = enc.rfind(';').unwrap();
        let mut tampered = enc.clone();
        let bytes = unsafe { tampered.as_bytes_mut() };
        let i = semi_pos + 6;
        bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
        assert!(cipher.decrypt(&tampered).is_err());
    }

    #[test]
    fn sig_prefix_truncated_to_8_chars() {
        assert_eq!(truncate_sig("wgMS4JCMUouQZRnB4vpscg=="), "wgMS4JCM");
        assert_eq!(truncate_sig("short"), "short");
        assert_eq!(truncate_sig(""), "");
    }
}
