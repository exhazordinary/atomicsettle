//! Core coordinator implementation.

use std::sync::Arc;

use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tracing::{info, warn, error, instrument};

use atomicsettle_common::{
    AtomicSettleError, ParticipantId, Result, Settlement, SettlementId, SettlementStatus,
};

use crate::config::CoordinatorConfig;
use crate::lock_manager::LockManager;
use crate::participant_manager::ParticipantManager;
use crate::settlement_processor::SettlementProcessor;
use crate::state::CoordinatorState;

/// Settlement request received from a participant.
#[derive(Debug, Clone)]
pub struct SettleRequest {
    /// Unique settlement ID.
    pub settlement_id: SettlementId,
    /// Idempotency key for duplicate detection.
    pub idempotency_key: String,
    /// Sending participant.
    pub sender: ParticipantId,
    /// Receiving participant.
    pub receiver: ParticipantId,
    /// Amount and currency.
    pub amount: atomicsettle_common::Money,
    /// Compliance data.
    pub compliance: Option<atomicsettle_common::ComplianceData>,
}

/// Settlement response returned to participants.
#[derive(Debug, Clone)]
pub enum SettleResponse {
    /// Settlement accepted and in progress.
    Accepted {
        settlement_id: SettlementId,
        status: SettlementStatus,
    },
    /// Settlement completed successfully.
    Success(Settlement),
    /// Settlement rejected.
    Rejected {
        settlement_id: SettlementId,
        reason: String,
    },
    /// Settlement failed after processing started.
    Failed {
        settlement_id: SettlementId,
        reason: String,
    },
}

/// The main coordinator that orchestrates settlements.
pub struct Coordinator {
    /// Configuration.
    config: CoordinatorConfig,
    /// Node ID for this coordinator instance.
    node_id: String,
    /// Current coordinator state.
    state: Arc<RwLock<CoordinatorState>>,
    /// Active settlements indexed by ID.
    settlements: Arc<DashMap<SettlementId, Settlement>>,
    /// Idempotency key to settlement ID mapping.
    idempotency_map: Arc<DashMap<String, SettlementId>>,
    /// Participant connection manager.
    participant_manager: Arc<ParticipantManager>,
    /// Lock manager for distributed locks.
    lock_manager: Arc<LockManager>,
    /// Settlement processor for business logic.
    settlement_processor: Arc<SettlementProcessor>,
    /// Shutdown signal sender.
    shutdown_tx: mpsc::Sender<()>,
    /// Shutdown signal receiver.
    shutdown_rx: Arc<RwLock<Option<mpsc::Receiver<()>>>>,
}

impl Coordinator {
    /// Create a new coordinator instance.
    pub fn new(config: CoordinatorConfig, node_id: String) -> Self {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let participant_manager = Arc::new(ParticipantManager::new());
        let lock_manager = Arc::new(LockManager::new(config.lock_config.clone()));
        let settlement_processor = Arc::new(SettlementProcessor::new(
            lock_manager.clone(),
            participant_manager.clone(),
        ));

        Self {
            config,
            node_id,
            state: Arc::new(RwLock::new(CoordinatorState::Starting)),
            settlements: Arc::new(DashMap::new()),
            idempotency_map: Arc::new(DashMap::new()),
            participant_manager,
            lock_manager,
            settlement_processor,
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(Some(shutdown_rx))),
        }
    }

    /// Start the coordinator.
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!(node_id = %self.node_id, "Starting coordinator");

        // Transition to running state
        *self.state.write() = CoordinatorState::Running;

        // Start background tasks
        self.start_background_tasks().await?;

        info!(node_id = %self.node_id, "Coordinator started successfully");
        Ok(())
    }

    /// Stop the coordinator gracefully.
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        info!(node_id = %self.node_id, "Stopping coordinator");

        *self.state.write() = CoordinatorState::ShuttingDown;

        // Signal shutdown to background tasks
        let _ = self.shutdown_tx.send(()).await;

        // Wait for pending settlements to complete or timeout
        self.drain_pending_settlements().await;

        *self.state.write() = CoordinatorState::Stopped;

        info!(node_id = %self.node_id, "Coordinator stopped");
        Ok(())
    }

    /// Handle an incoming settlement request.
    #[instrument(skip(self, request), fields(settlement_id = %request.settlement_id))]
    pub async fn handle_settlement(&self, request: SettleRequest) -> Result<SettleResponse> {
        // Check coordinator state
        if !self.is_accepting_requests() {
            return Err(AtomicSettleError::CoordinatorBusy {
                retry_after_ms: 1000,
            });
        }

        // Check idempotency
        if let Some(existing_id) = self.idempotency_map.get(&request.idempotency_key) {
            return self.get_existing_settlement_response(*existing_id);
        }

        // Validate request
        self.validate_request(&request)?;

        // Create settlement
        let settlement = self.create_settlement(&request)?;
        let settlement_id = settlement.id;

        // Store settlement and idempotency key
        self.settlements.insert(settlement_id, settlement.clone());
        self.idempotency_map
            .insert(request.idempotency_key.clone(), settlement_id);

        // Process settlement asynchronously
        let processor = self.settlement_processor.clone();
        let settlements = self.settlements.clone();

        tokio::spawn(async move {
            match processor.process(settlement_id).await {
                Ok(updated_settlement) => {
                    settlements.insert(settlement_id, updated_settlement);
                }
                Err(e) => {
                    error!(
                        settlement_id = %settlement_id,
                        error = %e,
                        "Settlement processing failed"
                    );
                }
            }
        });

        Ok(SettleResponse::Accepted {
            settlement_id,
            status: SettlementStatus::Initiated,
        })
    }

    /// Get the current status of a settlement.
    pub fn get_settlement_status(&self, settlement_id: SettlementId) -> Result<SettlementStatus> {
        self.settlements
            .get(&settlement_id)
            .map(|s| s.status)
            .ok_or(AtomicSettleError::SettlementNotFound(settlement_id))
    }

    /// Get a settlement by ID.
    pub fn get_settlement(&self, settlement_id: SettlementId) -> Result<Settlement> {
        self.settlements
            .get(&settlement_id)
            .map(|s| s.clone())
            .ok_or(AtomicSettleError::SettlementNotFound(settlement_id))
    }

    /// Check if the coordinator is accepting requests.
    pub fn is_accepting_requests(&self) -> bool {
        matches!(*self.state.read(), CoordinatorState::Running)
    }

    /// Get the current coordinator state.
    pub fn state(&self) -> CoordinatorState {
        *self.state.read()
    }

    /// Get the number of active settlements.
    pub fn active_settlement_count(&self) -> usize {
        self.settlements
            .iter()
            .filter(|s| s.status.is_in_progress())
            .count()
    }

    // --- Private methods ---

    fn validate_request(&self, request: &SettleRequest) -> Result<()> {
        // Validate sender exists
        if !self.participant_manager.is_participant_active(&request.sender) {
            return Err(AtomicSettleError::UnknownParticipant(request.sender.clone()));
        }

        // Validate receiver exists
        if !self.participant_manager.is_participant_active(&request.receiver) {
            return Err(AtomicSettleError::UnknownParticipant(request.receiver.clone()));
        }

        // Validate amount is positive
        if !request.amount.is_positive() {
            return Err(AtomicSettleError::InvalidMessage {
                message: "Amount must be positive".to_string(),
                field: Some("amount".to_string()),
            });
        }

        // Validate sender and receiver are different
        if request.sender == request.receiver {
            return Err(AtomicSettleError::InvalidMessage {
                message: "Sender and receiver must be different".to_string(),
                field: Some("receiver".to_string()),
            });
        }

        Ok(())
    }

    fn create_settlement(&self, request: &SettleRequest) -> Result<Settlement> {
        use atomicsettle_common::{AccountId, SettlementLeg};

        // Create a single leg settlement
        let leg = SettlementLeg::new(
            1,
            request.sender.clone(),
            AccountId::new(
                request.sender.clone(),
                "default",
                request.amount.currency.code(),
            ),
            request.receiver.clone(),
            AccountId::new(
                request.receiver.clone(),
                "default",
                request.amount.currency.code(),
            ),
            request.amount.clone(),
        );

        let mut settlement = Settlement::new(request.idempotency_key.clone(), vec![leg]);
        settlement.compliance = request.compliance.clone();

        Ok(settlement)
    }

    fn get_existing_settlement_response(&self, settlement_id: SettlementId) -> Result<SettleResponse> {
        let settlement = self
            .settlements
            .get(&settlement_id)
            .ok_or(AtomicSettleError::SettlementNotFound(settlement_id))?;

        match settlement.status {
            SettlementStatus::Settled => Ok(SettleResponse::Success(settlement.clone())),
            SettlementStatus::Rejected => Ok(SettleResponse::Rejected {
                settlement_id,
                reason: settlement
                    .failure
                    .as_ref()
                    .map(|f| f.message.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            }),
            SettlementStatus::Failed => Ok(SettleResponse::Failed {
                settlement_id,
                reason: settlement
                    .failure
                    .as_ref()
                    .map(|f| f.message.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
            }),
            _ => Ok(SettleResponse::Accepted {
                settlement_id,
                status: settlement.status,
            }),
        }
    }

    async fn start_background_tasks(&self) -> Result<()> {
        // Start lock cleanup task
        let lock_manager = self.lock_manager.clone();
        tokio::spawn(async move {
            lock_manager.run_cleanup_loop().await;
        });

        // Start heartbeat checker
        let participant_manager = self.participant_manager.clone();
        tokio::spawn(async move {
            participant_manager.run_heartbeat_checker().await;
        });

        Ok(())
    }

    async fn drain_pending_settlements(&self) {
        use tokio::time::{timeout, Duration};

        let drain_timeout = Duration::from_secs(30);

        let _ = timeout(drain_timeout, async {
            loop {
                let pending_count = self.active_settlement_count();
                if pending_count == 0 {
                    break;
                }
                info!(pending_count, "Waiting for pending settlements to complete");
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::{Currency, Money};
    use rust_decimal::Decimal;

    fn create_test_config() -> CoordinatorConfig {
        CoordinatorConfig::default()
    }

    fn create_test_request() -> SettleRequest {
        SettleRequest {
            settlement_id: SettlementId::new(),
            idempotency_key: "test-key-123".to_string(),
            sender: ParticipantId::new("BANK_A"),
            receiver: ParticipantId::new("BANK_B"),
            amount: Money::new(Decimal::from(1000), Currency::usd()),
            compliance: None,
        }
    }

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = create_test_config();
        let coordinator = Coordinator::new(config, "test-node-1".to_string());

        assert_eq!(coordinator.state(), CoordinatorState::Starting);
        assert_eq!(coordinator.active_settlement_count(), 0);
    }

    #[tokio::test]
    async fn test_coordinator_start_stop() {
        let config = create_test_config();
        let coordinator = Coordinator::new(config, "test-node-1".to_string());

        coordinator.start().await.unwrap();
        assert_eq!(coordinator.state(), CoordinatorState::Running);

        coordinator.stop().await.unwrap();
        assert_eq!(coordinator.state(), CoordinatorState::Stopped);
    }
}
