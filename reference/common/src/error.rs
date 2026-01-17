//! Error types for AtomicSettle protocol.

use crate::{ParticipantId, SettlementId, SettlementStatus};
use thiserror::Error;

/// Main error type for AtomicSettle operations.
#[derive(Error, Debug)]
pub enum AtomicSettleError {
    /// Invalid message format or content.
    #[error("Invalid message: {message}")]
    InvalidMessage {
        message: String,
        field: Option<String>,
    },

    /// Invalid cryptographic signature.
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Unknown participant.
    #[error("Unknown participant: {0}")]
    UnknownParticipant(ParticipantId),

    /// Participant is offline.
    #[error("Participant offline: {0}")]
    ParticipantOffline(ParticipantId),

    /// Rate limited.
    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    /// Coordinator is overloaded.
    #[error("Coordinator busy, retry after {retry_after_ms}ms")]
    CoordinatorBusy { retry_after_ms: u64 },

    /// Settlement not found.
    #[error("Settlement not found: {0}")]
    SettlementNotFound(SettlementId),

    /// Lock not found.
    #[error("Lock not found: {0}")]
    LockNotFound(String),

    /// Duplicate request (idempotency key already used).
    #[error("Duplicate request with idempotency key: {0}")]
    DuplicateRequest(String),

    /// Protocol version mismatch.
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },

    /// Internal coordinator error.
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Insufficient funds.
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: String, available: String },

    /// Lock acquisition failed.
    #[error("Lock failed for settlement {settlement_id}: {reason}")]
    LockFailed {
        settlement_id: SettlementId,
        reason: String,
    },

    /// Lock expired.
    #[error("Lock expired for settlement {0}")]
    LockExpired(SettlementId),

    /// Invalid state transition.
    #[error("Invalid transition from {from:?} to {to:?}")]
    InvalidTransition {
        from: SettlementStatus,
        to: SettlementStatus,
    },

    /// FX rate expired.
    #[error("FX rate expired")]
    FxRateExpired,

    /// Compliance rejection.
    #[error("Compliance rejected: {reason}")]
    ComplianceRejected { reason: String, check_type: String },

    /// Database error.
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Network error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Timeout.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Crypto error.
    #[error("Cryptographic error: {0}")]
    CryptoError(String),
}

impl AtomicSettleError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AtomicSettleError::RateLimited { .. }
                | AtomicSettleError::CoordinatorBusy { .. }
                | AtomicSettleError::ParticipantOffline(_)
                | AtomicSettleError::NetworkError(_)
                | AtomicSettleError::Timeout(_)
        )
    }

    /// Get suggested retry delay in milliseconds.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            AtomicSettleError::RateLimited { retry_after_ms } => Some(*retry_after_ms),
            AtomicSettleError::CoordinatorBusy { retry_after_ms } => Some(*retry_after_ms),
            AtomicSettleError::ParticipantOffline(_) => Some(1000),
            AtomicSettleError::NetworkError(_) => Some(500),
            AtomicSettleError::Timeout(_) => Some(1000),
            _ => None,
        }
    }

    /// Get error code for protocol messages.
    pub fn error_code(&self) -> &'static str {
        match self {
            AtomicSettleError::InvalidMessage { .. } => "INVALID_MESSAGE",
            AtomicSettleError::InvalidSignature(_) => "INVALID_SIGNATURE",
            AtomicSettleError::UnknownParticipant(_) => "UNKNOWN_PARTICIPANT",
            AtomicSettleError::ParticipantOffline(_) => "PARTICIPANT_OFFLINE",
            AtomicSettleError::RateLimited { .. } => "RATE_LIMITED",
            AtomicSettleError::CoordinatorBusy { .. } => "COORDINATOR_BUSY",
            AtomicSettleError::SettlementNotFound(_) => "SETTLEMENT_NOT_FOUND",
            AtomicSettleError::LockNotFound(_) => "LOCK_NOT_FOUND",
            AtomicSettleError::DuplicateRequest(_) => "DUPLICATE_REQUEST",
            AtomicSettleError::VersionMismatch { .. } => "VERSION_MISMATCH",
            AtomicSettleError::InternalError(_) => "INTERNAL_ERROR",
            AtomicSettleError::InsufficientFunds { .. } => "INSUFFICIENT_FUNDS",
            AtomicSettleError::LockFailed { .. } => "LOCK_FAILED",
            AtomicSettleError::LockExpired(_) => "LOCK_EXPIRED",
            AtomicSettleError::InvalidTransition { .. } => "INVALID_TRANSITION",
            AtomicSettleError::FxRateExpired => "FX_RATE_EXPIRED",
            AtomicSettleError::ComplianceRejected { .. } => "COMPLIANCE_REJECTED",
            AtomicSettleError::DatabaseError(_) => "DATABASE_ERROR",
            AtomicSettleError::NetworkError(_) => "NETWORK_ERROR",
            AtomicSettleError::Timeout(_) => "TIMEOUT",
            AtomicSettleError::ConfigurationError(_) => "CONFIGURATION_ERROR",
            AtomicSettleError::CryptoError(_) => "CRYPTO_ERROR",
        }
    }
}

/// Result type alias for AtomicSettle operations.
pub type Result<T> = std::result::Result<T, AtomicSettleError>;

/// Rejection reasons for settlement requests.
#[derive(Debug, Clone)]
pub struct RejectionReason {
    /// Error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Field that caused rejection (if applicable).
    pub field: Option<String>,
}

impl RejectionReason {
    /// Create a new rejection reason.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
        }
    }

    /// Create with field.
    pub fn with_field(
        code: impl Into<String>,
        message: impl Into<String>,
        field: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: Some(field.into()),
        }
    }
}
