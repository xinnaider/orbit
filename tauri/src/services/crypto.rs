/// Simple AES-256-GCM encryption for storing secrets in the database.
///
/// A 256-bit key is generated on first launch and stored at
/// `{app_data}/orbit.key`. This prevents credentials from being
/// stored in plaintext in the SQLite file.
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::prelude::*;
use rand::RngCore;
use std::path::{Path, PathBuf};

/// Global encryption key, initialized once at app startup.
static KEY: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();

/// Initialize the encryption key from disk, or generate a new one.
/// Must be called once at app startup with the app data directory.
pub fn init(data_dir: &Path) {
    KEY.get_or_init(|| load_or_create_key(data_dir));
}

fn key_path(data_dir: &Path) -> PathBuf {
    data_dir.join("orbit.key")
}

fn load_or_create_key(data_dir: &Path) -> [u8; 32] {
    let path = key_path(data_dir);
    if let Ok(bytes) = std::fs::read(&path) {
        if bytes.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&bytes);
            return key;
        }
    }
    // Generate new key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let _ = std::fs::write(&path, key);
    key
}

fn get_key() -> &'static [u8; 32] {
    KEY.get()
        .expect("crypto::init() must be called before encrypt/decrypt")
}

/// Encrypt a plaintext string. Returns a base64 string (nonce + ciphertext).
pub fn encrypt(plaintext: &str) -> Result<String, String> {
    let key = get_key();
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;

    // Prepend nonce to ciphertext and base64 encode
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64_STANDARD.encode(&combined))
}

/// Decrypt a base64 string (nonce + ciphertext) back to plaintext.
pub fn decrypt(encoded: &str) -> Result<String, String> {
    let key = get_key();
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| format!("cipher init: {e}"))?;

    let combined = BASE64_STANDARD
        .decode(encoded)
        .map_err(|e| format!("base64 decode: {e}"))?;

    if combined.len() < 12 {
        return Err("ciphertext too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("decrypt: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| format!("utf8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test_key() {
        KEY.get_or_init(|| {
            let mut key = [0u8; 32];
            OsRng.fill_bytes(&mut key);
            key
        });
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        init_test_key();
        let original = "my-secret-api-key-12345";
        let encrypted = encrypt(original).unwrap();
        assert_ne!(encrypted, original);
        let decrypted = decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn encrypt_produces_different_ciphertext_each_time() {
        init_test_key();
        let text = "same-input";
        let a = encrypt(text).unwrap();
        let b = encrypt(text).unwrap();
        assert_ne!(a, b); // different nonce → different ciphertext
        assert_eq!(decrypt(&a).unwrap(), text);
        assert_eq!(decrypt(&b).unwrap(), text);
    }

    #[test]
    fn decrypt_invalid_base64_returns_error() {
        init_test_key();
        assert!(decrypt("not-valid-base64!!!").is_err());
    }

    #[test]
    fn decrypt_short_data_returns_error() {
        init_test_key();
        let short = BASE64_STANDARD.encode(b"short");
        assert!(decrypt(&short).is_err());
    }
}
