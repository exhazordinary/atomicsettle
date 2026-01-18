//! Account definitions for ledger.

use atomicsettle_common::{AccountId, Currency, ParticipantId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Account status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountStatus {
    /// Account is active and can transact.
    Active,
    /// Account is frozen (no transactions allowed).
    Frozen,
    /// Account is closed.
    Closed,
}

/// A ledger account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Unique account identifier.
    pub id: AccountId,
    /// Owning participant.
    pub participant_id: ParticipantId,
    /// Account name/description.
    pub name: String,
    /// Account currency.
    pub currency: Currency,
    /// Account status.
    pub status: AccountStatus,
    /// When the account was created.
    pub created_at: DateTime<Utc>,
    /// When the account was last updated.
    pub updated_at: DateTime<Utc>,
}

impl Account {
    /// Create a new account.
    pub fn new(
        participant_id: ParticipantId,
        account_number: impl Into<String>,
        currency: Currency,
        name: impl Into<String>,
    ) -> Self {
        let account_number = account_number.into();
        let now = Utc::now();

        Self {
            id: AccountId::new(participant_id.clone(), &account_number, currency.code()),
            participant_id,
            name: name.into(),
            currency,
            status: AccountStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if account can transact.
    pub fn can_transact(&self) -> bool {
        self.status == AccountStatus::Active
    }

    /// Freeze the account.
    pub fn freeze(&mut self) {
        self.status = AccountStatus::Frozen;
        self.updated_at = Utc::now();
    }

    /// Unfreeze the account.
    pub fn unfreeze(&mut self) {
        self.status = AccountStatus::Active;
        self.updated_at = Utc::now();
    }

    /// Close the account.
    pub fn close(&mut self) {
        self.status = AccountStatus::Closed;
        self.updated_at = Utc::now();
    }
}
