//! Settlement types and state machine for AtomicSettle protocol.

use crate::{AccountId, FxRate, LockId, Money, ParticipantId, SettlementId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Settlement status representing the lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SettlementStatus {
    /// Settlement request received and validated.
    Initiated,
    /// All validation checks passed.
    Validated,
    /// Awaiting manual compliance review.
    PendingReview,
    /// Acquiring locks from participants.
    Locking,
    /// All locks acquired, awaiting commit.
    Locked,
    /// Executing atomic commit.
    Committing,
    /// Committed, awaiting acknowledgments.
    Committed,
    /// Complete, all parties acknowledged.
    Settled,
    /// Could not process request (before locking).
    Rejected,
    /// Failed after partial processing.
    Failed,
}

impl SettlementStatus {
    /// Check if this is a final state.
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            SettlementStatus::Settled | SettlementStatus::Rejected | SettlementStatus::Failed
        )
    }

    /// Check if settlement is in progress.
    pub fn is_in_progress(&self) -> bool {
        !self.is_final()
    }

    /// Get valid next states from current state.
    pub fn valid_transitions(&self) -> &[SettlementStatus] {
        match self {
            SettlementStatus::Initiated => &[
                SettlementStatus::Validated,
                SettlementStatus::PendingReview,
                SettlementStatus::Rejected,
            ],
            SettlementStatus::Validated => &[SettlementStatus::Locking, SettlementStatus::Rejected],
            SettlementStatus::PendingReview => {
                &[SettlementStatus::Validated, SettlementStatus::Rejected]
            }
            SettlementStatus::Locking => &[SettlementStatus::Locked, SettlementStatus::Failed],
            SettlementStatus::Locked => &[SettlementStatus::Committing, SettlementStatus::Failed],
            SettlementStatus::Committing => {
                &[SettlementStatus::Committed, SettlementStatus::Failed]
            }
            SettlementStatus::Committed => &[SettlementStatus::Settled],
            SettlementStatus::Settled => &[],
            SettlementStatus::Rejected => &[],
            SettlementStatus::Failed => &[],
        }
    }

    /// Check if transition to given state is valid.
    pub fn can_transition_to(&self, next: SettlementStatus) -> bool {
        self.valid_transitions().contains(&next)
    }
}

/// A single leg (directional transfer) within a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementLeg {
    /// Leg number within the settlement.
    pub leg_number: u32,
    /// Source participant.
    pub from_participant: ParticipantId,
    /// Source account.
    pub from_account: AccountId,
    /// Destination participant.
    pub to_participant: ParticipantId,
    /// Destination account.
    pub to_account: AccountId,
    /// Amount to transfer.
    pub amount: Money,
    /// FX instruction for this leg (if cross-currency).
    pub fx_instruction: Option<FxInstruction>,
    /// Lock ID for this leg (set after lock acquisition).
    pub lock_id: Option<LockId>,
    /// Converted amount (after FX, if applicable).
    pub converted_amount: Option<Money>,
}

impl SettlementLeg {
    /// Create a new settlement leg.
    pub fn new(
        leg_number: u32,
        from_participant: ParticipantId,
        from_account: AccountId,
        to_participant: ParticipantId,
        to_account: AccountId,
        amount: Money,
    ) -> Self {
        Self {
            leg_number,
            from_participant,
            from_account,
            to_participant,
            to_account,
            amount,
            fx_instruction: None,
            lock_id: None,
            converted_amount: None,
        }
    }

    /// Check if this is a cross-currency leg.
    pub fn is_cross_currency(&self) -> bool {
        self.from_account.currency != self.to_account.currency
    }
}

/// FX instruction specifying how currency conversion should be handled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxInstruction {
    /// Mode of FX execution.
    pub mode: FxMode,
    /// Target currency (for AT_COORDINATOR mode).
    pub target_currency: Option<String>,
    /// Pre-locked rate (optional).
    pub locked_rate: Option<FxRate>,
    /// Rate reference for lookup.
    pub rate_reference: Option<String>,
}

/// FX execution mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FxMode {
    /// Sender performs conversion.
    AtSource,
    /// Receiver performs conversion.
    AtDestination,
    /// Coordinator performs conversion.
    AtCoordinator,
}

/// Compliance data attached to a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceData {
    /// ISO 20022 purpose code.
    pub purpose_code: String,
    /// Payment reference / remittance info.
    pub remittance_info: Option<String>,
    /// Debtor information.
    pub debtor: Option<PartyInfo>,
    /// Creditor information.
    pub creditor: Option<PartyInfo>,
    /// Regulatory reporting data (jurisdiction-specific).
    pub regulatory_reporting: Option<String>,
}

/// Party information (debtor or creditor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyInfo {
    /// Party name.
    pub name: String,
    /// Identifier (Tax ID, LEI, etc.).
    pub identifier: Option<String>,
    /// Type of identifier.
    pub identifier_type: Option<String>,
    /// Address.
    pub address: Option<Address>,
}

/// Physical address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub street: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    /// ISO 3166-1 alpha-2 country code.
    pub country: String,
}

/// A complete settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    /// Unique settlement identifier.
    pub id: SettlementId,
    /// Idempotency key for duplicate detection.
    pub idempotency_key: String,
    /// Current status.
    pub status: SettlementStatus,
    /// Settlement legs.
    pub legs: Vec<SettlementLeg>,
    /// Compliance data.
    pub compliance: Option<ComplianceData>,
    /// FX details (if cross-currency).
    pub fx_details: Option<FxDetails>,
    /// Timing metrics.
    pub timing: SettlementTiming,
    /// Metadata.
    pub metadata: std::collections::HashMap<String, String>,
    /// Failure information (if failed).
    pub failure: Option<SettlementFailure>,
}

impl Settlement {
    /// Create a new settlement.
    pub fn new(idempotency_key: String, legs: Vec<SettlementLeg>) -> Self {
        Self {
            id: SettlementId::new(),
            idempotency_key,
            status: SettlementStatus::Initiated,
            legs,
            compliance: None,
            fx_details: None,
            timing: SettlementTiming::new(),
            metadata: std::collections::HashMap::new(),
            failure: None,
        }
    }

    /// Transition to a new status.
    pub fn transition_to(&mut self, new_status: SettlementStatus) -> Result<(), InvalidTransition> {
        if !self.status.can_transition_to(new_status) {
            return Err(InvalidTransition {
                from: self.status,
                to: new_status,
            });
        }

        self.status = new_status;

        // Update timing based on status
        let now = Utc::now();
        match new_status {
            SettlementStatus::Validated => self.timing.validated_at = Some(now),
            SettlementStatus::Locked => self.timing.locked_at = Some(now),
            SettlementStatus::Committed => self.timing.committed_at = Some(now),
            SettlementStatus::Settled => self.timing.settled_at = Some(now),
            _ => {}
        }

        Ok(())
    }

    /// Mark settlement as failed.
    pub fn fail(&mut self, failure: SettlementFailure) -> Result<(), InvalidTransition> {
        if self.status.is_final() {
            return Err(InvalidTransition {
                from: self.status,
                to: SettlementStatus::Failed,
            });
        }

        self.failure = Some(failure);
        self.status = SettlementStatus::Failed;
        self.timing.failed_at = Some(Utc::now());
        Ok(())
    }

    /// Get the total amount of this settlement (in source currency).
    pub fn total_amount(&self) -> Option<Money> {
        if self.legs.is_empty() {
            return None;
        }

        // Sum all legs in the same currency
        let first_currency = &self.legs[0].amount.currency;
        let mut total = self.legs[0].amount.value;

        for leg in &self.legs[1..] {
            if &leg.amount.currency != first_currency {
                // Mixed currencies, can't sum directly
                return None;
            }
            total += leg.amount.value;
        }

        Some(Money::new(total, first_currency.clone()))
    }

    /// Check if this is a cross-currency settlement.
    pub fn is_cross_currency(&self) -> bool {
        self.legs.iter().any(|leg| leg.is_cross_currency())
    }
}

/// Timing metrics for a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementTiming {
    /// When the settlement was initiated.
    pub initiated_at: DateTime<Utc>,
    /// When validation completed.
    pub validated_at: Option<DateTime<Utc>>,
    /// When all locks were acquired.
    pub locked_at: Option<DateTime<Utc>>,
    /// When the commit was executed.
    pub committed_at: Option<DateTime<Utc>>,
    /// When the settlement was finalized.
    pub settled_at: Option<DateTime<Utc>>,
    /// When the settlement failed (if applicable).
    pub failed_at: Option<DateTime<Utc>>,
}

impl SettlementTiming {
    /// Create new timing with current timestamp as initiation time.
    pub fn new() -> Self {
        Self {
            initiated_at: Utc::now(),
            validated_at: None,
            locked_at: None,
            committed_at: None,
            settled_at: None,
            failed_at: None,
        }
    }

    /// Get total duration in milliseconds (if completed).
    pub fn total_duration_ms(&self) -> Option<i64> {
        self.settled_at
            .map(|settled| (settled - self.initiated_at).num_milliseconds())
    }

    /// Get validation duration in milliseconds.
    pub fn validation_duration_ms(&self) -> Option<i64> {
        self.validated_at
            .map(|validated| (validated - self.initiated_at).num_milliseconds())
    }

    /// Get lock duration in milliseconds.
    pub fn lock_duration_ms(&self) -> Option<i64> {
        match (self.validated_at, self.locked_at) {
            (Some(validated), Some(locked)) => Some((locked - validated).num_milliseconds()),
            _ => None,
        }
    }

    /// Get commit duration in milliseconds.
    pub fn commit_duration_ms(&self) -> Option<i64> {
        match (self.locked_at, self.committed_at) {
            (Some(locked), Some(committed)) => Some((committed - locked).num_milliseconds()),
            _ => None,
        }
    }
}

impl Default for SettlementTiming {
    fn default() -> Self {
        Self::new()
    }
}

/// FX details for a cross-currency settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxDetails {
    /// Rate used for conversion.
    pub rate_used: FxRate,
    /// Original source amount.
    pub source_amount: Money,
    /// Converted amount.
    pub converted_amount: Money,
    /// Conversion reference.
    pub conversion_reference: String,
}

/// Settlement failure information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementFailure {
    /// Failure code.
    pub code: FailureCode,
    /// Human-readable message.
    pub message: String,
    /// Which leg failed (if applicable).
    pub failed_leg: Option<u32>,
    /// When the failure occurred.
    pub failed_at: DateTime<Utc>,
}

/// Failure codes for settlements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureCode {
    /// Lock acquisition timed out.
    LockTimeout,
    /// Participant became unavailable.
    ParticipantUnavailable,
    /// Coordinator internal error.
    CoordinatorError,
    /// Compliance check rejected.
    ComplianceRejected,
    /// FX rate expired before commit.
    FxRateExpired,
    /// Netting calculation failed.
    NettingFailure,
    /// Insufficient funds.
    InsufficientFunds,
    /// Invalid request.
    InvalidRequest,
}

/// Error when attempting invalid state transition.
#[derive(Debug, Clone)]
pub struct InvalidTransition {
    pub from: SettlementStatus,
    pub to: SettlementStatus,
}

impl std::fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Invalid state transition from {:?} to {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidTransition {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Currency;

    fn create_test_leg() -> SettlementLeg {
        SettlementLeg::new(
            1,
            ParticipantId::new("BANK_A"),
            AccountId::new(ParticipantId::new("BANK_A"), "12345", "USD"),
            ParticipantId::new("BANK_B"),
            AccountId::new(ParticipantId::new("BANK_B"), "67890", "USD"),
            Money::new(rust_decimal::Decimal::from(1000), Currency::usd()),
        )
    }

    #[test]
    fn test_settlement_creation() {
        let leg = create_test_leg();
        let settlement = Settlement::new("test-key".to_string(), vec![leg]);

        assert_eq!(settlement.status, SettlementStatus::Initiated);
        assert!(!settlement.is_cross_currency());
    }

    #[test]
    fn test_valid_transitions() {
        let leg = create_test_leg();
        let mut settlement = Settlement::new("test-key".to_string(), vec![leg]);

        assert!(settlement
            .transition_to(SettlementStatus::Validated)
            .is_ok());
        assert!(settlement.transition_to(SettlementStatus::Locking).is_ok());
        assert!(settlement.transition_to(SettlementStatus::Locked).is_ok());
        assert!(settlement
            .transition_to(SettlementStatus::Committing)
            .is_ok());
        assert!(settlement
            .transition_to(SettlementStatus::Committed)
            .is_ok());
        assert!(settlement.transition_to(SettlementStatus::Settled).is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        let leg = create_test_leg();
        let mut settlement = Settlement::new("test-key".to_string(), vec![leg]);

        // Can't go directly from Initiated to Locked
        assert!(settlement.transition_to(SettlementStatus::Locked).is_err());
    }

    #[test]
    fn test_final_states() {
        assert!(SettlementStatus::Settled.is_final());
        assert!(SettlementStatus::Rejected.is_final());
        assert!(SettlementStatus::Failed.is_final());
        assert!(!SettlementStatus::Initiated.is_final());
    }
}
