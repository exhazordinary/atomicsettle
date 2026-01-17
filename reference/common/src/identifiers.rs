//! Identifier types for AtomicSettle protocol entities.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a settlement.
/// Uses UUID v7 for time-ordered identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettlementId(Uuid);

impl SettlementId {
    /// Create a new settlement ID.
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for SettlementId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SettlementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a participant (bank or financial institution).
/// Typically follows ISO 9362 BIC format or an assigned identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantId(String);

impl ParticipantId {
    /// Create a new participant ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Validate the participant ID format.
    pub fn is_valid(&self) -> bool {
        // Basic validation: non-empty, alphanumeric with underscores
        !self.0.is_empty()
            && self.0.len() <= 64
            && self.0.chars().all(|c| c.is_alphanumeric() || c == '_')
    }
}

impl fmt::Display for ParticipantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ParticipantId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ParticipantId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Unique identifier for an account within a participant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId {
    /// The participant that owns this account.
    pub participant_id: ParticipantId,
    /// Account number within the participant.
    pub account_number: String,
    /// Currency of the account (ISO 4217).
    pub currency: String,
}

impl AccountId {
    /// Create a new account ID.
    pub fn new(
        participant_id: ParticipantId,
        account_number: impl Into<String>,
        currency: impl Into<String>,
    ) -> Self {
        Self {
            participant_id,
            account_number: account_number.into(),
            currency: currency.into(),
        }
    }

    /// Create a canonical string representation.
    pub fn canonical(&self) -> String {
        format!(
            "{}:{}:{}",
            self.participant_id, self.account_number, self.currency
        )
    }
}

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

/// Unique identifier for a lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockId(Uuid);

impl LockId {
    /// Create a new lock ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for LockId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    /// Create a new message ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a coordinator node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    /// Create a new node ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settlement_id_creation() {
        let id1 = SettlementId::new();
        let id2 = SettlementId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_settlement_id_parse() {
        let uuid_str = "019456ab-1234-7def-8901-234567890abc";
        let id = SettlementId::parse(uuid_str).unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_participant_id_validation() {
        assert!(ParticipantId::new("JPMORGAN_NY").is_valid());
        assert!(ParticipantId::new("BANK123").is_valid());
        assert!(!ParticipantId::new("").is_valid());
        assert!(!ParticipantId::new("bank-with-dash").is_valid());
    }

    #[test]
    fn test_account_id_canonical() {
        let account = AccountId::new(
            ParticipantId::new("JPMORGAN_NY"),
            "12345678",
            "USD",
        );
        assert_eq!(account.canonical(), "JPMORGAN_NY:12345678:USD");
    }
}
