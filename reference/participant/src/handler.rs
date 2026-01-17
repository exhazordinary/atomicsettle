//! Message handling for incoming coordinator messages.

use atomicsettle_common::{LockId, Money, Result, Settlement, SettlementId, SettlementStatus};

/// Types of incoming messages from coordinator.
#[derive(Debug, Clone)]
pub enum IncomingMessage {
    /// Settlement notification.
    SettlementNotification {
        settlement_id: SettlementId,
        status: SettlementStatus,
        settlement: Option<Settlement>,
    },
    /// Lock request.
    LockRequest {
        lock_id: LockId,
        settlement_id: SettlementId,
        amount: Money,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    /// Lock release.
    LockRelease {
        lock_id: LockId,
        reason: String,
    },
    /// Heartbeat acknowledgment.
    HeartbeatAck {
        server_time: chrono::DateTime<chrono::Utc>,
    },
}

/// Trait for handling incoming settlements and messages.
#[async_trait::async_trait]
pub trait SettlementHandler: Send + Sync {
    /// Handle an incoming message.
    async fn handle_message(&self, message: IncomingMessage) -> Result<()>;
}

/// Default handler that logs messages but doesn't process them.
pub struct LoggingHandler;

#[async_trait::async_trait]
impl SettlementHandler for LoggingHandler {
    async fn handle_message(&self, message: IncomingMessage) -> Result<()> {
        match message {
            IncomingMessage::SettlementNotification {
                settlement_id,
                status,
                ..
            } => {
                tracing::info!(
                    settlement_id = %settlement_id,
                    status = ?status,
                    "Settlement notification received"
                );
            }
            IncomingMessage::LockRequest {
                lock_id,
                settlement_id,
                amount,
                ..
            } => {
                tracing::info!(
                    lock_id = %lock_id,
                    settlement_id = %settlement_id,
                    amount = %amount,
                    "Lock request received"
                );
            }
            IncomingMessage::LockRelease { lock_id, reason } => {
                tracing::info!(
                    lock_id = %lock_id,
                    reason = %reason,
                    "Lock release received"
                );
            }
            IncomingMessage::HeartbeatAck { server_time } => {
                tracing::debug!(server_time = %server_time, "Heartbeat acknowledged");
            }
        }
        Ok(())
    }
}

/// Handler that forwards messages to callback functions.
pub struct CallbackHandler {
    on_settlement: Option<Box<dyn Fn(Settlement) + Send + Sync>>,
    on_lock_request: Option<Box<dyn Fn(LockId, SettlementId, Money) + Send + Sync>>,
    on_lock_release: Option<Box<dyn Fn(LockId, String) + Send + Sync>>,
}

impl CallbackHandler {
    /// Create a new callback handler.
    pub fn new() -> Self {
        Self {
            on_settlement: None,
            on_lock_request: None,
            on_lock_release: None,
        }
    }

    /// Set settlement callback.
    pub fn on_settlement<F>(mut self, callback: F) -> Self
    where
        F: Fn(Settlement) + Send + Sync + 'static,
    {
        self.on_settlement = Some(Box::new(callback));
        self
    }

    /// Set lock request callback.
    pub fn on_lock_request<F>(mut self, callback: F) -> Self
    where
        F: Fn(LockId, SettlementId, Money) + Send + Sync + 'static,
    {
        self.on_lock_request = Some(Box::new(callback));
        self
    }

    /// Set lock release callback.
    pub fn on_lock_release<F>(mut self, callback: F) -> Self
    where
        F: Fn(LockId, String) + Send + Sync + 'static,
    {
        self.on_lock_release = Some(Box::new(callback));
        self
    }
}

impl Default for CallbackHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SettlementHandler for CallbackHandler {
    async fn handle_message(&self, message: IncomingMessage) -> Result<()> {
        match message {
            IncomingMessage::SettlementNotification {
                settlement: Some(settlement),
                ..
            } => {
                if let Some(callback) = &self.on_settlement {
                    callback(settlement);
                }
            }
            IncomingMessage::LockRequest {
                lock_id,
                settlement_id,
                amount,
                ..
            } => {
                if let Some(callback) = &self.on_lock_request {
                    callback(lock_id, settlement_id, amount);
                }
            }
            IncomingMessage::LockRelease { lock_id, reason } => {
                if let Some(callback) = &self.on_lock_release {
                    callback(lock_id, reason);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
