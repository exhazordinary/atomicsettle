//! Distributed lock management for settlements.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{info, warn};

use atomicsettle_common::{LockId, Money, ParticipantId, SettlementId};

use crate::config::LockConfig;

/// Lock status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStatus {
    /// Lock is active and holding funds.
    Active,
    /// Lock was consumed by successful settlement.
    Consumed,
    /// Lock was released due to failure or timeout.
    Released,
    /// Lock expired.
    Expired,
}

/// A lock on funds at a participant.
#[derive(Debug, Clone)]
pub struct Lock {
    /// Unique lock ID.
    pub id: LockId,
    /// Settlement this lock belongs to.
    pub settlement_id: SettlementId,
    /// Participant holding the lock.
    pub participant_id: ParticipantId,
    /// Amount locked.
    pub amount: Money,
    /// Current status.
    pub status: LockStatus,
    /// When the lock was created.
    pub created_at: Instant,
    /// When the lock expires.
    pub expires_at: Instant,
    /// When the lock was confirmed by participant.
    pub confirmed_at: Option<Instant>,
}

impl Lock {
    /// Create a new lock.
    pub fn new(
        settlement_id: SettlementId,
        participant_id: ParticipantId,
        amount: Money,
        duration: Duration,
    ) -> Self {
        let now = Instant::now();
        Self {
            id: LockId::new(),
            settlement_id,
            participant_id,
            amount,
            status: LockStatus::Active,
            created_at: now,
            expires_at: now + duration,
            confirmed_at: None,
        }
    }

    /// Check if lock is active.
    pub fn is_active(&self) -> bool {
        self.status == LockStatus::Active && !self.is_expired()
    }

    /// Check if lock has expired.
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }

    /// Get remaining time until expiry.
    pub fn remaining_time(&self) -> Duration {
        let now = Instant::now();
        if now > self.expires_at {
            Duration::ZERO
        } else {
            self.expires_at - now
        }
    }

    /// Mark lock as confirmed.
    pub fn confirm(&mut self) {
        self.confirmed_at = Some(Instant::now());
    }

    /// Mark lock as consumed.
    pub fn consume(&mut self) {
        self.status = LockStatus::Consumed;
    }

    /// Mark lock as released.
    pub fn release(&mut self) {
        self.status = LockStatus::Released;
    }

    /// Mark lock as expired.
    pub fn expire(&mut self) {
        self.status = LockStatus::Expired;
    }
}

/// Result of a lock acquisition attempt.
#[derive(Debug)]
pub enum LockResult {
    /// Lock acquired successfully.
    Acquired(Lock),
    /// Lock failed due to insufficient funds.
    InsufficientFunds { available: Money },
    /// Lock failed due to conflict with existing lock.
    Conflict { existing_lock_id: LockId },
    /// Lock failed due to participant being unavailable.
    ParticipantUnavailable,
    /// Lock failed due to other error.
    Error(String),
}

/// Manager for distributed locks.
pub struct LockManager {
    /// Active locks by ID.
    locks: Arc<DashMap<LockId, Lock>>,
    /// Locks by settlement ID.
    locks_by_settlement: Arc<DashMap<SettlementId, Vec<LockId>>>,
    /// Locks by participant.
    locks_by_participant: Arc<DashMap<ParticipantId, Vec<LockId>>>,
    /// Configuration.
    config: LockConfig,
}

impl LockManager {
    /// Create a new lock manager.
    pub fn new(config: LockConfig) -> Self {
        Self {
            locks: Arc::new(DashMap::new()),
            locks_by_settlement: Arc::new(DashMap::new()),
            locks_by_participant: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Create a new lock (does not send to participant).
    pub fn create_lock(
        &self,
        settlement_id: SettlementId,
        participant_id: ParticipantId,
        amount: Money,
    ) -> Lock {
        let lock = Lock::new(
            settlement_id,
            participant_id.clone(),
            amount,
            self.config.default_duration,
        );

        let lock_id = lock.id;

        // Store lock
        self.locks.insert(lock_id, lock.clone());

        // Index by settlement
        self.locks_by_settlement
            .entry(settlement_id)
            .or_insert_with(Vec::new)
            .push(lock_id);

        // Index by participant
        self.locks_by_participant
            .entry(participant_id)
            .or_insert_with(Vec::new)
            .push(lock_id);

        info!(
            lock_id = %lock_id,
            settlement_id = %settlement_id,
            "Lock created"
        );

        lock
    }

    /// Get a lock by ID.
    pub fn get_lock(&self, lock_id: &LockId) -> Option<Lock> {
        self.locks.get(lock_id).map(|l| l.clone())
    }

    /// Get all locks for a settlement.
    pub fn get_locks_for_settlement(&self, settlement_id: &SettlementId) -> Vec<Lock> {
        self.locks_by_settlement
            .get(settlement_id)
            .map(|lock_ids| {
                lock_ids
                    .iter()
                    .filter_map(|id| self.get_lock(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Confirm a lock (participant acknowledged).
    pub fn confirm_lock(&self, lock_id: &LockId) -> bool {
        if let Some(mut lock) = self.locks.get_mut(lock_id) {
            lock.confirm();
            info!(lock_id = %lock_id, "Lock confirmed");
            return true;
        }
        false
    }

    /// Consume a lock (settlement completed).
    pub fn consume_lock(&self, lock_id: &LockId) -> bool {
        if let Some(mut lock) = self.locks.get_mut(lock_id) {
            lock.consume();
            info!(lock_id = %lock_id, "Lock consumed");
            return true;
        }
        false
    }

    /// Release a lock (settlement failed or aborted).
    pub fn release_lock(&self, lock_id: &LockId) -> bool {
        if let Some(mut lock) = self.locks.get_mut(lock_id) {
            lock.release();
            info!(lock_id = %lock_id, "Lock released");
            return true;
        }
        false
    }

    /// Release all locks for a settlement.
    pub fn release_locks_for_settlement(&self, settlement_id: &SettlementId) {
        let locks = self.get_locks_for_settlement(settlement_id);
        for lock in locks {
            if lock.status == LockStatus::Active {
                self.release_lock(&lock.id);
            }
        }
    }

    /// Consume all locks for a settlement.
    pub fn consume_locks_for_settlement(&self, settlement_id: &SettlementId) {
        let locks = self.get_locks_for_settlement(settlement_id);
        for lock in locks {
            if lock.status == LockStatus::Active {
                self.consume_lock(&lock.id);
            }
        }
    }

    /// Check if all locks for a settlement are confirmed.
    pub fn are_all_locks_confirmed(&self, settlement_id: &SettlementId) -> bool {
        let locks = self.get_locks_for_settlement(settlement_id);
        !locks.is_empty() && locks.iter().all(|l| l.confirmed_at.is_some() && l.is_active())
    }

    /// Get count of active locks.
    pub fn active_lock_count(&self) -> usize {
        self.locks.iter().filter(|l| l.is_active()).count()
    }

    /// Get count of active locks for a participant.
    pub fn active_lock_count_for_participant(&self, participant_id: &ParticipantId) -> usize {
        self.locks_by_participant
            .get(participant_id)
            .map(|lock_ids| {
                lock_ids
                    .iter()
                    .filter_map(|id| self.get_lock(id))
                    .filter(|l| l.is_active())
                    .count()
            })
            .unwrap_or(0)
    }

    /// Run cleanup loop to expire stale locks.
    pub async fn run_cleanup_loop(&self) {
        loop {
            tokio::time::sleep(self.config.cleanup_interval).await;
            self.cleanup_expired_locks();
        }
    }

    /// Clean up expired locks.
    fn cleanup_expired_locks(&self) {
        let expired_locks: Vec<LockId> = self
            .locks
            .iter()
            .filter(|l| l.status == LockStatus::Active && l.is_expired())
            .map(|l| l.id)
            .collect();

        for lock_id in expired_locks {
            if let Some(mut lock) = self.locks.get_mut(&lock_id) {
                lock.expire();
                warn!(
                    lock_id = %lock_id,
                    settlement_id = %lock.settlement_id,
                    "Lock expired"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::Currency;
    use rust_decimal::Decimal;

    fn create_test_lock_manager() -> LockManager {
        LockManager::new(LockConfig::default())
    }

    #[test]
    fn test_lock_creation() {
        let manager = create_test_lock_manager();
        let settlement_id = SettlementId::new();
        let participant_id = ParticipantId::new("BANK_A");
        let amount = Money::new(Decimal::from(1000), Currency::usd());

        let lock = manager.create_lock(settlement_id, participant_id, amount);

        assert!(lock.is_active());
        assert!(!lock.is_expired());
    }

    #[test]
    fn test_lock_confirm() {
        let manager = create_test_lock_manager();
        let settlement_id = SettlementId::new();
        let participant_id = ParticipantId::new("BANK_A");
        let amount = Money::new(Decimal::from(1000), Currency::usd());

        let lock = manager.create_lock(settlement_id, participant_id, amount);
        assert!(manager.confirm_lock(&lock.id));

        let updated_lock = manager.get_lock(&lock.id).unwrap();
        assert!(updated_lock.confirmed_at.is_some());
    }

    #[test]
    fn test_lock_release() {
        let manager = create_test_lock_manager();
        let settlement_id = SettlementId::new();
        let participant_id = ParticipantId::new("BANK_A");
        let amount = Money::new(Decimal::from(1000), Currency::usd());

        let lock = manager.create_lock(settlement_id, participant_id, amount);
        assert!(manager.release_lock(&lock.id));

        let updated_lock = manager.get_lock(&lock.id).unwrap();
        assert_eq!(updated_lock.status, LockStatus::Released);
    }

    #[test]
    fn test_locks_by_settlement() {
        let manager = create_test_lock_manager();
        let settlement_id = SettlementId::new();
        let amount = Money::new(Decimal::from(1000), Currency::usd());

        manager.create_lock(
            settlement_id,
            ParticipantId::new("BANK_A"),
            amount.clone(),
        );
        manager.create_lock(settlement_id, ParticipantId::new("BANK_B"), amount);

        let locks = manager.get_locks_for_settlement(&settlement_id);
        assert_eq!(locks.len(), 2);
    }
}
