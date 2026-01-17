//! Digital signature support using Ed25519.

use ed25519_dalek::{
    Signer, SigningKey as Ed25519SigningKey, Verifier, VerifyingKey as Ed25519VerifyingKey,
};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::{CryptoError, Result};

/// A signing key (private key) for creating signatures.
pub struct SigningKey {
    inner: Ed25519SigningKey,
    key_id: String,
}

impl SigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Result<Self> {
        let mut csprng = OsRng;
        let inner = Ed25519SigningKey::generate(&mut csprng);
        let key_id = hex::encode(&inner.verifying_key().as_bytes()[..8]);

        Ok(Self { inner, key_id })
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("Invalid key length".to_string()))?;

        let inner = Ed25519SigningKey::from_bytes(&bytes);
        let key_id = hex::encode(&inner.verifying_key().as_bytes()[..8]);

        Ok(Self { inner, key_id })
    }

    /// Get the corresponding verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            inner: self.inner.verifying_key(),
            key_id: self.key_id.clone(),
        }
    }

    /// Get the key ID.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        let sig = self.inner.sign(message);
        Signature {
            bytes: sig.to_bytes().to_vec(),
            key_id: self.key_id.clone(),
            algorithm: "Ed25519".to_string(),
        }
    }

    /// Get raw key bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }
}

/// A verifying key (public key) for verifying signatures.
#[derive(Clone)]
pub struct VerifyingKey {
    inner: Ed25519VerifyingKey,
    key_id: String,
}

impl VerifyingKey {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| CryptoError::InvalidKey("Invalid key length".to_string()))?;

        let inner = Ed25519VerifyingKey::from_bytes(&bytes)
            .map_err(|e| CryptoError::InvalidKey(e.to_string()))?;

        let key_id = hex::encode(&bytes[..8]);

        Ok(Self { inner, key_id })
    }

    /// Get the key ID.
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Verify a signature.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        let sig_bytes: [u8; 64] = signature
            .bytes
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;

        let sig = ed25519_dalek::Signature::from_bytes(&sig_bytes);

        self.inner
            .verify(message, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Get raw key bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }
}

/// A digital signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// Raw signature bytes.
    pub bytes: Vec<u8>,
    /// ID of the key that created this signature.
    pub key_id: String,
    /// Algorithm used (always "Ed25519" for now).
    pub algorithm: String,
}

impl Signature {
    /// Get signature as hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(&self.bytes)
    }

    /// Create from hex string.
    pub fn from_hex(hex_str: &str, key_id: impl Into<String>) -> Result<Self> {
        let bytes =
            hex::decode(hex_str).map_err(|e| CryptoError::InvalidSignature)?;

        Ok(Self {
            bytes,
            key_id: key_id.into(),
            algorithm: "Ed25519".to_string(),
        })
    }
}

// Add hex dependency inline for this module
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> std::result::Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }

        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_verify() {
        let signing_key = SigningKey::generate().unwrap();
        let verifying_key = signing_key.verifying_key();

        let message = b"Hello, AtomicSettle!";
        let signature = signing_key.sign(message);

        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let signing_key = SigningKey::generate().unwrap();
        let verifying_key = signing_key.verifying_key();

        let message = b"Hello, AtomicSettle!";
        let mut signature = signing_key.sign(message);

        // Corrupt the signature
        signature.bytes[0] ^= 0xff;

        assert!(verifying_key.verify(message, &signature).is_err());
    }

    #[test]
    fn test_key_serialization() {
        let signing_key = SigningKey::generate().unwrap();
        let bytes = signing_key.to_bytes();

        let restored = SigningKey::from_bytes(&bytes).unwrap();
        assert_eq!(signing_key.key_id(), restored.key_id());
    }
}
