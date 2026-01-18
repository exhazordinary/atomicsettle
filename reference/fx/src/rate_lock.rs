//! Rate locking for guaranteed conversion rates.

use atomicsettle_common::FxRate;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

use crate::error::{FxError, FxResult};

/// A locked FX rate that can be used for a guaranteed conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLock {
    /// Unique lock ID.
    pub id: Uuid,
    /// The locked rate.
    pub rate: FxRate,
    /// When the lock was created.
    pub created_at: DateTime<Utc>,
    /// When the lock expires.
    pub expires_at: DateTime<Utc>,
    /// Participant who created the lock.
    pub participant_id: String,
    /// Whether the lock has been used.
    pub used: bool,
}

impl RateLock {
    /// Create a new rate lock.
    pub fn new(rate: FxRate, duration: Duration, participant_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::now_v7(),
            rate,
            created_at: now,
            expires_at: now + duration,
            participant_id,
            used: false,
        }
    }

    /// Check if the lock is still valid.
    pub fn is_valid(&self) -> bool {
        !self.used && Utc::now() < self.expires_at
    }

    /// Get remaining time until expiry.
    pub fn time_remaining(&self) -> Duration {
        let remaining = self.expires_at.signed_duration_since(Utc::now());
        if remaining < Duration::zero() {
            Duration::zero()
        } else {
            remaining
        }
    }

    /// Mark the lock as used.
    pub fn mark_used(&mut self) {
        self.used = true;
    }
}

/// Configuration for rate lock manager.
#[derive(Debug, Clone)]
pub struct RateLockConfig {
    /// Default lock duration.
    pub default_duration: Duration,
    /// Maximum lock duration.
    pub max_duration: Duration,
    /// Maximum locks per participant.
    pub max_locks_per_participant: usize,
}

impl Default for RateLockConfig {
    fn default() -> Self {
        Self {
            default_duration: Duration::seconds(30),
            max_duration: Duration::minutes(5),
            max_locks_per_participant: 100,
        }
    }
}

/// Manages rate locks.
pub struct RateLockManager {
    locks: DashMap<Uuid, RateLock>,
    participant_locks: DashMap<String, Vec<Uuid>>,
    config: RateLockConfig,
}

impl RateLockManager {
    /// Create a new rate lock manager.
    pub fn new() -> Self {
        Self::with_config(RateLockConfig::default())
    }

    /// Create with custom configuration.
    pub fn with_config(config: RateLockConfig) -> Self {
        Self {
            locks: DashMap::new(),
            participant_locks: DashMap::new(),
            config,
        }
    }

    /// Create a new rate lock.
    pub fn create_lock(
        &self,
        rate: FxRate,
        duration: Option<Duration>,
        participant_id: String,
    ) -> FxResult<RateLock> {
        // Check participant lock limit
        let participant_count = self
            .participant_locks
            .get(&participant_id)
            .map(|v| v.len())
            .unwrap_or(0);

        if participant_count >= self.config.max_locks_per_participant {
            return Err(FxError::InvalidRateLock(format!(
                "Participant {} has reached maximum locks",
                participant_id
            )));
        }

        // Determine lock duration
        let lock_duration = duration.unwrap_or(self.config.default_duration);
        let lock_duration = if lock_duration > self.config.max_duration {
            self.config.max_duration
        } else {
            lock_duration
        };

        // Create lock
        let lock = RateLock::new(rate, lock_duration, participant_id.clone());
        let lock_id = lock.id;

        debug!(
            lock_id = %lock_id,
            participant = %participant_id,
            expires_at = %lock.expires_at,
            "Created rate lock"
        );

        // Store lock
        self.locks.insert(lock_id, lock.clone());

        // Track by participant
        self.participant_locks
            .entry(participant_id)
            .or_default()
            .push(lock_id);

        Ok(lock)
    }

    /// Get a rate lock by ID.
    pub fn get_lock(&self, lock_id: Uuid) -> Option<RateLock> {
        self.locks.get(&lock_id).map(|r| r.clone())
    }

    /// Use a rate lock (marks it as used).
    pub fn use_lock(&self, lock_id: Uuid) -> FxResult<RateLock> {
        let mut lock = self
            .locks
            .get_mut(&lock_id)
            .ok_or_else(|| FxError::InvalidRateLock(lock_id.to_string()))?;

        if lock.used {
            return Err(FxError::InvalidRateLock(format!(
                "Lock {} already used",
                lock_id
            )));
        }

        if !lock.is_valid() {
            return Err(FxError::RateLockExpired(lock_id.to_string()));
        }

        lock.mark_used();
        debug!(lock_id = %lock_id, "Rate lock used");

        Ok(lock.clone())
    }

    /// Cancel a rate lock.
    pub fn cancel_lock(&self, lock_id: Uuid, participant_id: &str) -> FxResult<()> {
        let lock = self
            .locks
            .get(&lock_id)
            .ok_or_else(|| FxError::InvalidRateLock(lock_id.to_string()))?;

        if lock.participant_id != participant_id {
            return Err(FxError::InvalidRateLock(format!(
                "Lock {} does not belong to participant {}",
                lock_id, participant_id
            )));
        }

        drop(lock);
        self.locks.remove(&lock_id);

        // Remove from participant tracking
        if let Some(mut locks) = self.participant_locks.get_mut(participant_id) {
            locks.retain(|id| *id != lock_id);
        }

        debug!(lock_id = %lock_id, "Rate lock cancelled");
        Ok(())
    }

    /// Clean up expired locks.
    pub fn cleanup_expired(&self) {
        let expired: Vec<Uuid> = self
            .locks
            .iter()
            .filter(|entry| !entry.value().is_valid())
            .map(|entry| *entry.key())
            .collect();

        for lock_id in expired {
            if let Some((_, lock)) = self.locks.remove(&lock_id) {
                // Remove from participant tracking
                if let Some(mut locks) = self.participant_locks.get_mut(&lock.participant_id) {
                    locks.retain(|id| *id != lock_id);
                }
                debug!(lock_id = %lock_id, "Expired rate lock cleaned up");
            }
        }
    }

    /// Get all locks for a participant.
    pub fn get_participant_locks(&self, participant_id: &str) -> Vec<RateLock> {
        self.participant_locks
            .get(participant_id)
            .map(|lock_ids| {
                lock_ids
                    .iter()
                    .filter_map(|id| self.locks.get(id).map(|r| r.clone()))
                    .filter(|lock| lock.is_valid())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get statistics.
    pub fn stats(&self) -> RateLockStats {
        let total = self.locks.len();
        let valid = self.locks.iter().filter(|e| e.is_valid()).count();
        let used = self.locks.iter().filter(|e| e.used).count();

        RateLockStats {
            total_locks: total,
            valid_locks: valid,
            expired_locks: total - valid,
            used_locks: used,
        }
    }
}

impl Default for RateLockManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Rate lock statistics.
#[derive(Debug, Clone)]
pub struct RateLockStats {
    pub total_locks: usize,
    pub valid_locks: usize,
    pub expired_locks: usize,
    pub used_locks: usize,
}

/// Shared rate lock manager.
pub type SharedRateLockManager = Arc<RateLockManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::{Currency, CurrencyPair};
    use rust_decimal_macros::dec;

    fn make_test_rate() -> FxRate {
        FxRate::new(
            CurrencyPair::new(Currency::usd(), Currency::eur()),
            dec!(0.91),
            dec!(0.93),
            30,
            "TEST",
        )
    }

    #[test]
    fn test_create_lock() {
        let manager = RateLockManager::new();
        let rate = make_test_rate();

        let lock = manager
            .create_lock(rate.clone(), None, "BANK_A".to_string())
            .unwrap();

        assert!(lock.is_valid());
        assert!(!lock.used);
        assert_eq!(lock.participant_id, "BANK_A");
    }

    #[test]
    fn test_use_lock() {
        let manager = RateLockManager::new();
        let rate = make_test_rate();

        let lock = manager
            .create_lock(rate, None, "BANK_A".to_string())
            .unwrap();

        let used_lock = manager.use_lock(lock.id).unwrap();
        assert!(used_lock.used);

        // Can't use again
        assert!(manager.use_lock(lock.id).is_err());
    }

    #[test]
    fn test_cancel_lock() {
        let manager = RateLockManager::new();
        let rate = make_test_rate();

        let lock = manager
            .create_lock(rate, None, "BANK_A".to_string())
            .unwrap();

        // Can't cancel with wrong participant
        assert!(manager.cancel_lock(lock.id, "BANK_B").is_err());

        // Can cancel with correct participant
        manager.cancel_lock(lock.id, "BANK_A").unwrap();

        // Lock no longer exists
        assert!(manager.get_lock(lock.id).is_none());
    }

    #[test]
    fn test_participant_lock_limit() {
        let config = RateLockConfig {
            max_locks_per_participant: 2,
            ..Default::default()
        };
        let manager = RateLockManager::with_config(config);

        // Create max locks
        for _ in 0..2 {
            manager
                .create_lock(make_test_rate(), None, "BANK_A".to_string())
                .unwrap();
        }

        // Third should fail
        assert!(manager
            .create_lock(make_test_rate(), None, "BANK_A".to_string())
            .is_err());

        // Different participant should work
        assert!(manager
            .create_lock(make_test_rate(), None, "BANK_B".to_string())
            .is_ok());
    }
}
