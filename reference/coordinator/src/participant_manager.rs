//! Participant connection management.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{info, warn};

use atomicsettle_common::ParticipantId;

/// Participant connection state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantState {
    /// Participant is connected and active.
    Active,
    /// Participant is connected but not yet verified.
    Pending,
    /// Participant is disconnected.
    Disconnected,
    /// Participant is suspended (administrative action).
    Suspended,
}

/// Information about a connected participant.
#[derive(Debug, Clone)]
pub struct ParticipantInfo {
    /// Participant ID.
    pub id: ParticipantId,
    /// Current state.
    pub state: ParticipantState,
    /// When the participant connected.
    pub connected_at: Instant,
    /// Last heartbeat received.
    pub last_heartbeat: Instant,
    /// Certificate fingerprint.
    pub cert_fingerprint: Option<String>,
    /// Protocol version.
    pub protocol_version: String,
    /// Supported capabilities.
    pub capabilities: Vec<String>,
}

impl ParticipantInfo {
    /// Create new participant info.
    pub fn new(id: ParticipantId, protocol_version: String) -> Self {
        let now = Instant::now();
        Self {
            id,
            state: ParticipantState::Pending,
            connected_at: now,
            last_heartbeat: now,
            cert_fingerprint: None,
            protocol_version,
            capabilities: Vec::new(),
        }
    }

    /// Check if participant is active.
    pub fn is_active(&self) -> bool {
        self.state == ParticipantState::Active
    }

    /// Update last heartbeat time.
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    /// Get time since last heartbeat.
    pub fn time_since_heartbeat(&self) -> Duration {
        self.last_heartbeat.elapsed()
    }
}

/// Notification sent to participants.
#[derive(Debug, Clone)]
pub enum ParticipantNotification {
    /// Settlement notification.
    Settlement {
        settlement_id: atomicsettle_common::SettlementId,
        status: atomicsettle_common::SettlementStatus,
    },
    /// Lock request.
    LockRequest {
        lock_id: atomicsettle_common::LockId,
        settlement_id: atomicsettle_common::SettlementId,
        amount: atomicsettle_common::Money,
    },
    /// Lock release.
    LockRelease {
        lock_id: atomicsettle_common::LockId,
        reason: String,
    },
    /// Heartbeat.
    Heartbeat,
}

/// Channel for sending notifications to a participant.
pub type NotificationSender = mpsc::Sender<ParticipantNotification>;

/// Manager for participant connections.
pub struct ParticipantManager {
    /// Connected participants.
    participants: Arc<DashMap<ParticipantId, ParticipantInfo>>,
    /// Notification channels.
    notification_channels: Arc<DashMap<ParticipantId, NotificationSender>>,
    /// Heartbeat timeout.
    heartbeat_timeout: Duration,
}

impl ParticipantManager {
    /// Create a new participant manager.
    pub fn new() -> Self {
        Self {
            participants: Arc::new(DashMap::new()),
            notification_channels: Arc::new(DashMap::new()),
            heartbeat_timeout: Duration::from_secs(15),
        }
    }

    /// Register a new participant connection.
    pub fn register(
        &self,
        id: ParticipantId,
        protocol_version: String,
    ) -> mpsc::Receiver<ParticipantNotification> {
        let info = ParticipantInfo::new(id.clone(), protocol_version);
        self.participants.insert(id.clone(), info);

        // Create notification channel
        let (tx, rx) = mpsc::channel(100);
        self.notification_channels.insert(id.clone(), tx);

        info!(participant_id = %id, "Participant registered");
        rx
    }

    /// Mark participant as active after verification.
    pub fn activate(&self, id: &ParticipantId) -> bool {
        if let Some(mut info) = self.participants.get_mut(id) {
            info.state = ParticipantState::Active;
            info!(participant_id = %id, "Participant activated");
            return true;
        }
        false
    }

    /// Unregister a participant.
    pub fn unregister(&self, id: &ParticipantId) {
        self.participants.remove(id);
        self.notification_channels.remove(id);
        info!(participant_id = %id, "Participant unregistered");
    }

    /// Check if a participant is active.
    pub fn is_participant_active(&self, id: &ParticipantId) -> bool {
        self.participants
            .get(id)
            .map(|info| info.is_active())
            .unwrap_or(false)
    }

    /// Get participant info.
    pub fn get_participant(&self, id: &ParticipantId) -> Option<ParticipantInfo> {
        self.participants.get(id).map(|info| info.clone())
    }

    /// Update heartbeat for a participant.
    pub fn update_heartbeat(&self, id: &ParticipantId) -> bool {
        if let Some(mut info) = self.participants.get_mut(id) {
            info.update_heartbeat();
            return true;
        }
        false
    }

    /// Send notification to a participant.
    pub async fn notify(
        &self,
        id: &ParticipantId,
        notification: ParticipantNotification,
    ) -> Result<(), String> {
        if let Some(sender) = self.notification_channels.get(id) {
            sender
                .send(notification)
                .await
                .map_err(|e| format!("Failed to send notification: {}", e))?;
            Ok(())
        } else {
            Err(format!("Participant {} not found", id))
        }
    }

    /// Broadcast notification to all active participants.
    pub async fn broadcast(&self, notification: ParticipantNotification) {
        for entry in self.participants.iter() {
            if entry.value().is_active() {
                let _ = self.notify(entry.key(), notification.clone()).await;
            }
        }
    }

    /// Get count of active participants.
    pub fn active_count(&self) -> usize {
        self.participants
            .iter()
            .filter(|e| e.value().is_active())
            .count()
    }

    /// Get all participant IDs.
    pub fn get_all_participant_ids(&self) -> Vec<ParticipantId> {
        self.participants.iter().map(|e| e.key().clone()).collect()
    }

    /// Run heartbeat checker loop.
    pub async fn run_heartbeat_checker(&self) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let stale_participants: Vec<ParticipantId> = self
                .participants
                .iter()
                .filter(|e| {
                    e.value().is_active()
                        && e.value().time_since_heartbeat() > self.heartbeat_timeout
                })
                .map(|e| e.key().clone())
                .collect();

            for id in stale_participants {
                warn!(participant_id = %id, "Participant heartbeat timeout");
                if let Some(mut info) = self.participants.get_mut(&id) {
                    info.state = ParticipantState::Disconnected;
                }
            }
        }
    }
}

impl Default for ParticipantManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_participant_registration() {
        let manager = ParticipantManager::new();
        let id = ParticipantId::new("TEST_BANK");

        let _rx = manager.register(id.clone(), "1.0".to_string());

        assert!(manager.get_participant(&id).is_some());
        assert!(!manager.is_participant_active(&id)); // Not yet activated
    }

    #[tokio::test]
    async fn test_participant_activation() {
        let manager = ParticipantManager::new();
        let id = ParticipantId::new("TEST_BANK");

        let _rx = manager.register(id.clone(), "1.0".to_string());
        assert!(manager.activate(&id));
        assert!(manager.is_participant_active(&id));
    }

    #[tokio::test]
    async fn test_participant_unregister() {
        let manager = ParticipantManager::new();
        let id = ParticipantId::new("TEST_BANK");

        let _rx = manager.register(id.clone(), "1.0".to_string());
        manager.unregister(&id);
        assert!(manager.get_participant(&id).is_none());
    }
}
