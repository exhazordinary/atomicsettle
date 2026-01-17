//! Currency conversion types and operations.

use atomicsettle_common::{Currency, CurrencyPair, FxRate, Money};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rate_lock::RateLock;

/// Represents a completed currency conversion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversion {
    /// Unique conversion ID.
    pub id: Uuid,
    /// Input amount.
    pub input: Money,
    /// Output amount.
    pub output: Money,
    /// Rate used for conversion.
    pub rate: FxRate,
    /// Whether a rate lock was used.
    pub rate_lock_id: Option<Uuid>,
    /// When the conversion was executed.
    pub executed_at: DateTime<Utc>,
}

impl Conversion {
    /// Create a new conversion record.
    pub fn new(input: Money, output: Money, rate: FxRate, rate_lock_id: Option<Uuid>) -> Self {
        Self {
            id: Uuid::now_v7(),
            input,
            output,
            rate,
            rate_lock_id,
            executed_at: Utc::now(),
        }
    }

    /// Get the effective rate used.
    pub fn effective_rate(&self) -> Decimal {
        if self.input.value.is_zero() {
            return Decimal::ZERO;
        }
        self.output.value / self.input.value
    }

    /// Get the currency pair.
    pub fn pair(&self) -> CurrencyPair {
        CurrencyPair::new(self.input.currency.clone(), self.output.currency.clone())
    }
}

/// Request to perform a conversion.
#[derive(Debug, Clone)]
pub struct ConversionRequest {
    /// Amount to convert.
    pub amount: Money,
    /// Target currency.
    pub target_currency: Currency,
    /// Optional rate lock to use.
    pub rate_lock: Option<RateLock>,
    /// Whether to use bid or ask rate.
    pub rate_side: RateSide,
}

impl ConversionRequest {
    /// Create a new conversion request.
    pub fn new(amount: Money, target_currency: Currency) -> Self {
        Self {
            amount,
            target_currency,
            rate_lock: None,
            rate_side: RateSide::Mid,
        }
    }

    /// Use a specific rate lock.
    pub fn with_rate_lock(mut self, lock: RateLock) -> Self {
        self.rate_lock = Some(lock);
        self
    }

    /// Use bid rate (for selling base currency).
    pub fn at_bid(mut self) -> Self {
        self.rate_side = RateSide::Bid;
        self
    }

    /// Use ask rate (for buying base currency).
    pub fn at_ask(mut self) -> Self {
        self.rate_side = RateSide::Ask;
        self
    }
}

/// Which side of the rate to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateSide {
    /// Use bid price.
    Bid,
    /// Use ask price.
    Ask,
    /// Use mid-market rate.
    Mid,
}

impl RateSide {
    /// Get the rate value from an FxRate.
    pub fn get_rate(&self, rate: &FxRate) -> Decimal {
        match self {
            RateSide::Bid => rate.bid,
            RateSide::Ask => rate.ask,
            RateSide::Mid => rate.mid,
        }
    }
}

/// Builder for complex conversions.
pub struct ConversionBuilder {
    amount: Option<Money>,
    target_currency: Option<Currency>,
    rate_lock: Option<RateLock>,
    rate_side: RateSide,
}

impl ConversionBuilder {
    /// Create a new conversion builder.
    pub fn new() -> Self {
        Self {
            amount: None,
            target_currency: None,
            rate_lock: None,
            rate_side: RateSide::Mid,
        }
    }

    /// Set the amount to convert.
    pub fn amount(mut self, amount: Money) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Set the target currency.
    pub fn to(mut self, currency: Currency) -> Self {
        self.target_currency = Some(currency);
        self
    }

    /// Use a rate lock.
    pub fn with_lock(mut self, lock: RateLock) -> Self {
        self.rate_lock = Some(lock);
        self
    }

    /// Use bid rate.
    pub fn at_bid(mut self) -> Self {
        self.rate_side = RateSide::Bid;
        self
    }

    /// Use ask rate.
    pub fn at_ask(mut self) -> Self {
        self.rate_side = RateSide::Ask;
        self
    }

    /// Build the conversion request.
    pub fn build(self) -> Option<ConversionRequest> {
        Some(ConversionRequest {
            amount: self.amount?,
            target_currency: self.target_currency?,
            rate_lock: self.rate_lock,
            rate_side: self.rate_side,
        })
    }
}

impl Default for ConversionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_conversion_effective_rate() {
        let input = Money::new(dec!(1000), Currency::usd());
        let output = Money::new(dec!(920), Currency::eur());
        let rate = FxRate::new(
            CurrencyPair::new(Currency::usd(), Currency::eur()),
            dec!(0.91),
            dec!(0.93),
            30,
            "TEST",
        );

        let conversion = Conversion::new(input, output, rate, None);

        assert_eq!(conversion.effective_rate(), dec!(0.92));
    }

    #[test]
    fn test_rate_side() {
        let rate = FxRate::new(
            CurrencyPair::new(Currency::usd(), Currency::eur()),
            dec!(0.91),
            dec!(0.93),
            30,
            "TEST",
        );

        assert_eq!(RateSide::Bid.get_rate(&rate), dec!(0.91));
        assert_eq!(RateSide::Ask.get_rate(&rate), dec!(0.93));
        assert_eq!(RateSide::Mid.get_rate(&rate), dec!(0.92));
    }

    #[test]
    fn test_conversion_builder() {
        let request = ConversionBuilder::new()
            .amount(Money::new(dec!(1000), Currency::usd()))
            .to(Currency::eur())
            .at_bid()
            .build()
            .unwrap();

        assert_eq!(request.amount.value, dec!(1000));
        assert_eq!(request.target_currency, Currency::eur());
        assert_eq!(request.rate_side, RateSide::Bid);
    }
}
