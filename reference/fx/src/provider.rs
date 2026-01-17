//! Rate provider traits and implementations.

use async_trait::async_trait;
use atomicsettle_common::{Currency, CurrencyPair, FxRate};
use rust_decimal::Decimal;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::error::{FxError, FxResult};

/// Trait for FX rate providers.
#[async_trait]
pub trait RateProvider: Send + Sync {
    /// Get the provider name.
    fn name(&self) -> &str;

    /// Get rate for a currency pair.
    async fn get_rate(&self, pair: &CurrencyPair) -> FxResult<FxRate>;

    /// Check if this provider supports the given currency pair.
    fn supports_pair(&self, pair: &CurrencyPair) -> bool;

    /// Get all supported currency pairs.
    fn supported_pairs(&self) -> Vec<CurrencyPair>;
}

/// Aggregates multiple rate providers and returns median rate.
pub struct AggregatedRateProvider {
    providers: Vec<Arc<dyn RateProvider>>,
    min_providers: usize,
    max_deviation_bps: u32,
}

impl AggregatedRateProvider {
    /// Create a new aggregated provider.
    pub fn new(providers: Vec<Arc<dyn RateProvider>>) -> Self {
        Self {
            providers,
            min_providers: 1,
            max_deviation_bps: 100, // 1% max deviation
        }
    }

    /// Set minimum number of providers required for a valid rate.
    pub fn with_min_providers(mut self, min: usize) -> Self {
        self.min_providers = min;
        self
    }

    /// Set maximum allowed deviation between providers in basis points.
    pub fn with_max_deviation(mut self, bps: u32) -> Self {
        self.max_deviation_bps = bps;
        self
    }

    /// Calculate median of rates.
    fn calculate_median(&self, rates: &mut [FxRate]) -> FxRate {
        rates.sort_by(|a, b| a.mid.cmp(&b.mid));
        let mid_idx = rates.len() / 2;

        if rates.len() % 2 == 0 && rates.len() > 1 {
            // Average of two middle values
            let mid = (rates[mid_idx - 1].mid + rates[mid_idx].mid) / Decimal::TWO;
            let bid = (rates[mid_idx - 1].bid + rates[mid_idx].bid) / Decimal::TWO;
            let ask = (rates[mid_idx - 1].ask + rates[mid_idx].ask) / Decimal::TWO;

            FxRate::new(
                rates[mid_idx].pair.clone(),
                bid,
                ask,
                rates[mid_idx].valid_until.signed_duration_since(chrono::Utc::now()).num_seconds(),
                "AGGREGATED",
            )
        } else {
            let mut rate = rates[mid_idx].clone();
            rate.source = "AGGREGATED".to_string();
            rate
        }
    }

    /// Check if rates deviate too much.
    fn check_deviation(&self, rates: &[FxRate], pair: &CurrencyPair) -> FxResult<()> {
        if rates.len() < 2 {
            return Ok(());
        }

        let mids: Vec<Decimal> = rates.iter().map(|r| r.mid).collect();
        let min = mids.iter().min().unwrap();
        let max = mids.iter().max().unwrap();

        if min.is_zero() {
            return Err(FxError::ProviderError("Zero rate detected".to_string()));
        }

        let deviation = ((*max - *min) / *min) * Decimal::from(10000);
        let deviation_bps = deviation.to_string().parse::<u32>().unwrap_or(0);

        if deviation_bps > self.max_deviation_bps {
            return Err(FxError::RateDeviation {
                pair: pair.clone(),
                deviation_bps,
            });
        }

        Ok(())
    }
}

#[async_trait]
impl RateProvider for AggregatedRateProvider {
    fn name(&self) -> &str {
        "AGGREGATED"
    }

    async fn get_rate(&self, pair: &CurrencyPair) -> FxResult<FxRate> {
        let mut rates = Vec::new();
        let mut errors = Vec::new();

        for provider in &self.providers {
            if !provider.supports_pair(pair) {
                continue;
            }

            match provider.get_rate(pair).await {
                Ok(rate) => {
                    debug!(
                        provider = provider.name(),
                        pair = %pair,
                        mid = %rate.mid,
                        "Got rate from provider"
                    );
                    rates.push(rate);
                }
                Err(e) => {
                    warn!(
                        provider = provider.name(),
                        pair = %pair,
                        error = %e,
                        "Provider failed to return rate"
                    );
                    errors.push(e);
                }
            }
        }

        if rates.len() < self.min_providers {
            return Err(FxError::RateNotAvailable(pair.clone()));
        }

        // Check for excessive deviation
        self.check_deviation(&rates, pair)?;

        // Return median rate
        Ok(self.calculate_median(&mut rates))
    }

    fn supports_pair(&self, pair: &CurrencyPair) -> bool {
        self.providers.iter().any(|p| p.supports_pair(pair))
    }

    fn supported_pairs(&self) -> Vec<CurrencyPair> {
        let mut pairs: Vec<CurrencyPair> = self
            .providers
            .iter()
            .flat_map(|p| p.supported_pairs())
            .collect();
        pairs.sort_by(|a, b| format!("{}", a).cmp(&format!("{}", b)));
        pairs.dedup();
        pairs
    }
}

/// Mock rate provider for testing.
#[cfg(any(test, feature = "test-utils"))]
pub struct MockRateProvider {
    name: String,
    rates: dashmap::DashMap<String, FxRate>,
}

#[cfg(any(test, feature = "test-utils"))]
impl MockRateProvider {
    /// Create a new mock provider.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            rates: dashmap::DashMap::new(),
        }
    }

    /// Set a rate for a currency pair.
    pub fn set_rate(&self, rate: FxRate) {
        let key = format!("{}", rate.pair);
        self.rates.insert(key, rate);
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl RateProvider for MockRateProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn get_rate(&self, pair: &CurrencyPair) -> FxResult<FxRate> {
        let key = format!("{}", pair);
        self.rates
            .get(&key)
            .map(|r| r.clone())
            .ok_or_else(|| FxError::RateNotAvailable(pair.clone()))
    }

    fn supports_pair(&self, pair: &CurrencyPair) -> bool {
        let key = format!("{}", pair);
        self.rates.contains_key(&key)
    }

    fn supported_pairs(&self) -> Vec<CurrencyPair> {
        self.rates.iter().map(|r| r.pair.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_test_rate(base: &str, quote: &str, bid: Decimal, ask: Decimal) -> FxRate {
        FxRate::new(
            CurrencyPair::new(Currency::new(base), Currency::new(quote)),
            bid,
            ask,
            30,
            "TEST",
        )
    }

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockRateProvider::new("test");
        let rate = make_test_rate("USD", "EUR", dec!(0.91), dec!(0.93));
        provider.set_rate(rate.clone());

        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());
        let result = provider.get_rate(&pair).await.unwrap();

        assert_eq!(result.pair, pair);
        assert_eq!(result.bid, dec!(0.91));
        assert_eq!(result.ask, dec!(0.93));
    }

    #[tokio::test]
    async fn test_aggregated_provider_median() {
        let p1 = Arc::new(MockRateProvider::new("p1"));
        let p2 = Arc::new(MockRateProvider::new("p2"));
        let p3 = Arc::new(MockRateProvider::new("p3"));

        p1.set_rate(make_test_rate("USD", "EUR", dec!(0.90), dec!(0.92)));
        p2.set_rate(make_test_rate("USD", "EUR", dec!(0.91), dec!(0.93)));
        p3.set_rate(make_test_rate("USD", "EUR", dec!(0.92), dec!(0.94)));

        let aggregated = AggregatedRateProvider::new(vec![p1, p2, p3]);

        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());
        let result = aggregated.get_rate(&pair).await.unwrap();

        // Should return median (middle rate)
        assert_eq!(result.bid, dec!(0.91));
        assert_eq!(result.ask, dec!(0.93));
        assert_eq!(result.source, "AGGREGATED");
    }
}
