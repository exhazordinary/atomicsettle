//! Simulation scenarios.

use serde::{Deserialize, Serialize};

/// A simulation scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    /// Scenario name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Duration in seconds.
    pub duration_secs: u64,
    /// Steps in the scenario.
    pub steps: Vec<ScenarioStep>,
}

/// A step in a scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScenarioStep {
    /// Wait for a duration.
    Wait { seconds: u64 },
    /// Send a settlement.
    SendSettlement {
        from_bank: String,
        to_bank: String,
        amount: String,
        currency: String,
    },
    /// Inject a fault.
    InjectFault { fault_type: FaultType, target: String },
    /// Clear a fault.
    ClearFault { target: String },
    /// Assert a condition.
    Assert { condition: AssertCondition },
}

/// Types of faults that can be injected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FaultType {
    /// Bank goes offline.
    BankOffline,
    /// Network latency.
    NetworkLatency { delay_ms: u64 },
    /// Coordinator overload.
    CoordinatorOverload,
    /// Lock timeout.
    LockTimeout,
}

/// Conditions that can be asserted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssertCondition {
    /// Settlement succeeded.
    SettlementSucceeded { settlement_id: String },
    /// Settlement failed.
    SettlementFailed { settlement_id: String },
    /// Bank balance equals.
    BalanceEquals {
        bank: String,
        currency: String,
        amount: String,
    },
}

impl Scenario {
    /// Load a scenario by name.
    pub fn load(name: &str) -> anyhow::Result<Self> {
        match name {
            "simple-settlement" => Ok(Self::simple_settlement()),
            "multi-currency" => Ok(Self::multi_currency()),
            "high-volume" => Ok(Self::high_volume()),
            "failure-recovery" => Ok(Self::failure_recovery()),
            _ => Err(anyhow::anyhow!("Unknown scenario: {}", name)),
        }
    }

    /// Simple 2-party settlement scenario.
    fn simple_settlement() -> Self {
        Self {
            name: "simple-settlement".to_string(),
            description: "Basic 2-party settlement in USD".to_string(),
            duration_secs: 10,
            steps: vec![
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_A".to_string(),
                    to_bank: "BANK_B".to_string(),
                    amount: "1000000".to_string(),
                    currency: "USD".to_string(),
                },
                ScenarioStep::Wait { seconds: 5 },
            ],
        }
    }

    /// Multi-currency settlement scenario.
    fn multi_currency() -> Self {
        Self {
            name: "multi-currency".to_string(),
            description: "Cross-currency settlement with FX".to_string(),
            duration_secs: 30,
            steps: vec![
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_A".to_string(),
                    to_bank: "BANK_B".to_string(),
                    amount: "1000000".to_string(),
                    currency: "USD".to_string(),
                },
                ScenarioStep::Wait { seconds: 3 },
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_B".to_string(),
                    to_bank: "BANK_C".to_string(),
                    amount: "500000".to_string(),
                    currency: "EUR".to_string(),
                },
                ScenarioStep::Wait { seconds: 3 },
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_C".to_string(),
                    to_bank: "BANK_A".to_string(),
                    amount: "250000".to_string(),
                    currency: "GBP".to_string(),
                },
                ScenarioStep::Wait { seconds: 5 },
            ],
        }
    }

    /// High-volume stress test scenario.
    fn high_volume() -> Self {
        Self {
            name: "high-volume".to_string(),
            description: "Stress test with high settlement volume".to_string(),
            duration_secs: 60,
            steps: vec![
                // Generate many settlements
                ScenarioStep::Wait { seconds: 60 },
            ],
        }
    }

    /// Failure and recovery scenario.
    fn failure_recovery() -> Self {
        Self {
            name: "failure-recovery".to_string(),
            description: "Test failure handling and recovery".to_string(),
            duration_secs: 60,
            steps: vec![
                // Start a settlement
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_A".to_string(),
                    to_bank: "BANK_B".to_string(),
                    amount: "1000000".to_string(),
                    currency: "USD".to_string(),
                },
                ScenarioStep::Wait { seconds: 5 },
                // Take bank B offline
                ScenarioStep::InjectFault {
                    fault_type: FaultType::BankOffline,
                    target: "BANK_B".to_string(),
                },
                // Try another settlement (should fail)
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_A".to_string(),
                    to_bank: "BANK_B".to_string(),
                    amount: "500000".to_string(),
                    currency: "USD".to_string(),
                },
                ScenarioStep::Wait { seconds: 10 },
                // Bring bank B back online
                ScenarioStep::ClearFault {
                    target: "BANK_B".to_string(),
                },
                // Retry settlement (should succeed)
                ScenarioStep::SendSettlement {
                    from_bank: "BANK_A".to_string(),
                    to_bank: "BANK_B".to_string(),
                    amount: "500000".to_string(),
                    currency: "USD".to_string(),
                },
                ScenarioStep::Wait { seconds: 5 },
            ],
        }
    }
}
