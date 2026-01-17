//! AES-GCM encryption support.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::{CryptoError, Result};

/// Encrypted payload with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedPayload {
    /// Algorithm identifier.
    pub algorithm: String,
    /// Nonce (12 bytes for AES-GCM).
    pub nonce: Vec<u8>,
    /// Ciphertext.
    pub ciphertext: Vec<u8>,
}

/// Encrypt plaintext using AES-256-GCM.
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `plaintext` - Data to encrypt
/// * `aad` - Additional authenticated data (not encrypted, but authenticated)
pub fn encrypt(key: &[u8; 32], plaintext: &[u8], aad: Option<&[u8]>) -> Result<EncryptedPayload> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    Ok(EncryptedPayload {
        algorithm: "AES-256-GCM".to_string(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
    })
}

/// Decrypt ciphertext using AES-256-GCM.
///
/// # Arguments
/// * `key` - 32-byte encryption key
/// * `payload` - Encrypted payload to decrypt
/// * `aad` - Additional authenticated data (must match what was used during encryption)
pub fn decrypt(key: &[u8; 32], payload: &EncryptedPayload, aad: Option<&[u8]>) -> Result<Vec<u8>> {
    if payload.algorithm != "AES-256-GCM" {
        return Err(CryptoError::DecryptionFailed(format!(
            "Unsupported algorithm: {}",
            payload.algorithm
        )));
    }

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let nonce_bytes: [u8; 12] = payload
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| CryptoError::DecryptionFailed("Invalid nonce length".to_string()))?;

    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, payload.ciphertext.as_slice())
        .map_err(|_| CryptoError::DecryptionFailed("Decryption failed".to_string()))
}

/// Derive an encryption key using HKDF.
pub fn derive_key(shared_secret: &[u8], salt: &[u8], info: &[u8]) -> Result<[u8; 32]> {
    use hkdf::Hkdf;
    use sha2::Sha256;

    let hk = Hkdf::<Sha256>::new(Some(salt), shared_secret);
    let mut key = [0u8; 32];
    hk.expand(info, &mut key)
        .map_err(|e| CryptoError::KeyGenerationFailed(e.to_string()))?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0u8; 32]; // Zero key for testing only
        let plaintext = b"Hello, AtomicSettle!";

        let encrypted = encrypt(&key, plaintext, None).unwrap();
        let decrypted = decrypt(&key, &encrypted, None).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_nonces() {
        let key = [0u8; 32];
        let plaintext = b"Same message";

        let enc1 = encrypt(&key, plaintext, None).unwrap();
        let enc2 = encrypt(&key, plaintext, None).unwrap();

        // Nonces should be different
        assert_ne!(enc1.nonce, enc2.nonce);
        // Ciphertexts should be different
        assert_ne!(enc1.ciphertext, enc2.ciphertext);
    }

    #[test]
    fn test_wrong_key() {
        let key1 = [0u8; 32];
        let key2 = [1u8; 32];
        let plaintext = b"Secret message";

        let encrypted = encrypt(&key1, plaintext, None).unwrap();
        let result = decrypt(&key2, &encrypted, None);

        assert!(result.is_err());
    }
}
