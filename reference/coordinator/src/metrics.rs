//! Metrics collection for coordinator monitoring.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Coordinator metrics.
pub struct Metrics {
    /// Total settlements processed.
    pub settlements_total: AtomicU64,
    /// Successful settlements.
    pub settlements_success: AtomicU64,
    /// Failed settlements.
    pub settlements_failed: AtomicU64,
    /// Rejected settlements.
    pub settlements_rejected: AtomicU64,
    /// Active settlements in progress.
    pub settlements_active: AtomicU64,
    /// Total locks acquired.
    pub locks_acquired: AtomicU64,
    /// Active locks.
    pub locks_active: AtomicU64,
    /// Lock timeouts.
    pub locks_timeout: AtomicU64,
    /// Active participant connections.
    pub participants_active: AtomicU64,
    /// Total messages received.
    pub messages_received: AtomicU64,
    /// Total messages sent.
    pub messages_sent: AtomicU64,
}

impl Metrics {
    /// Create new metrics instance.
    pub fn new() -> Self {
        Self {
            settlements_total: AtomicU64::new(0),
            settlements_success: AtomicU64::new(0),
            settlements_failed: AtomicU64::new(0),
            settlements_rejected: AtomicU64::new(0),
            settlements_active: AtomicU64::new(0),
            locks_acquired: AtomicU64::new(0),
            locks_active: AtomicU64::new(0),
            locks_timeout: AtomicU64::new(0),
            participants_active: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
            messages_sent: AtomicU64::new(0),
        }
    }

    /// Increment settlement initiated.
    pub fn settlement_initiated(&self) {
        self.settlements_total.fetch_add(1, Ordering::Relaxed);
        self.settlements_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record settlement success.
    pub fn settlement_success(&self) {
        self.settlements_success.fetch_add(1, Ordering::Relaxed);
        self.settlements_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record settlement failure.
    pub fn settlement_failed(&self) {
        self.settlements_failed.fetch_add(1, Ordering::Relaxed);
        self.settlements_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record settlement rejection.
    pub fn settlement_rejected(&self) {
        self.settlements_rejected.fetch_add(1, Ordering::Relaxed);
        self.settlements_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment lock acquired.
    pub fn lock_acquired(&self) {
        self.locks_acquired.fetch_add(1, Ordering::Relaxed);
        self.locks_active.fetch_add(1, Ordering::Relaxed);
    }

    /// Record lock released.
    pub fn lock_released(&self) {
        self.locks_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record lock timeout.
    pub fn lock_timeout(&self) {
        self.locks_timeout.fetch_add(1, Ordering::Relaxed);
        self.locks_active.fetch_sub(1, Ordering::Relaxed);
    }

    /// Set active participants count.
    pub fn set_participants_active(&self, count: u64) {
        self.participants_active.store(count, Ordering::Relaxed);
    }

    /// Increment messages received.
    pub fn message_received(&self) {
        self.messages_received.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment messages sent.
    pub fn message_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current metrics snapshot.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            settlements_total: self.settlements_total.load(Ordering::Relaxed),
            settlements_success: self.settlements_success.load(Ordering::Relaxed),
            settlements_failed: self.settlements_failed.load(Ordering::Relaxed),
            settlements_rejected: self.settlements_rejected.load(Ordering::Relaxed),
            settlements_active: self.settlements_active.load(Ordering::Relaxed),
            locks_acquired: self.locks_acquired.load(Ordering::Relaxed),
            locks_active: self.locks_active.load(Ordering::Relaxed),
            locks_timeout: self.locks_timeout.load(Ordering::Relaxed),
            participants_active: self.participants_active.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
        }
    }

    /// Export metrics in Prometheus format.
    pub fn to_prometheus(&self) -> String {
        let snapshot = self.snapshot();
        format!(
            r#"# HELP atomicsettle_settlements_total Total number of settlements
# TYPE atomicsettle_settlements_total counter
atomicsettle_settlements_total {}

# HELP atomicsettle_settlements_success Total successful settlements
# TYPE atomicsettle_settlements_success counter
atomicsettle_settlements_success {}

# HELP atomicsettle_settlements_failed Total failed settlements
# TYPE atomicsettle_settlements_failed counter
atomicsettle_settlements_failed {}

# HELP atomicsettle_settlements_rejected Total rejected settlements
# TYPE atomicsettle_settlements_rejected counter
atomicsettle_settlements_rejected {}

# HELP atomicsettle_settlements_active Current active settlements
# TYPE atomicsettle_settlements_active gauge
atomicsettle_settlements_active {}

# HELP atomicsettle_locks_acquired Total locks acquired
# TYPE atomicsettle_locks_acquired counter
atomicsettle_locks_acquired {}

# HELP atomicsettle_locks_active Current active locks
# TYPE atomicsettle_locks_active gauge
atomicsettle_locks_active {}

# HELP atomicsettle_locks_timeout Total lock timeouts
# TYPE atomicsettle_locks_timeout counter
atomicsettle_locks_timeout {}

# HELP atomicsettle_participants_active Current active participants
# TYPE atomicsettle_participants_active gauge
atomicsettle_participants_active {}

# HELP atomicsettle_messages_received Total messages received
# TYPE atomicsettle_messages_received counter
atomicsettle_messages_received {}

# HELP atomicsettle_messages_sent Total messages sent
# TYPE atomicsettle_messages_sent counter
atomicsettle_messages_sent {}
"#,
            snapshot.settlements_total,
            snapshot.settlements_success,
            snapshot.settlements_failed,
            snapshot.settlements_rejected,
            snapshot.settlements_active,
            snapshot.locks_acquired,
            snapshot.locks_active,
            snapshot.locks_timeout,
            snapshot.participants_active,
            snapshot.messages_received,
            snapshot.messages_sent,
        )
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of metrics at a point in time.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub settlements_total: u64,
    pub settlements_success: u64,
    pub settlements_failed: u64,
    pub settlements_rejected: u64,
    pub settlements_active: u64,
    pub locks_acquired: u64,
    pub locks_active: u64,
    pub locks_timeout: u64,
    pub participants_active: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
}

/// Shared metrics instance.
pub type SharedMetrics = Arc<Metrics>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_increment() {
        let metrics = Metrics::new();

        metrics.settlement_initiated();
        metrics.settlement_initiated();
        metrics.settlement_success();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.settlements_total, 2);
        assert_eq!(snapshot.settlements_success, 1);
        assert_eq!(snapshot.settlements_active, 1);
    }

    #[test]
    fn test_prometheus_export() {
        let metrics = Metrics::new();
        metrics.settlement_initiated();

        let output = metrics.to_prometheus();
        assert!(output.contains("atomicsettle_settlements_total 1"));
    }
}
