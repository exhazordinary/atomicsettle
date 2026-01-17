//! Protocol message types.
//!
//! These types represent the messages exchanged between participants
//! and the coordinator in the AtomicSettle protocol.

use atomicsettle_common::{Currency, Money, ParticipantId, SettlementId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Settlement request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleRequest {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Unique settlement ID.
    pub settlement_id: SettlementId,
    /// Idempotency key for duplicate detection.
    pub idempotency_key: String,
    /// Request timestamp.
    pub timestamp: DateTime<Utc>,
    /// Sender information.
    pub sender: ParticipantInfo,
    /// Receiver information.
    pub receiver: ParticipantInfo,
    /// Settlement amount.
    pub amount: Money,
    /// FX instruction.
    pub fx_instruction: Option<FxInstruction>,
    /// Compliance information.
    pub compliance: ComplianceInfo,
    /// Sender's digital signature.
    pub signature: Option<Vec<u8>>,
}

/// Settlement response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleResponse {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Settlement ID this responds to.
    pub settlement_id: SettlementId,
    /// Response status.
    pub status: SettlementStatus,
    /// Status reason (for failures).
    pub reason: Option<String>,
    /// Response timestamp.
    pub timestamp: DateTime<Utc>,
    /// Coordinator signature.
    pub signature: Option<Vec<u8>>,
}

/// Lock request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleLock {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Lock ID.
    pub lock_id: Uuid,
    /// Lock expiration time.
    pub expires_at: DateTime<Utc>,
    /// Accounts to lock.
    pub locked_accounts: Vec<LockedAccount>,
    /// Coordinator signature.
    pub signature: Option<Vec<u8>>,
}

/// Commit message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleCommit {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Lock ID being committed.
    pub lock_id: Uuid,
    /// Commit timestamp.
    pub timestamp: DateTime<Utc>,
    /// Coordinator signature.
    pub signature: Option<Vec<u8>>,
}

/// Confirmation message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleConfirm {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Confirmation timestamp.
    pub timestamp: DateTime<Utc>,
    /// Final settlement details.
    pub settlement: SettlementDetails,
    /// Coordinator signature.
    pub signature: Option<Vec<u8>>,
}

/// Abort message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettleAbort {
    /// Protocol version.
    pub version: String,
    /// Message type identifier.
    pub message_type: MessageType,
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Abort reason.
    pub reason: String,
    /// Abort timestamp.
    pub timestamp: DateTime<Utc>,
    /// Coordinator signature.
    pub signature: Option<Vec<u8>>,
}

/// Message type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MessageType {
    SettleRequest,
    SettleValidate,
    SettleLock,
    SettleCommit,
    SettleConfirm,
    SettleAbort,
}

/// Settlement status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementStatus {
    /// Settlement initiated.
    Initiated,
    /// Settlement validated.
    Validated,
    /// Funds locked.
    Locked,
    /// Settlement committed.
    Committed,
    /// Settlement completed successfully.
    Settled,
    /// Settlement rejected.
    Rejected,
    /// Settlement failed.
    Failed,
}

/// Participant information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantInfo {
    /// Participant ID.
    pub participant_id: ParticipantId,
    /// Account ID.
    pub account_id: String,
}

/// FX instruction for currency conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxInstruction {
    /// Where conversion should happen.
    pub conversion_point: ConversionPoint,
    /// Rate reference for locked rate.
    pub rate_reference: Option<String>,
}

/// Where FX conversion occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConversionPoint {
    /// Convert at source.
    AtSource,
    /// Convert at destination.
    AtDestination,
    /// Convert at coordinator.
    Coordinator,
}

/// Compliance information for regulatory purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceInfo {
    /// Purpose code (ISO 20022).
    pub purpose_code: String,
    /// Remittance information.
    pub remittance_info: String,
    /// Originator details (for Travel Rule).
    pub originator: Option<Originator>,
    /// Beneficiary details.
    pub beneficiary: Option<Beneficiary>,
}

/// Originator information for Travel Rule compliance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Originator {
    /// Full name.
    pub name: String,
    /// Account number.
    pub account_number: String,
    /// Address.
    pub address: Option<String>,
    /// National ID.
    pub national_id: Option<String>,
    /// Date of birth.
    pub date_of_birth: Option<String>,
    /// Place of birth.
    pub place_of_birth: Option<String>,
}

/// Beneficiary information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beneficiary {
    /// Full name.
    pub name: String,
    /// Account number.
    pub account_number: String,
    /// Address.
    pub address: Option<String>,
}

/// Locked account information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedAccount {
    /// Account ID.
    pub account_id: String,
    /// Locked amount.
    pub amount: Money,
}

/// Final settlement details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementDetails {
    /// Settlement ID.
    pub settlement_id: SettlementId,
    /// Sender participant.
    pub sender: ParticipantInfo,
    /// Receiver participant.
    pub receiver: ParticipantInfo,
    /// Amount sent.
    pub sent_amount: Money,
    /// Amount received (may differ due to FX).
    pub received_amount: Money,
    /// FX rate used (if applicable).
    pub fx_rate: Option<String>,
    /// Settlement timestamp.
    pub settled_at: DateTime<Utc>,
}

impl SettleRequest {
    /// Create a new settlement request.
    pub fn new(
        sender: ParticipantInfo,
        receiver: ParticipantInfo,
        amount: Money,
    ) -> Self {
        Self {
            version: "1.0".to_string(),
            message_type: MessageType::SettleRequest,
            settlement_id: SettlementId::new(),
            idempotency_key: Uuid::now_v7().to_string(),
            timestamp: Utc::now(),
            sender,
            receiver,
            amount,
            fx_instruction: None,
            compliance: ComplianceInfo {
                purpose_code: "OTHR".to_string(),
                remittance_info: String::new(),
                originator: None,
                beneficiary: None,
            },
            signature: None,
        }
    }
}
