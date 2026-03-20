use std::path::{Path, PathBuf};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use keyring::Entry;
use rand::{rngs::OsRng, RngCore};
use serde_json::Value;

/// Simple encrypted secure store for Linux.
///
/// MVP implementation:
/// - Stores a per-device master key in the OS keyring (`keyring` crate).
/// - Encrypts each item using AES-256-GCM and stores it as a file under app config.
#[derive(Clone)]
pub struct SecureStore {
    base_dir: PathBuf,
    master_key: [u8; 32],
}

impl SecureStore {
    pub async fn new_default() -> Self {
        // No async work today, but keep API consistent.
        let base_dir = directories::ProjectDirs::from("com", "konstruct", "construct-desktop")
            .expect("ProjectDirs")
            .config_dir()
            .join("secure_store");

        std::fs::create_dir_all(&base_dir).ok();

        let master_key = load_or_create_master_key();
        Self { base_dir, master_key }
    }

    pub fn put_bytes(&self, key: &str, data: &[u8]) -> anyhow::Result<()> {
        let path = self.item_path(key);
        let encrypted = encrypt(&self.master_key, data)?;
        std::fs::write(path, encrypted)?;
        Ok(())
    }

    pub fn get_bytes(&self, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.item_path(key);
        if !path.exists() {
            return Ok(None);
        }
        let encrypted = std::fs::read(path)?;
        let decrypted = decrypt(&self.master_key, &encrypted)?;
        Ok(Some(decrypted))
    }

    pub fn delete(&self, key: &str) -> anyhow::Result<()> {
        let path = self.item_path(key);
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        Ok(())
    }

    fn item_path(&self, key: &str) -> PathBuf {
        // Simple filename mapping. We keep key as-is; it must be safe for filenames.
        // For our core integration keys are predictable (session_<contact>, etc.).
        let safe: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect();
        self.base_dir.join(format!("{safe}.bin"))
    }
}

fn load_or_create_master_key() -> [u8; 32] {
    let entry = Entry::new("construct-desktop", "master_key")
        .expect("keyring entry");
    if let Ok(s) = entry.get_password() {
        if let Ok(v) = serde_json::from_str::<Value>(&s) {
            if let Some(k) = v.get("k").and_then(|x| x.as_str()) {
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(k) {
                    if bytes.len() == 32 {
                        let mut out = [0u8; 32];
                        out.copy_from_slice(&bytes);
                        return out;
                    }
                }
            }
        }
    }

    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let payload = serde_json::json!({
        "k": base64::engine::general_purpose::STANDARD.encode(key)
    })
    .to_string();
    let _ = entry.set_password(&payload);
    key
}

fn encrypt(master_key: &[u8; 32], plaintext: &[u8]) -> anyhow::Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::<Aes256Gcm>::from_slice(&nonce_bytes);
    let ciphertext = cipher.encrypt(nonce, plaintext)?;
    let mut out = Vec::with_capacity(12 + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt(master_key: &[u8; 32], encrypted: &[u8]) -> anyhow::Result<Vec<u8>> {
    if encrypted.len() < 12 {
        anyhow::bail!("encrypted blob too short");
    }
    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(master_key));
    let nonce = Nonce::<Aes256Gcm>::from_slice(nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext)?;
    Ok(plaintext)
}

