//! Journal entry types for double-entry bookkeeping.

use atomicsettle_common::{AccountId, Currency, SettlementId};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Type of journal entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryType {
    /// Debit entry (increases asset/expense, decreases liability/equity/revenue).
    Debit,
    /// Credit entry (decreases asset/expense, increases liability/equity/revenue).
    Credit,
}

/// A single journal entry in the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Unique entry ID.
    pub id: Uuid,
    /// Settlement this entry belongs to.
    pub settlement_id: SettlementId,
    /// Leg number within the settlement.
    pub leg_number: u32,
    /// Account affected.
    pub account_id: AccountId,
    /// Entry type (debit or credit).
    pub entry_type: EntryType,
    /// Amount.
    pub amount: Decimal,
    /// Currency.
    pub currency: Currency,
    /// Balance after this entry.
    pub balance_after: Decimal,
    /// When this entry was created.
    pub created_at: DateTime<Utc>,
}

impl JournalEntry {
    /// Create a debit entry.
    pub fn debit(
        settlement_id: SettlementId,
        leg_number: u32,
        account_id: AccountId,
        amount: Decimal,
        currency: Currency,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            settlement_id,
            leg_number,
            account_id,
            entry_type: EntryType::Debit,
            amount,
            currency,
            balance_after: Decimal::ZERO,
            created_at: Utc::now(),
        }
    }

    /// Create a credit entry.
    pub fn credit(
        settlement_id: SettlementId,
        leg_number: u32,
        account_id: AccountId,
        amount: Decimal,
        currency: Currency,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            settlement_id,
            leg_number,
            account_id,
            entry_type: EntryType::Credit,
            amount,
            currency,
            balance_after: Decimal::ZERO,
            created_at: Utc::now(),
        }
    }

    /// Get signed amount (positive for debit, negative for credit in asset accounts).
    pub fn signed_amount(&self) -> Decimal {
        match self.entry_type {
            EntryType::Debit => self.amount,
            EntryType::Credit => -self.amount,
        }
    }
}

/// A batch of journal entries that must be committed together.
#[derive(Debug, Clone)]
pub struct JournalBatch {
    /// Entries in the batch.
    pub entries: Vec<JournalEntry>,
    /// Settlement ID for the batch.
    pub settlement_id: SettlementId,
}

impl JournalBatch {
    /// Create a new batch.
    pub fn new(settlement_id: SettlementId) -> Self {
        Self {
            entries: Vec::new(),
            settlement_id,
        }
    }

    /// Add an entry to the batch.
    pub fn add_entry(&mut self, entry: JournalEntry) {
        self.entries.push(entry);
    }

    /// Verify the batch is balanced (debits == credits per currency).
    pub fn is_balanced(&self) -> bool {
        use std::collections::HashMap;

        let mut balances: HashMap<String, Decimal> = HashMap::new();

        for entry in &self.entries {
            let currency = entry.currency.code().to_string();
            let amount = match entry.entry_type {
                EntryType::Debit => entry.amount,
                EntryType::Credit => -entry.amount,
            };

            *balances.entry(currency).or_insert(Decimal::ZERO) += amount;
        }

        balances.values().all(|&balance| balance == Decimal::ZERO)
    }

    /// Get total debits.
    pub fn total_debits(&self) -> Decimal {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Debit)
            .map(|e| e.amount)
            .sum()
    }

    /// Get total credits.
    pub fn total_credits(&self) -> Decimal {
        self.entries
            .iter()
            .filter(|e| e.entry_type == EntryType::Credit)
            .map(|e| e.amount)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::ParticipantId;

    #[test]
    fn test_balanced_batch() {
        let settlement_id = SettlementId::new();
        let mut batch = JournalBatch::new(settlement_id);

        let account_a = AccountId::new(ParticipantId::new("BANK_A"), "12345", "USD");
        let account_b = AccountId::new(ParticipantId::new("BANK_B"), "67890", "USD");

        batch.add_entry(JournalEntry::debit(
            settlement_id,
            1,
            account_a,
            Decimal::from(1000),
            Currency::usd(),
        ));

        batch.add_entry(JournalEntry::credit(
            settlement_id,
            1,
            account_b,
            Decimal::from(1000),
            Currency::usd(),
        ));

        assert!(batch.is_balanced());
        assert_eq!(batch.total_debits(), Decimal::from(1000));
        assert_eq!(batch.total_credits(), Decimal::from(1000));
    }

    #[test]
    fn test_unbalanced_batch() {
        let settlement_id = SettlementId::new();
        let mut batch = JournalBatch::new(settlement_id);

        let account_a = AccountId::new(ParticipantId::new("BANK_A"), "12345", "USD");

        batch.add_entry(JournalEntry::debit(
            settlement_id,
            1,
            account_a,
            Decimal::from(1000),
            Currency::usd(),
        ));

        assert!(!batch.is_balanced());
    }
}
