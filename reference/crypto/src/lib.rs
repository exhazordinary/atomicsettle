//! AtomicSettle Cryptographic Primitives
//!
//! Provides signing, verification, and encryption for protocol messages.

pub mod signing;
pub mod encryption;
pub mod hash;

pub use signing::{SigningKey, VerifyingKey, Signature};
pub use encryption::{encrypt, decrypt, EncryptedPayload};
pub use hash::{sha256, sha384};

/// Errors from cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;
