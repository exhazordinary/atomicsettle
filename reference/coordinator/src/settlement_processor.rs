//! Settlement processing logic.

use std::sync::Arc;

use tracing::{info, warn, error, instrument};

use atomicsettle_common::{
    AtomicSettleError, FailureCode, Result, Settlement, SettlementFailure, SettlementId,
    SettlementStatus,
};

use crate::lock_manager::LockManager;
use crate::participant_manager::{ParticipantManager, ParticipantNotification};

/// Settlement processor handles the settlement lifecycle.
pub struct SettlementProcessor {
    /// Lock manager.
    #[allow(dead_code)]
    lock_manager: Arc<LockManager>,
    /// Participant manager.
    #[allow(dead_code)]
    participant_manager: Arc<ParticipantManager>,
}

impl SettlementProcessor {
    /// Create a new settlement processor.
    pub fn new(
        lock_manager: Arc<LockManager>,
        participant_manager: Arc<ParticipantManager>,
    ) -> Self {
        Self {
            lock_manager,
            participant_manager,
        }
    }

    /// Process a settlement through its lifecycle.
    #[instrument(skip(self), fields(settlement_id = %settlement_id))]
    pub async fn process(&self, settlement_id: SettlementId) -> Result<Settlement> {
        // This is a placeholder implementation showing the settlement flow
        // In a real implementation, this would interact with the database and participants

        info!(settlement_id = %settlement_id, "Processing settlement");

        // 1. Validate
        // 2. Acquire locks
        // 3. Commit
        // 4. Notify participants
        // 5. Wait for acknowledgments

        // For now, return a mock processed settlement
        Err(AtomicSettleError::InternalError(
            "Settlement processor not fully implemented".to_string(),
        ))
    }

    /// Validate a settlement.
    #[allow(dead_code)]
    async fn validate(&self, settlement: &mut Settlement) -> Result<()> {
        info!(
            settlement_id = %settlement.id,
            "Validating settlement"
        );

        // Check all participants are active
        for leg in &settlement.legs {
            if !self
                .participant_manager
                .is_participant_active(&leg.from_participant)
            {
                return Err(AtomicSettleError::ParticipantOffline(
                    leg.from_participant.clone(),
                ));
            }
            if !self
                .participant_manager
                .is_participant_active(&leg.to_participant)
            {
                return Err(AtomicSettleError::ParticipantOffline(
                    leg.to_participant.clone(),
                ));
            }
        }

        settlement.transition_to(SettlementStatus::Validated).map_err(|_| {
            AtomicSettleError::InvalidTransition {
                from: settlement.status,
                to: SettlementStatus::Validated,
            }
        })?;

        Ok(())
    }

    /// Acquire locks from all source participants.
    #[allow(dead_code)]
    async fn acquire_locks(&self, settlement: &mut Settlement) -> Result<()> {
        info!(
            settlement_id = %settlement.id,
            "Acquiring locks"
        );

        settlement.transition_to(SettlementStatus::Locking).map_err(|_| {
            AtomicSettleError::InvalidTransition {
                from: settlement.status,
                to: SettlementStatus::Locking,
            }
        })?;

        // Create locks for each leg
        for leg in &mut settlement.legs {
            let lock = self.lock_manager.create_lock(
                settlement.id,
                leg.from_participant.clone(),
                leg.amount.clone(),
            );
            leg.lock_id = Some(lock.id);

            // Send lock request to participant
            let notification = ParticipantNotification::LockRequest {
                lock_id: lock.id,
                settlement_id: settlement.id,
                amount: leg.amount.clone(),
            };

            if let Err(e) = self
                .participant_manager
                .notify(&leg.from_participant, notification)
                .await
            {
                warn!(
                    settlement_id = %settlement.id,
                    participant = %leg.from_participant,
                    error = %e,
                    "Failed to send lock request"
                );
                // Rollback locks acquired so far
                self.lock_manager.release_locks_for_settlement(&settlement.id);
                return Err(AtomicSettleError::LockFailed {
                    settlement_id: settlement.id,
                    reason: e,
                });
            }
        }

        // Wait for lock confirmations (with timeout)
        // In real implementation, this would use channels or async wait
        let timeout = tokio::time::Duration::from_secs(10);
        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            if self.lock_manager.are_all_locks_confirmed(&settlement.id) {
                settlement.transition_to(SettlementStatus::Locked).map_err(|_| {
                    AtomicSettleError::InvalidTransition {
                        from: settlement.status,
                        to: SettlementStatus::Locked,
                    }
                })?;
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Lock timeout
        warn!(settlement_id = %settlement.id, "Lock acquisition timeout");
        self.lock_manager.release_locks_for_settlement(&settlement.id);
        Err(AtomicSettleError::Timeout("Lock acquisition".to_string()))
    }

    /// Execute atomic commit.
    #[allow(dead_code)]
    async fn commit(&self, settlement: &mut Settlement) -> Result<()> {
        info!(
            settlement_id = %settlement.id,
            "Committing settlement"
        );

        settlement.transition_to(SettlementStatus::Committing).map_err(|_| {
            AtomicSettleError::InvalidTransition {
                from: settlement.status,
                to: SettlementStatus::Committing,
            }
        })?;

        // In real implementation:
        // 1. Start database transaction
        // 2. Verify all locks still valid
        // 3. Execute ledger transfers
        // 4. Mark locks as consumed
        // 5. Commit transaction

        // Mark locks as consumed
        self.lock_manager.consume_locks_for_settlement(&settlement.id);

        settlement.transition_to(SettlementStatus::Committed).map_err(|_| {
            AtomicSettleError::InvalidTransition {
                from: settlement.status,
                to: SettlementStatus::Committed,
            }
        })?;

        Ok(())
    }

    /// Notify participants of settlement completion.
    #[allow(dead_code)]
    async fn notify_completion(&self, settlement: &mut Settlement) -> Result<()> {
        info!(
            settlement_id = %settlement.id,
            "Notifying participants of completion"
        );

        // Notify all involved participants
        let notification = ParticipantNotification::Settlement {
            settlement_id: settlement.id,
            status: SettlementStatus::Committed,
        };

        for leg in &settlement.legs {
            let _ = self
                .participant_manager
                .notify(&leg.from_participant, notification.clone())
                .await;
            let _ = self
                .participant_manager
                .notify(&leg.to_participant, notification.clone())
                .await;
        }

        // Mark as settled (acknowledgment is fire-and-forget)
        settlement.transition_to(SettlementStatus::Settled).map_err(|_| {
            AtomicSettleError::InvalidTransition {
                from: settlement.status,
                to: SettlementStatus::Settled,
            }
        })?;

        Ok(())
    }

    /// Handle settlement failure.
    #[allow(dead_code)]
    async fn handle_failure(&self, settlement: &mut Settlement, error: &AtomicSettleError) {
        error!(
            settlement_id = %settlement.id,
            error = %error,
            "Settlement failed"
        );

        // Release any held locks
        self.lock_manager.release_locks_for_settlement(&settlement.id);

        // Create failure record
        let failure = SettlementFailure {
            code: error_to_failure_code(error),
            message: error.to_string(),
            failed_leg: None,
            failed_at: chrono::Utc::now(),
        };

        let _ = settlement.fail(failure);

        // Notify participants of failure
        let notification = ParticipantNotification::Settlement {
            settlement_id: settlement.id,
            status: SettlementStatus::Failed,
        };

        for leg in &settlement.legs {
            let _ = self
                .participant_manager
                .notify(&leg.from_participant, notification.clone())
                .await;
            let _ = self
                .participant_manager
                .notify(&leg.to_participant, notification.clone())
                .await;
        }
    }
}

/// Convert error to failure code.
#[allow(dead_code)]
fn error_to_failure_code(error: &AtomicSettleError) -> FailureCode {
    match error {
        AtomicSettleError::Timeout(_) => FailureCode::LockTimeout,
        AtomicSettleError::ParticipantOffline(_) => FailureCode::ParticipantUnavailable,
        AtomicSettleError::InsufficientFunds { .. } => FailureCode::InsufficientFunds,
        AtomicSettleError::ComplianceRejected { .. } => FailureCode::ComplianceRejected,
        AtomicSettleError::FxRateExpired => FailureCode::FxRateExpired,
        _ => FailureCode::CoordinatorError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LockConfig;

    fn create_test_processor() -> SettlementProcessor {
        let lock_manager = Arc::new(LockManager::new(LockConfig::default()));
        let participant_manager = Arc::new(ParticipantManager::new());
        SettlementProcessor::new(lock_manager, participant_manager)
    }

    #[tokio::test]
    async fn test_processor_creation() {
        let _processor = create_test_processor();
        // Just verify it creates successfully
        assert!(true);
    }
}
