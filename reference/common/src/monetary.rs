//! Monetary types for AtomicSettle protocol.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Mul, Sub};

/// A monetary amount with currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    /// The amount value (high precision decimal).
    pub value: Decimal,
    /// ISO 4217 currency code.
    pub currency: Currency,
}

impl Money {
    /// Create a new Money instance.
    pub fn new(value: Decimal, currency: Currency) -> Self {
        Self { value, currency }
    }

    /// Create from a string value.
    pub fn from_str(value: &str, currency: Currency) -> Result<Self, rust_decimal::Error> {
        Ok(Self {
            value: value.parse()?,
            currency,
        })
    }

    /// Create a zero amount in the given currency.
    pub fn zero(currency: Currency) -> Self {
        Self {
            value: Decimal::ZERO,
            currency,
        }
    }

    /// Check if the amount is positive.
    pub fn is_positive(&self) -> bool {
        self.value > Decimal::ZERO
    }

    /// Check if the amount is zero.
    pub fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    /// Check if the amount is negative.
    pub fn is_negative(&self) -> bool {
        self.value < Decimal::ZERO
    }

    /// Get the absolute value.
    pub fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
            currency: self.currency.clone(),
        }
    }

    /// Round to the currency's standard decimal places.
    pub fn round(&self) -> Self {
        let places = self.currency.decimal_places();
        Self {
            value: self.value.round_dp(places),
            currency: self.currency.clone(),
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.value, self.currency)
    }
}

impl Add for Money {
    type Output = Result<Money, CurrencyMismatchError>;

    fn add(self, other: Money) -> Self::Output {
        if self.currency != other.currency {
            return Err(CurrencyMismatchError {
                expected: self.currency,
                actual: other.currency,
            });
        }
        Ok(Money {
            value: self.value + other.value,
            currency: self.currency,
        })
    }
}

impl Sub for Money {
    type Output = Result<Money, CurrencyMismatchError>;

    fn sub(self, other: Money) -> Self::Output {
        if self.currency != other.currency {
            return Err(CurrencyMismatchError {
                expected: self.currency,
                actual: other.currency,
            });
        }
        Ok(Money {
            value: self.value - other.value,
            currency: self.currency,
        })
    }
}

impl Mul<Decimal> for Money {
    type Output = Money;

    fn mul(self, rate: Decimal) -> Self::Output {
        Money {
            value: self.value * rate,
            currency: self.currency,
        }
    }
}

/// Error when attempting operations on different currencies.
#[derive(Debug, Clone)]
pub struct CurrencyMismatchError {
    pub expected: Currency,
    pub actual: Currency,
}

impl fmt::Display for CurrencyMismatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Currency mismatch: expected {}, got {}",
            self.expected, self.actual
        )
    }
}

impl std::error::Error for CurrencyMismatchError {}

/// ISO 4217 currency code.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Currency(String);

impl Currency {
    /// Create a new currency from code.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }

    /// Get the currency code.
    pub fn code(&self) -> &str {
        &self.0
    }

    /// Get the standard decimal places for this currency.
    pub fn decimal_places(&self) -> u32 {
        match self.0.as_str() {
            "JPY" | "KRW" | "VND" => 0,
            "BHD" | "KWD" | "OMR" => 3,
            _ => 2,
        }
    }

    /// Common currencies
    pub fn usd() -> Self {
        Self::new("USD")
    }

    pub fn eur() -> Self {
        Self::new("EUR")
    }

    pub fn gbp() -> Self {
        Self::new("GBP")
    }

    pub fn jpy() -> Self {
        Self::new("JPY")
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Currency {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// A currency pair for FX operations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurrencyPair {
    /// Base currency (being bought/sold).
    pub base: Currency,
    /// Quote currency (pricing currency).
    pub quote: Currency,
}

impl CurrencyPair {
    /// Create a new currency pair.
    pub fn new(base: Currency, quote: Currency) -> Self {
        Self { base, quote }
    }

    /// Get the inverse pair.
    pub fn inverse(&self) -> Self {
        Self {
            base: self.quote.clone(),
            quote: self.base.clone(),
        }
    }
}

impl fmt::Display for CurrencyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// FX rate between two currencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxRate {
    /// The currency pair.
    pub pair: CurrencyPair,
    /// Bid price (buy price from market maker's perspective).
    pub bid: Decimal,
    /// Ask price (sell price from market maker's perspective).
    pub ask: Decimal,
    /// Mid-market rate.
    pub mid: Decimal,
    /// When this rate was quoted.
    pub quoted_at: chrono::DateTime<chrono::Utc>,
    /// When this rate expires.
    pub valid_until: chrono::DateTime<chrono::Utc>,
    /// Rate source.
    pub source: String,
}

impl FxRate {
    /// Create a new FX rate.
    pub fn new(
        pair: CurrencyPair,
        bid: Decimal,
        ask: Decimal,
        valid_for_seconds: i64,
        source: impl Into<String>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            pair,
            bid,
            ask,
            mid: (bid + ask) / Decimal::TWO,
            quoted_at: now,
            valid_until: now + chrono::Duration::seconds(valid_for_seconds),
            source: source.into(),
        }
    }

    /// Check if the rate is still valid.
    pub fn is_valid(&self) -> bool {
        chrono::Utc::now() < self.valid_until
    }

    /// Get the spread in basis points.
    pub fn spread_bps(&self) -> Decimal {
        ((self.ask - self.bid) / self.mid) * Decimal::from(10000)
    }

    /// Convert an amount using the mid rate.
    pub fn convert(&self, amount: &Money) -> Result<Money, CurrencyMismatchError> {
        if amount.currency != self.pair.base {
            return Err(CurrencyMismatchError {
                expected: self.pair.base.clone(),
                actual: amount.currency.clone(),
            });
        }

        Ok(Money::new(amount.value * self.mid, self.pair.quote.clone()).round())
    }
}

/// Account balance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// Currency of the balance.
    pub currency: Currency,
    /// Available balance (can be used for settlements).
    pub available: Decimal,
    /// Locked balance (reserved for pending settlements).
    pub locked: Decimal,
    /// Pending incoming amount.
    pub pending_in: Decimal,
    /// Pending outgoing amount.
    pub pending_out: Decimal,
}

impl Balance {
    /// Create a new balance.
    pub fn new(currency: Currency) -> Self {
        Self {
            currency,
            available: Decimal::ZERO,
            locked: Decimal::ZERO,
            pending_in: Decimal::ZERO,
            pending_out: Decimal::ZERO,
        }
    }

    /// Get the total balance (available + locked).
    pub fn total(&self) -> Decimal {
        self.available + self.locked
    }

    /// Check if amount can be locked.
    pub fn can_lock(&self, amount: Decimal) -> bool {
        self.available >= amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_money_operations() {
        let m1 = Money::from_str("100.00", Currency::usd()).unwrap();
        let m2 = Money::from_str("50.00", Currency::usd()).unwrap();

        let sum = (m1.clone() + m2.clone()).unwrap();
        assert_eq!(sum.value, Decimal::from(150));

        let diff = (m1.clone() - m2.clone()).unwrap();
        assert_eq!(diff.value, Decimal::from(50));
    }

    #[test]
    fn test_currency_mismatch() {
        let m1 = Money::from_str("100.00", Currency::usd()).unwrap();
        let m2 = Money::from_str("100.00", Currency::eur()).unwrap();

        assert!((m1 + m2).is_err());
    }

    #[test]
    fn test_fx_rate_conversion() {
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());
        let rate = FxRate::new(
            pair,
            Decimal::from_str_exact("0.91").unwrap(),
            Decimal::from_str_exact("0.93").unwrap(),
            30,
            "TEST",
        );

        let usd = Money::from_str("1000.00", Currency::usd()).unwrap();
        let eur = rate.convert(&usd).unwrap();

        assert_eq!(eur.currency, Currency::eur());
        // Mid rate is 0.92
        assert_eq!(eur.value, Decimal::from(920));
    }

    #[test]
    fn test_currency_decimal_places() {
        assert_eq!(Currency::usd().decimal_places(), 2);
        assert_eq!(Currency::eur().decimal_places(), 2);
        assert_eq!(Currency::jpy().decimal_places(), 0);
    }
}
