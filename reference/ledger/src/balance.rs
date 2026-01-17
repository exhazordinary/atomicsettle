//! Account balance tracking.

use atomicsettle_common::{AccountId, Currency};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Account balance at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    /// Account identifier.
    pub account_id: AccountId,
    /// Currency.
    pub currency: Currency,
    /// Available balance (can be used for transactions).
    pub balance: Decimal,
    /// Locked balance (reserved for pending settlements).
    pub locked_balance: Decimal,
    /// Pending credits (incoming settlements not yet final).
    pub pending_credits: Decimal,
    /// Pending debits (outgoing settlements not yet final).
    pub pending_debits: Decimal,
    /// When this balance was last updated.
    pub updated_at: DateTime<Utc>,
}

impl AccountBalance {
    /// Create a new zero balance.
    pub fn zero(account_id: AccountId, currency: Currency) -> Self {
        Self {
            account_id,
            currency,
            balance: Decimal::ZERO,
            locked_balance: Decimal::ZERO,
            pending_credits: Decimal::ZERO,
            pending_debits: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    /// Get total balance (available + locked).
    pub fn total(&self) -> Decimal {
        self.balance + self.locked_balance
    }

    /// Get available balance (excludes locked).
    pub fn available(&self) -> Decimal {
        self.balance
    }

    /// Check if amount can be locked.
    pub fn can_lock(&self, amount: Decimal) -> bool {
        self.balance >= amount
    }

    /// Check if account has sufficient funds for a transaction.
    pub fn has_sufficient_funds(&self, amount: Decimal) -> bool {
        self.balance >= amount
    }

    /// Get projected balance (includes pending).
    pub fn projected(&self) -> Decimal {
        self.total() + self.pending_credits - self.pending_debits
    }
}

/// Balance change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceChange {
    /// Account affected.
    pub account_id: AccountId,
    /// Change type.
    pub change_type: BalanceChangeType,
    /// Amount changed.
    pub amount: Decimal,
    /// Balance before change.
    pub balance_before: Decimal,
    /// Balance after change.
    pub balance_after: Decimal,
    /// Reference (settlement ID, etc.).
    pub reference: String,
    /// When the change occurred.
    pub timestamp: DateTime<Utc>,
}

/// Type of balance change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BalanceChangeType {
    /// Credit (increase).
    Credit,
    /// Debit (decrease).
    Debit,
    /// Lock (move from available to locked).
    Lock,
    /// Unlock (move from locked to available).
    Unlock,
    /// Consume locked (locked funds used for settlement).
    ConsumeLocked,
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::ParticipantId;

    #[test]
    fn test_balance_operations() {
        let account_id = AccountId::new(ParticipantId::new("BANK_A"), "12345", "USD");
        let mut balance = AccountBalance::zero(account_id, Currency::usd());

        balance.balance = Decimal::from(10000);
        balance.locked_balance = Decimal::from(2000);

        assert_eq!(balance.total(), Decimal::from(12000));
        assert_eq!(balance.available(), Decimal::from(10000));
        assert!(balance.can_lock(Decimal::from(5000)));
        assert!(!balance.can_lock(Decimal::from(15000)));
    }
}
