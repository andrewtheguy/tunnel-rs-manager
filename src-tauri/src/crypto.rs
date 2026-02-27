use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::Aead,
};
use argon2::Argon2;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedToken {
    pub alg: String,
    pub iv: String,
    pub salt: String,
    pub data: String,
}

pub fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.len() < 12 {
        return Err("Passphrase must be at least 12 characters".to_string());
    }
    if !passphrase.chars().any(|c| c.is_uppercase()) {
        return Err("Passphrase must contain at least one uppercase letter".to_string());
    }
    if !passphrase.chars().any(|c| c.is_lowercase()) {
        return Err("Passphrase must contain at least one lowercase letter".to_string());
    }
    if !passphrase.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Passphrase must contain at least one symbol".to_string());
    }
    Ok(())
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32], String> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Key derivation failed: {}", e))?;
    Ok(key)
}

pub fn encrypt_token(token: &str, passphrase: &str) -> Result<EncryptedToken, String> {
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce_bytes);

    let key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, token.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    Ok(EncryptedToken {
        alg: "aes256gcm".to_string(),
        iv: BASE64.encode(nonce_bytes),
        salt: BASE64.encode(salt),
        data: BASE64.encode(ciphertext),
    })
}

pub fn decrypt_token(envelope: &EncryptedToken, passphrase: &str) -> Result<String, String> {
    if envelope.alg != "aes256gcm" {
        return Err(format!("Unsupported algorithm: {}", envelope.alg));
    }

    let salt = BASE64.decode(&envelope.salt)
        .map_err(|e| format!("Invalid salt encoding: {}", e))?;
    let nonce_bytes = BASE64.decode(&envelope.iv)
        .map_err(|e| format!("Invalid IV encoding: {}", e))?;
    let ciphertext = BASE64.decode(&envelope.data)
        .map_err(|e| format!("Invalid data encoding: {}", e))?;

    if nonce_bytes.len() != 12 {
        return Err("Invalid IV length".to_string());
    }

    let key = derive_key(passphrase, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed: wrong passphrase or corrupted data".to_string())?;

    String::from_utf8(plaintext)
        .map_err(|e| format!("Decrypted data is not valid UTF-8: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let token = "my_secret_token_value";
        let passphrase = "MySecure!Pass12";
        let envelope = encrypt_token(token, passphrase).unwrap();
        let decrypted = decrypt_token(&envelope, passphrase).unwrap();
        assert_eq!(token, decrypted);
    }

    #[test]
    fn wrong_passphrase_fails() {
        let envelope = encrypt_token("secret", "MySecure!Pass12").unwrap();
        let result = decrypt_token(&envelope, "WrongPass!phrase");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrong passphrase"));
    }

    #[test]
    fn validation_too_short() {
        assert!(validate_passphrase("Short!aA").is_err());
    }

    #[test]
    fn validation_no_uppercase() {
        assert!(validate_passphrase("alllowercase!1").is_err());
    }

    #[test]
    fn validation_no_lowercase() {
        assert!(validate_passphrase("ALLUPPERCASE!1").is_err());
    }

    #[test]
    fn validation_no_symbol() {
        assert!(validate_passphrase("NoSymbolsHere1").is_err());
    }

    #[test]
    fn validation_valid() {
        assert!(validate_passphrase("ValidPass!123").is_ok());
    }
}
