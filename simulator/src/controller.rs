//! Simulation controller.

use std::sync::Arc;
use std::time::Duration;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rust_decimal::Decimal;
use tokio::sync::RwLock;
use tracing::{info, warn};

use atomicsettle_common::{Currency, Money, ParticipantId, SettlementId};

use crate::bank::{BankFactory, SimulatedBank};
use crate::metrics::SimulationMetrics;
use crate::scenario::{Scenario, ScenarioStep};

/// Controls the simulation.
pub struct SimulationController {
    /// Number of banks.
    bank_count: usize,
    /// Simulation speed multiplier.
    speed: f64,
    /// Random number generator.
    rng: Arc<RwLock<StdRng>>,
    /// Simulated banks.
    banks: Arc<RwLock<Vec<SimulatedBank>>>,
    /// Simulation metrics.
    metrics: Arc<RwLock<SimulationMetrics>>,
    /// Running flag.
    running: Arc<RwLock<bool>>,
}

impl SimulationController {
    /// Create a new simulation controller.
    pub fn new(bank_count: usize, speed: f64, seed: Option<u64>) -> Self {
        let rng = match seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };

        Self {
            bank_count,
            speed,
            rng: Arc::new(RwLock::new(rng)),
            banks: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(SimulationMetrics::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the simulation.
    pub async fn initialize(&mut self) -> anyhow::Result<()> {
        info!("Initializing simulation with {} banks", self.bank_count);

        // Create banks
        let banks = BankFactory::create_banks(self.bank_count);

        // Initialize balances
        let initial_balance = Decimal::from(100_000_000); // $100M
        for bank in &banks {
            bank.initialize_balances(initial_balance).await;
            info!("Initialized bank {} with ${} balance", bank.id, initial_balance);
        }

        *self.banks.write().await = banks;

        Ok(())
    }

    /// Run a scenario.
    pub async fn run_scenario(&self, scenario: Scenario) -> anyhow::Result<()> {
        info!("Running scenario: {} - {}", scenario.name, scenario.description);

        *self.running.write().await = true;

        for step in &scenario.steps {
            if !*self.running.read().await {
                break;
            }

            self.execute_step(step).await?;
        }

        *self.running.write().await = false;

        Ok(())
    }

    /// Run in continuous mode.
    pub async fn run(&self, duration: Option<Duration>) -> anyhow::Result<()> {
        info!("Running simulation in continuous mode");

        *self.running.write().await = true;

        let start = std::time::Instant::now();

        // Spawn settlement generator
        let banks = self.banks.clone();
        let metrics = self.metrics.clone();
        let rng = self.rng.clone();
        let running = self.running.clone();
        let speed = self.speed;

        let handle = tokio::spawn(async move {
            loop {
                if !*running.read().await {
                    break;
                }

                // Generate random settlement
                let banks_guard = banks.read().await;
                if banks_guard.len() < 2 {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }

                let (from_idx, to_idx) = {
                    let mut rng_guard = rng.write().await;
                    let from = rng_guard.gen_range(0..banks_guard.len());
                    let mut to = rng_guard.gen_range(0..banks_guard.len());
                    while to == from {
                        to = rng_guard.gen_range(0..banks_guard.len());
                    }
                    (from, to)
                };

                let amount = {
                    let mut rng_guard = rng.write().await;
                    Decimal::from(rng_guard.gen_range(1000..1_000_000))
                };

                let from_bank = &banks_guard[from_idx];
                let to_bank = &banks_guard[to_idx];

                info!(
                    "Generating settlement: {} -> {} for ${}",
                    from_bank.id, to_bank.id, amount
                );

                // Simulate settlement
                let settlement_id = SettlementId::new().to_string();
                let money = Money::new(amount, Currency::usd());

                // Debit from sender
                if from_bank.debit(&money).await.is_ok() {
                    // Credit to receiver
                    to_bank.credit(&money).await;

                    from_bank.record_sent(settlement_id.clone()).await;
                    to_bank.record_received(settlement_id.clone()).await;

                    // Record metrics
                    let latency = rng.write().await.gen_range(50..500);
                    metrics.write().await.record_success(latency);
                } else {
                    metrics.write().await.record_failure();
                }

                drop(banks_guard);

                // Wait based on speed
                let delay = Duration::from_millis((1000.0 / speed) as u64);
                tokio::time::sleep(delay).await;
            }
        });

        // Wait for duration or Ctrl+C
        match duration {
            Some(d) => {
                tokio::time::sleep(d).await;
            }
            None => {
                tokio::signal::ctrl_c().await?;
            }
        }

        *self.running.write().await = false;
        handle.await?;

        Ok(())
    }

    /// Execute a single scenario step.
    async fn execute_step(&self, step: &ScenarioStep) -> anyhow::Result<()> {
        match step {
            ScenarioStep::Wait { seconds } => {
                let adjusted = (*seconds as f64 / self.speed) as u64;
                info!("Waiting {} seconds (adjusted: {})", seconds, adjusted);
                tokio::time::sleep(Duration::from_secs(adjusted)).await;
            }
            ScenarioStep::SendSettlement {
                from_bank,
                to_bank,
                amount,
                currency,
            } => {
                info!(
                    "Sending settlement: {} -> {} {} {}",
                    from_bank, to_bank, amount, currency
                );

                let banks = self.banks.read().await;
                let from = banks.iter().find(|b| b.id.as_str() == from_bank);
                let to = banks.iter().find(|b| b.id.as_str() == to_bank);

                if let (Some(from), Some(to)) = (from, to) {
                    let amount_dec = Decimal::from_str_exact(amount).unwrap_or(Decimal::ZERO);
                    let currency = Currency::new(currency);
                    let money = Money::new(amount_dec, currency);

                    if from.debit(&money).await.is_ok() {
                        to.credit(&money).await;
                        let settlement_id = SettlementId::new().to_string();
                        from.record_sent(settlement_id.clone()).await;
                        to.record_received(settlement_id).await;
                        self.metrics.write().await.record_success(100);
                    } else {
                        self.metrics.write().await.record_failure();
                    }
                } else {
                    warn!("Banks not found: {} or {}", from_bank, to_bank);
                }
            }
            ScenarioStep::InjectFault { fault_type, target } => {
                info!("Injecting fault {:?} on {}", fault_type, target);
                // Fault injection would be implemented here
            }
            ScenarioStep::ClearFault { target } => {
                info!("Clearing fault on {}", target);
                // Fault clearing would be implemented here
            }
            ScenarioStep::Assert { condition } => {
                info!("Asserting condition: {:?}", condition);
                // Assertion would be implemented here
            }
        }

        Ok(())
    }

    /// Get simulation metrics.
    pub fn get_metrics(&self) -> SimulationMetrics {
        // Block on async read
        futures::executor::block_on(async { self.metrics.read().await.clone() })
    }

    /// Stop the simulation.
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }
}
