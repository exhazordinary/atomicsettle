//! Simulation metrics.

use std::collections::VecDeque;

/// Simulation metrics.
#[derive(Debug, Clone)]
pub struct SimulationMetrics {
    /// Total settlements attempted.
    pub total_settlements: u64,
    /// Successful settlements.
    pub successful_settlements: u64,
    /// Failed settlements.
    pub failed_settlements: u64,
    /// Latency samples (ms).
    latency_samples: VecDeque<u64>,
    /// Maximum samples to keep.
    max_samples: usize,
}

impl SimulationMetrics {
    /// Create new metrics.
    pub fn new() -> Self {
        Self {
            total_settlements: 0,
            successful_settlements: 0,
            failed_settlements: 0,
            latency_samples: VecDeque::with_capacity(10000),
            max_samples: 10000,
        }
    }

    /// Record a successful settlement.
    pub fn record_success(&mut self, latency_ms: u64) {
        self.total_settlements += 1;
        self.successful_settlements += 1;

        if self.latency_samples.len() >= self.max_samples {
            self.latency_samples.pop_front();
        }
        self.latency_samples.push_back(latency_ms);
    }

    /// Record a failed settlement.
    pub fn record_failure(&mut self) {
        self.total_settlements += 1;
        self.failed_settlements += 1;
    }

    /// Get average latency in ms.
    pub fn average_latency_ms(&self) -> u64 {
        if self.latency_samples.is_empty() {
            return 0;
        }

        let sum: u64 = self.latency_samples.iter().sum();
        sum / self.latency_samples.len() as u64
    }

    /// Get p50 latency.
    #[allow(dead_code)]
    pub fn p50_latency_ms(&self) -> u64 {
        self.percentile_latency(50)
    }

    /// Get p99 latency.
    #[allow(dead_code)]
    pub fn p99_latency_ms(&self) -> u64 {
        self.percentile_latency(99)
    }

    /// Get percentile latency.
    #[allow(dead_code)]
    fn percentile_latency(&self, percentile: usize) -> u64 {
        if self.latency_samples.is_empty() {
            return 0;
        }

        let mut sorted: Vec<_> = self.latency_samples.iter().copied().collect();
        sorted.sort_unstable();

        let idx = (sorted.len() * percentile / 100).min(sorted.len() - 1);
        sorted[idx]
    }

    /// Get success rate.
    #[allow(dead_code)]
    pub fn success_rate(&self) -> f64 {
        if self.total_settlements == 0 {
            return 0.0;
        }

        self.successful_settlements as f64 / self.total_settlements as f64
    }

    /// Get throughput (settlements per second).
    #[allow(dead_code)]
    pub fn throughput(&self, duration_secs: u64) -> f64 {
        if duration_secs == 0 {
            return 0.0;
        }

        self.total_settlements as f64 / duration_secs as f64
    }
}

impl Default for SimulationMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let mut metrics = SimulationMetrics::new();

        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_success(150);
        metrics.record_failure();

        assert_eq!(metrics.total_settlements, 4);
        assert_eq!(metrics.successful_settlements, 3);
        assert_eq!(metrics.failed_settlements, 1);
        assert_eq!(metrics.average_latency_ms(), 150);
        assert_eq!(metrics.success_rate(), 0.75);
    }
}
