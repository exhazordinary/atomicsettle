//! Participant client for connecting to AtomicSettle network.

use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, instrument};

use atomicsettle_common::{
    AtomicSettleError, Balance, Currency, Money, ParticipantId, Result, Settlement,
    SettlementId, SettlementStatus,
};

use crate::config::ParticipantConfig;
use crate::connection::CoordinatorConnection;
use crate::handler::SettlementHandler;

/// Request to send a settlement.
#[derive(Debug, Clone)]
pub struct SettlementRequest {
    /// Destination participant.
    pub to_participant: ParticipantId,
    /// Amount to send.
    pub amount: Money,
    /// Purpose code (ISO 20022).
    pub purpose: String,
    /// Remittance information.
    pub remittance_info: Option<String>,
    /// Idempotency key (optional, generated if not provided).
    pub idempotency_key: Option<String>,
}

/// Client state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    /// Not connected.
    Disconnected,
    /// Connecting to coordinator.
    Connecting,
    /// Connected and operational.
    Connected,
    /// Reconnecting after disconnect.
    Reconnecting,
}

/// Participant client for banks to interact with AtomicSettle.
pub struct ParticipantClient {
    /// Configuration.
    config: ParticipantConfig,
    /// Participant ID.
    participant_id: ParticipantId,
    /// Current state.
    state: Arc<RwLock<ClientState>>,
    /// Connection to coordinator.
    connection: Arc<RwLock<Option<CoordinatorConnection>>>,
    /// Settlement handler for incoming settlements.
    settlement_handler: Arc<dyn SettlementHandler>,
    /// Shutdown signal sender.
    shutdown_tx: mpsc::Sender<()>,
}

impl ParticipantClient {
    /// Create a new participant client.
    pub fn new(
        config: ParticipantConfig,
        participant_id: ParticipantId,
        settlement_handler: Arc<dyn SettlementHandler>,
    ) -> Self {
        let (shutdown_tx, _) = mpsc::channel(1);

        Self {
            config,
            participant_id,
            state: Arc::new(RwLock::new(ClientState::Disconnected)),
            connection: Arc::new(RwLock::new(None)),
            settlement_handler,
            shutdown_tx,
        }
    }

    /// Connect to the coordinator.
    #[instrument(skip(self))]
    pub async fn connect(&self) -> Result<()> {
        info!(participant_id = %self.participant_id, "Connecting to coordinator");

        *self.state.write().await = ClientState::Connecting;

        // Create connection
        let connection = CoordinatorConnection::new(
            self.config.coordinator_url.clone(),
            self.participant_id.clone(),
            self.config.protocol_version.clone(),
        )
        .await?;

        *self.connection.write().await = Some(connection);
        *self.state.write().await = ClientState::Connected;

        info!(
            participant_id = %self.participant_id,
            coordinator_url = %self.config.coordinator_url,
            "Connected to coordinator"
        );

        // Start heartbeat loop
        self.start_heartbeat_loop();

        // Start incoming message handler
        self.start_message_handler();

        Ok(())
    }

    /// Disconnect from the coordinator.
    #[instrument(skip(self))]
    pub async fn disconnect(&self) -> Result<()> {
        info!(participant_id = %self.participant_id, "Disconnecting");

        let _ = self.shutdown_tx.send(()).await;

        if let Some(connection) = self.connection.write().await.take() {
            connection.close().await?;
        }

        *self.state.write().await = ClientState::Disconnected;

        info!(participant_id = %self.participant_id, "Disconnected");
        Ok(())
    }

    /// Send a settlement to another participant.
    #[instrument(skip(self, request))]
    pub async fn send_settlement(&self, request: SettlementRequest) -> Result<Settlement> {
        // Verify connected
        if *self.state.read().await != ClientState::Connected {
            return Err(AtomicSettleError::NetworkError(
                "Not connected to coordinator".to_string(),
            ));
        }

        let idempotency_key = request
            .idempotency_key
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        info!(
            participant_id = %self.participant_id,
            to_participant = %request.to_participant,
            amount = %request.amount,
            idempotency_key = %idempotency_key,
            "Sending settlement request"
        );

        // Get connection
        let connection_guard = self.connection.read().await;
        let connection = connection_guard
            .as_ref()
            .ok_or(AtomicSettleError::NetworkError("No connection".to_string()))?;

        // Send request via connection
        let settlement = connection
            .send_settlement_request(
                request.to_participant,
                request.amount,
                request.purpose,
                request.remittance_info,
                idempotency_key,
            )
            .await?;

        info!(
            settlement_id = %settlement.id,
            status = ?settlement.status,
            "Settlement initiated"
        );

        Ok(settlement)
    }

    /// Get current balance for a currency.
    pub async fn get_balance(&self, currency: Currency) -> Result<Balance> {
        // Verify connected
        if *self.state.read().await != ClientState::Connected {
            return Err(AtomicSettleError::NetworkError(
                "Not connected to coordinator".to_string(),
            ));
        }

        let connection_guard = self.connection.read().await;
        let connection = connection_guard
            .as_ref()
            .ok_or(AtomicSettleError::NetworkError("No connection".to_string()))?;

        connection.query_balance(currency).await
    }

    /// Get settlement by ID.
    pub async fn get_settlement(&self, settlement_id: SettlementId) -> Result<Settlement> {
        // Verify connected
        if *self.state.read().await != ClientState::Connected {
            return Err(AtomicSettleError::NetworkError(
                "Not connected to coordinator".to_string(),
            ));
        }

        let connection_guard = self.connection.read().await;
        let connection = connection_guard
            .as_ref()
            .ok_or(AtomicSettleError::NetworkError("No connection".to_string()))?;

        connection.get_settlement(settlement_id).await
    }

    /// Register callback for incoming settlements.
    pub fn on_incoming<F>(&self, callback: F)
    where
        F: Fn(Settlement) + Send + Sync + 'static,
    {
        // This would register the callback with the message handler
        // Simplified implementation
    }

    /// Get current client state.
    pub async fn state(&self) -> ClientState {
        *self.state.read().await
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ClientState::Connected
    }

    // --- Private methods ---

    fn start_heartbeat_loop(&self) {
        let connection = self.connection.clone();
        let state = self.state.clone();
        let interval = self.config.heartbeat_interval;

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                if *state.read().await != ClientState::Connected {
                    break;
                }

                if let Some(conn) = connection.read().await.as_ref() {
                    if let Err(e) = conn.send_heartbeat().await {
                        warn!(error = %e, "Heartbeat failed");
                    }
                }
            }
        });
    }

    fn start_message_handler(&self) {
        let connection = self.connection.clone();
        let state = self.state.clone();
        let handler = self.settlement_handler.clone();

        tokio::spawn(async move {
            loop {
                if *state.read().await != ClientState::Connected {
                    break;
                }

                if let Some(conn) = connection.read().await.as_ref() {
                    match conn.receive_message().await {
                        Ok(Some(message)) => {
                            // Handle incoming message
                            if let Err(e) = handler.handle_message(message).await {
                                warn!(error = %e, "Error handling message");
                            }
                        }
                        Ok(None) => {
                            // No message available
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        }
                        Err(e) => {
                            warn!(error = %e, "Error receiving message");
                            break;
                        }
                    }
                }
            }
        });
    }
}

/// Builder for ParticipantClient.
pub struct ParticipantClientBuilder {
    config: ParticipantConfig,
    participant_id: Option<ParticipantId>,
    settlement_handler: Option<Arc<dyn SettlementHandler>>,
}

impl ParticipantClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            config: ParticipantConfig::default(),
            participant_id: None,
            settlement_handler: None,
        }
    }

    /// Set participant ID.
    pub fn participant_id(mut self, id: ParticipantId) -> Self {
        self.participant_id = Some(id);
        self
    }

    /// Set coordinator URL.
    pub fn coordinator_url(mut self, url: impl Into<String>) -> Self {
        self.config.coordinator_url = url.into();
        self
    }

    /// Set settlement handler.
    pub fn settlement_handler(mut self, handler: Arc<dyn SettlementHandler>) -> Self {
        self.settlement_handler = Some(handler);
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<ParticipantClient> {
        let participant_id = self
            .participant_id
            .ok_or(AtomicSettleError::ConfigurationError(
                "Participant ID is required".to_string(),
            ))?;

        let settlement_handler = self
            .settlement_handler
            .ok_or(AtomicSettleError::ConfigurationError(
                "Settlement handler is required".to_string(),
            ))?;

        Ok(ParticipantClient::new(
            self.config,
            participant_id,
            settlement_handler,
        ))
    }
}

impl Default for ParticipantClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHandler;

    #[async_trait::async_trait]
    impl SettlementHandler for MockHandler {
        async fn handle_message(&self, _message: crate::handler::IncomingMessage) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        let handler = Arc::new(MockHandler);
        let client = ParticipantClient::new(
            ParticipantConfig::default(),
            ParticipantId::new("TEST_BANK"),
            handler,
        );

        assert_eq!(client.state().await, ClientState::Disconnected);
    }
}
