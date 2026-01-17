//! Core ledger engine implementation.

use std::sync::Arc;

use rust_decimal::Decimal;
use tracing::{info, instrument};

use atomicsettle_common::{AccountId, Currency, Money, Result, Settlement, SettlementId};

use crate::account::Account;
use crate::balance::AccountBalance;
use crate::journal::{EntryType, JournalEntry};

/// The ledger engine manages double-entry bookkeeping for settlements.
pub struct LedgerEngine {
    /// Database connection pool (placeholder).
    // db: sqlx::PgPool,
}

impl LedgerEngine {
    /// Create a new ledger engine.
    pub fn new() -> Self {
        Self {}
    }

    /// Record a settlement with full audit trail.
    #[instrument(skip(self, settlement))]
    pub async fn record_settlement(&self, settlement: &Settlement) -> Result<Vec<JournalEntry>> {
        info!(
            settlement_id = %settlement.id,
            legs = settlement.legs.len(),
            "Recording settlement"
        );

        let mut entries = Vec::new();

        // Create journal entries for each leg
        for leg in &settlement.legs {
            // Debit source account
            let debit_entry = JournalEntry {
                id: uuid::Uuid::new_v4(),
                settlement_id: settlement.id,
                leg_number: leg.leg_number,
                account_id: leg.from_account.clone(),
                entry_type: EntryType::Debit,
                amount: leg.amount.value,
                currency: leg.amount.currency.clone(),
                balance_after: Decimal::ZERO, // Would be calculated from DB
                created_at: chrono::Utc::now(),
            };
            entries.push(debit_entry);

            // Credit destination account
            let amount = leg
                .converted_amount
                .as_ref()
                .unwrap_or(&leg.amount);

            let credit_entry = JournalEntry {
                id: uuid::Uuid::new_v4(),
                settlement_id: settlement.id,
                leg_number: leg.leg_number,
                account_id: leg.to_account.clone(),
                entry_type: EntryType::Credit,
                amount: amount.value,
                currency: amount.currency.clone(),
                balance_after: Decimal::ZERO, // Would be calculated from DB
                created_at: chrono::Utc::now(),
            };
            entries.push(credit_entry);
        }

        // In a real implementation:
        // 1. Start database transaction
        // 2. Insert all journal entries
        // 3. Update account balances
        // 4. Verify debits == credits
        // 5. Commit transaction

        info!(
            settlement_id = %settlement.id,
            entries = entries.len(),
            "Settlement recorded"
        );

        Ok(entries)
    }

    /// Get account balance.
    pub async fn get_balance(&self, account_id: &AccountId) -> Result<AccountBalance> {
        // In a real implementation, query from database
        Ok(AccountBalance {
            account_id: account_id.clone(),
            currency: Currency::new(&account_id.currency),
            balance: Decimal::ZERO,
            locked_balance: Decimal::ZERO,
            pending_credits: Decimal::ZERO,
            pending_debits: Decimal::ZERO,
            updated_at: chrono::Utc::now(),
        })
    }

    /// Debit an account (reduce balance).
    #[instrument(skip(self))]
    pub async fn debit(
        &self,
        account_id: &AccountId,
        amount: Decimal,
        settlement_id: SettlementId,
        leg_number: u32,
    ) -> Result<JournalEntry> {
        info!(
            account = %account_id,
            amount = %amount,
            settlement_id = %settlement_id,
            "Debiting account"
        );

        let entry = JournalEntry {
            id: uuid::Uuid::new_v4(),
            settlement_id,
            leg_number,
            account_id: account_id.clone(),
            entry_type: EntryType::Debit,
            amount,
            currency: Currency::new(&account_id.currency),
            balance_after: Decimal::ZERO,
            created_at: chrono::Utc::now(),
        };

        Ok(entry)
    }

    /// Credit an account (increase balance).
    #[instrument(skip(self))]
    pub async fn credit(
        &self,
        account_id: &AccountId,
        amount: Decimal,
        settlement_id: SettlementId,
        leg_number: u32,
    ) -> Result<JournalEntry> {
        info!(
            account = %account_id,
            amount = %amount,
            settlement_id = %settlement_id,
            "Crediting account"
        );

        let entry = JournalEntry {
            id: uuid::Uuid::new_v4(),
            settlement_id,
            leg_number,
            account_id: account_id.clone(),
            entry_type: EntryType::Credit,
            amount,
            currency: Currency::new(&account_id.currency),
            balance_after: Decimal::ZERO,
            created_at: chrono::Utc::now(),
        };

        Ok(entry)
    }

    /// Lock funds in an account.
    pub async fn lock_funds(&self, account_id: &AccountId, amount: Decimal) -> Result<()> {
        info!(
            account = %account_id,
            amount = %amount,
            "Locking funds"
        );

        // In a real implementation:
        // UPDATE accounts
        // SET available_balance = available_balance - amount,
        //     locked_balance = locked_balance + amount
        // WHERE account_id = $1 AND available_balance >= amount

        Ok(())
    }

    /// Unlock funds in an account.
    pub async fn unlock_funds(&self, account_id: &AccountId, amount: Decimal) -> Result<()> {
        info!(
            account = %account_id,
            amount = %amount,
            "Unlocking funds"
        );

        // In a real implementation:
        // UPDATE accounts
        // SET available_balance = available_balance + amount,
        //     locked_balance = locked_balance - amount
        // WHERE account_id = $1

        Ok(())
    }

    /// Get journal entries for a settlement.
    pub async fn get_settlement_entries(
        &self,
        settlement_id: SettlementId,
    ) -> Result<Vec<JournalEntry>> {
        // In a real implementation, query from database
        Ok(Vec::new())
    }

    /// Verify ledger integrity (debits == credits).
    pub async fn verify_integrity(&self) -> Result<bool> {
        // In a real implementation:
        // SELECT SUM(CASE WHEN entry_type = 'DEBIT' THEN amount ELSE 0 END) as total_debits,
        //        SUM(CASE WHEN entry_type = 'CREDIT' THEN amount ELSE 0 END) as total_credits
        // FROM journal_entries
        // GROUP BY currency
        // HAVING total_debits != total_credits

        Ok(true)
    }
}

impl Default for LedgerEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::{ParticipantId, SettlementLeg};

    fn create_test_settlement() -> Settlement {
        let leg = SettlementLeg::new(
            1,
            ParticipantId::new("BANK_A"),
            AccountId::new(ParticipantId::new("BANK_A"), "12345", "USD"),
            ParticipantId::new("BANK_B"),
            AccountId::new(ParticipantId::new("BANK_B"), "67890", "USD"),
            Money::new(Decimal::from(1000), Currency::usd()),
        );

        Settlement::new("test-key".to_string(), vec![leg])
    }

    #[tokio::test]
    async fn test_record_settlement() {
        let engine = LedgerEngine::new();
        let settlement = create_test_settlement();

        let entries = engine.record_settlement(&settlement).await.unwrap();

        // Should have 2 entries (1 debit, 1 credit) for 1 leg
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.entry_type == EntryType::Debit));
        assert!(entries.iter().any(|e| e.entry_type == EntryType::Credit));
    }
}
