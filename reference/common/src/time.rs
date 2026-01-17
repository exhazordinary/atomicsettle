//! Time utilities and constants for AtomicSettle protocol.

use chrono::{DateTime, Duration, Utc};

/// Protocol timing constants.
pub mod constants {
    use super::Duration;

    /// Default lock duration (30 seconds).
    pub fn default_lock_duration() -> Duration {
        Duration::seconds(30)
    }

    /// Maximum lock duration (60 seconds).
    pub fn max_lock_duration() -> Duration {
        Duration::seconds(60)
    }

    /// Minimum lock duration (5 seconds).
    pub fn min_lock_duration() -> Duration {
        Duration::seconds(5)
    }

    /// Lock acquisition timeout (10 seconds).
    pub fn lock_acquisition_timeout() -> Duration {
        Duration::seconds(10)
    }

    /// FX rate lock duration (30 seconds).
    pub fn fx_rate_lock_duration() -> Duration {
        Duration::seconds(30)
    }

    /// Participant heartbeat interval (5 seconds).
    pub fn heartbeat_interval() -> Duration {
        Duration::seconds(5)
    }

    /// Participant heartbeat timeout (15 seconds).
    pub fn heartbeat_timeout() -> Duration {
        Duration::seconds(15)
    }

    /// Settlement acknowledgment timeout (60 seconds).
    pub fn acknowledgment_timeout() -> Duration {
        Duration::seconds(60)
    }

    /// Message freshness window (5 minutes).
    pub fn message_freshness_window() -> Duration {
        Duration::minutes(5)
    }

    /// Netting window (100 milliseconds).
    pub fn netting_window() -> Duration {
        Duration::milliseconds(100)
    }

    /// Maximum clock skew tolerance (100 milliseconds).
    pub fn max_clock_skew() -> Duration {
        Duration::milliseconds(100)
    }
}

/// Performance targets.
pub mod targets {
    /// Target end-to-end latency p50 (1000ms).
    pub const LATENCY_P50_MS: i64 = 1000;

    /// Target end-to-end latency p99 (3000ms).
    pub const LATENCY_P99_MS: i64 = 3000;

    /// Target lock acquisition time (500ms).
    pub const LOCK_ACQUISITION_MS: i64 = 500;

    /// Target commit execution time (200ms).
    pub const COMMIT_EXECUTION_MS: i64 = 200;

    /// Target throughput (settlements per second).
    pub const TARGET_TPS: u64 = 10_000;
}

/// A timestamp with timezone (always UTC for AtomicSettle).
pub type Timestamp = DateTime<Utc>;

/// Get the current timestamp.
pub fn now() -> Timestamp {
    Utc::now()
}

/// Check if a timestamp is within the freshness window.
pub fn is_fresh(timestamp: Timestamp) -> bool {
    let diff = (now() - timestamp).abs();
    diff < constants::message_freshness_window()
}

/// Check if a timestamp has expired (is in the past).
pub fn is_expired(expiry: Timestamp) -> bool {
    now() > expiry
}

/// Calculate expiry time from now.
pub fn expires_in(duration: Duration) -> Timestamp {
    now() + duration
}

/// Duration extensions for convenient construction.
pub trait DurationExt {
    fn as_std(&self) -> std::time::Duration;
}

impl DurationExt for Duration {
    fn as_std(&self) -> std::time::Duration {
        self.to_std().unwrap_or(std::time::Duration::ZERO)
    }
}

/// Timeout wrapper for async operations.
#[derive(Debug, Clone)]
pub struct Timeout {
    /// Deadline for the operation.
    pub deadline: Timestamp,
    /// Operation description.
    pub operation: String,
}

impl Timeout {
    /// Create a new timeout.
    pub fn new(duration: Duration, operation: impl Into<String>) -> Self {
        Self {
            deadline: expires_in(duration),
            operation: operation.into(),
        }
    }

    /// Check if the timeout has been exceeded.
    pub fn is_exceeded(&self) -> bool {
        is_expired(self.deadline)
    }

    /// Get remaining duration.
    pub fn remaining(&self) -> Duration {
        let remaining = self.deadline - now();
        if remaining < Duration::zero() {
            Duration::zero()
        } else {
            remaining
        }
    }

    /// Get remaining as std::time::Duration.
    pub fn remaining_std(&self) -> std::time::Duration {
        self.remaining().as_std()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_fresh() {
        let recent = now() - Duration::seconds(10);
        assert!(is_fresh(recent));

        let old = now() - Duration::minutes(10);
        assert!(!is_fresh(old));
    }

    #[test]
    fn test_is_expired() {
        let past = now() - Duration::seconds(10);
        assert!(is_expired(past));

        let future = now() + Duration::seconds(10);
        assert!(!is_expired(future));
    }

    #[test]
    fn test_timeout() {
        let timeout = Timeout::new(Duration::seconds(10), "test");
        assert!(!timeout.is_exceeded());
        assert!(timeout.remaining() > Duration::zero());
    }
}
