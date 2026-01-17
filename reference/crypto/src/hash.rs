//! Cryptographic hash functions.

use sha2::{Digest, Sha256, Sha384};

/// Compute SHA-256 hash.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA-384 hash.
pub fn sha384(data: &[u8]) -> [u8; 48] {
    let mut hasher = Sha384::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA-256 hash and return as hex string.
pub fn sha256_hex(data: &[u8]) -> String {
    let hash = sha256(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Compute SHA-384 hash and return as hex string.
pub fn sha384_hex(data: &[u8]) -> String {
    let hash = sha384(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let hash = sha256(b"hello");
        let hex = sha256_hex(b"hello");

        assert_eq!(hash.len(), 32);
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_sha384() {
        let hash = sha384(b"hello");
        let hex = sha384_hex(b"hello");

        assert_eq!(hash.len(), 48);
        assert_eq!(hex.len(), 96);
    }

    #[test]
    fn test_empty_input() {
        let hash = sha256(b"");
        assert_eq!(hash.len(), 32);
    }
}
