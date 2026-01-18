//! Simulated bank for testing.

use std::sync::Arc;

use rust_decimal::Decimal;
use tokio::sync::RwLock;

use atomicsettle_common::{Currency, Money, ParticipantId};

/// A simulated bank for testing.
pub struct SimulatedBank {
    /// Bank identifier.
    pub id: ParticipantId,
    /// Bank name.
    #[allow(dead_code)]
    pub name: String,
    /// Current balances by currency.
    balances: Arc<RwLock<std::collections::HashMap<Currency, Decimal>>>,
    /// Settlement history.
    settlements_sent: Arc<RwLock<Vec<String>>>,
    settlements_received: Arc<RwLock<Vec<String>>>,
}

impl SimulatedBank {
    /// Create a new simulated bank.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let id_str = id.into();
        Self {
            id: ParticipantId::new(&id_str),
            name: name.into(),
            balances: Arc::new(RwLock::new(std::collections::HashMap::new())),
            settlements_sent: Arc::new(RwLock::new(Vec::new())),
            settlements_received: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize with default balances.
    pub async fn initialize_balances(&self, initial_balance: Decimal) {
        let mut balances = self.balances.write().await;
        balances.insert(Currency::usd(), initial_balance);
        balances.insert(Currency::eur(), initial_balance * Decimal::from_str_exact("0.92").unwrap());
        balances.insert(Currency::gbp(), initial_balance * Decimal::from_str_exact("0.79").unwrap());
    }

    /// Get balance for currency.
    #[allow(dead_code)]
    pub async fn get_balance(&self, currency: &Currency) -> Decimal {
        self.balances
            .read()
            .await
            .get(currency)
            .copied()
            .unwrap_or(Decimal::ZERO)
    }

    /// Debit balance (for sending).
    pub async fn debit(&self, amount: &Money) -> Result<(), String> {
        let mut balances = self.balances.write().await;
        let balance = balances.entry(amount.currency.clone()).or_insert(Decimal::ZERO);

        if *balance < amount.value {
            return Err(format!(
                "Insufficient balance: {} < {}",
                balance, amount.value
            ));
        }

        *balance -= amount.value;
        Ok(())
    }

    /// Credit balance (for receiving).
    pub async fn credit(&self, amount: &Money) {
        let mut balances = self.balances.write().await;
        let balance = balances.entry(amount.currency.clone()).or_insert(Decimal::ZERO);
        *balance += amount.value;
    }

    /// Record sent settlement.
    pub async fn record_sent(&self, settlement_id: String) {
        self.settlements_sent.write().await.push(settlement_id);
    }

    /// Record received settlement.
    pub async fn record_received(&self, settlement_id: String) {
        self.settlements_received.write().await.push(settlement_id);
    }

    /// Get count of sent settlements.
    #[allow(dead_code)]
    pub async fn sent_count(&self) -> usize {
        self.settlements_sent.read().await.len()
    }

    /// Get count of received settlements.
    #[allow(dead_code)]
    pub async fn received_count(&self) -> usize {
        self.settlements_received.read().await.len()
    }
}


/// Bank factory for creating test banks.
pub struct BankFactory;

impl BankFactory {
    /// Create N simulated banks.
    pub fn create_banks(count: usize) -> Vec<SimulatedBank> {
        let bank_names = [
            ("BANK_A", "Alpha Bank"),
            ("BANK_B", "Beta Financial"),
            ("BANK_C", "Central Trust"),
            ("BANK_D", "Delta Holdings"),
            ("BANK_E", "Eastern Bank"),
            ("BANK_F", "First National"),
            ("BANK_G", "Global Finance"),
            ("BANK_H", "Harbor Bank"),
            ("BANK_I", "International Trust"),
            ("BANK_J", "Jade Financial"),
        ];

        (0..count)
            .map(|i| {
                let (id, name) = if i < bank_names.len() {
                    bank_names[i]
                } else {
                    // Generate names for banks beyond the predefined list
                    let id = format!("BANK_{}", i + 1);
                    let name = format!("Bank {}", i + 1);
                    return SimulatedBank::new(id, name);
                };
                SimulatedBank::new(id, name)
            })
            .collect()
    }
}
