//! Main FX engine implementation.

use std::sync::Arc;

use atomicsettle_common::{Currency, CurrencyPair, FxRate, Money};
use chrono::Duration;
use tracing::{debug, info, instrument};
use uuid::Uuid;

use crate::cache::{RateCache, RateCacheConfig};
use crate::conversion::{Conversion, ConversionRequest, RateSide};
use crate::error::{FxError, FxResult};
use crate::provider::RateProvider;
use crate::rate_lock::{RateLock, RateLockConfig, RateLockManager};

/// Configuration for the FX engine.
#[derive(Debug, Clone)]
pub struct FxEngineConfig {
    /// Cache configuration.
    pub cache: RateCacheConfig,
    /// Rate lock configuration.
    pub rate_lock: RateLockConfig,
    /// Maximum allowed spread in basis points.
    pub max_spread_bps: u32,
    /// Whether to use cached rates.
    pub use_cache: bool,
}

impl Default for FxEngineConfig {
    fn default() -> Self {
        Self {
            cache: RateCacheConfig::default(),
            rate_lock: RateLockConfig::default(),
            max_spread_bps: 200, // 2% max spread
            use_cache: true,
        }
    }
}

/// The main FX engine.
pub struct FxEngine {
    provider: Arc<dyn RateProvider>,
    cache: RateCache,
    lock_manager: RateLockManager,
    config: FxEngineConfig,
}

impl FxEngine {
    /// Create a new FX engine with the given provider.
    pub fn new(provider: Arc<dyn RateProvider>, config: FxEngineConfig) -> Self {
        Self {
            provider,
            cache: RateCache::with_config(config.cache.clone()),
            lock_manager: RateLockManager::with_config(config.rate_lock.clone()),
            config,
        }
    }

    /// Get the current rate for a currency pair.
    #[instrument(skip(self), fields(pair = %pair))]
    pub async fn get_rate(&self, pair: &CurrencyPair) -> FxResult<FxRate> {
        // Check cache first
        if self.config.use_cache {
            if let Some(cached) = self.cache.get(pair) {
                debug!("Using cached rate");
                return Ok(cached);
            }
        }

        // Fetch from provider
        let rate = self.provider.get_rate(pair).await?;

        // Validate spread
        self.validate_spread(&rate)?;

        // Cache the rate
        if self.config.use_cache {
            self.cache.insert(rate.clone());
        }

        Ok(rate)
    }

    /// Get rate between two currencies.
    pub async fn get_rate_for(
        &self,
        from: Currency,
        to: Currency,
    ) -> FxResult<FxRate> {
        let pair = CurrencyPair::new(from, to);
        self.get_rate(&pair).await
    }

    /// Convert an amount to another currency.
    #[instrument(skip(self), fields(
        from_currency = %request.amount.currency,
        to_currency = %request.target_currency,
        amount = %request.amount.value
    ))]
    pub async fn convert(&self, request: ConversionRequest) -> FxResult<Conversion> {
        let pair = CurrencyPair::new(
            request.amount.currency.clone(),
            request.target_currency.clone(),
        );

        // Get rate (from lock or fresh)
        let (rate, lock_id) = if let Some(lock) = request.rate_lock {
            let used_lock = self.lock_manager.use_lock(lock.id)?;
            (used_lock.rate, Some(lock.id))
        } else {
            (self.get_rate(&pair).await?, None)
        };

        // Validate currencies
        if request.amount.currency != rate.pair.base {
            return Err(FxError::CurrencyMismatch {
                expected: rate.pair.base.clone(),
                actual: request.amount.currency.clone(),
            });
        }

        // Calculate output
        let conversion_rate = request.rate_side.get_rate(&rate);
        let output_value = (request.amount.value * conversion_rate).round_dp(
            request.target_currency.decimal_places(),
        );
        let output = Money::new(output_value, request.target_currency);

        let conversion = Conversion::new(request.amount, output, rate, lock_id);

        info!(
            conversion_id = %conversion.id,
            effective_rate = %conversion.effective_rate(),
            "Conversion completed"
        );

        Ok(conversion)
    }

    /// Simple conversion using mid-market rate.
    pub async fn convert_simple(
        &self,
        amount: &Money,
        to: Currency,
    ) -> FxResult<Money> {
        let request = ConversionRequest::new(amount.clone(), to);
        let conversion = self.convert(request).await?;
        Ok(conversion.output)
    }

    /// Create a rate lock for guaranteed conversion.
    #[instrument(skip(self))]
    pub async fn create_rate_lock(
        &self,
        pair: &CurrencyPair,
        duration: Option<Duration>,
        participant_id: String,
    ) -> FxResult<RateLock> {
        let rate = self.get_rate(pair).await?;
        let lock = self.lock_manager.create_lock(rate, duration, participant_id)?;

        info!(
            lock_id = %lock.id,
            pair = %pair,
            expires_at = %lock.expires_at,
            "Created rate lock"
        );

        Ok(lock)
    }

    /// Get a rate lock by ID.
    pub fn get_rate_lock(&self, lock_id: Uuid) -> Option<RateLock> {
        self.lock_manager.get_lock(lock_id)
    }

    /// Cancel a rate lock.
    pub fn cancel_rate_lock(&self, lock_id: Uuid, participant_id: &str) -> FxResult<()> {
        self.lock_manager.cancel_lock(lock_id, participant_id)
    }

    /// Get all supported currency pairs.
    pub fn supported_pairs(&self) -> Vec<CurrencyPair> {
        self.provider.supported_pairs()
    }

    /// Check if a currency pair is supported.
    pub fn supports_pair(&self, pair: &CurrencyPair) -> bool {
        self.provider.supports_pair(pair)
    }

    /// Get engine statistics.
    pub fn stats(&self) -> FxEngineStats {
        FxEngineStats {
            cache_stats: self.cache.stats(),
            lock_stats: self.lock_manager.stats(),
        }
    }

    /// Clean up expired caches and locks.
    pub fn cleanup(&self) {
        self.cache.evict_expired();
        self.lock_manager.cleanup_expired();
    }

    /// Validate that spread is within acceptable limits.
    fn validate_spread(&self, rate: &FxRate) -> FxResult<()> {
        let spread_bps = rate.spread_bps();
        let spread_u32 = spread_bps.trunc().to_string().parse::<u32>().unwrap_or(0);

        if spread_u32 > self.config.max_spread_bps {
            return Err(FxError::SpreadTooWide {
                pair: rate.pair.clone(),
                spread_bps: spread_u32,
                max_bps: self.config.max_spread_bps,
            });
        }

        Ok(())
    }
}

/// Engine statistics.
#[derive(Debug, Clone)]
pub struct FxEngineStats {
    pub cache_stats: crate::cache::CacheStats,
    pub lock_stats: crate::rate_lock::RateLockStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MockRateProvider;
    use rust_decimal_macros::dec;

    fn setup_engine() -> FxEngine {
        let provider = Arc::new(MockRateProvider::new("test"));

        // Add some test rates
        provider.set_rate(FxRate::new(
            CurrencyPair::new(Currency::usd(), Currency::eur()),
            dec!(0.91),
            dec!(0.93),
            30,
            "TEST",
        ));

        provider.set_rate(FxRate::new(
            CurrencyPair::new(Currency::gbp(), Currency::usd()),
            dec!(1.26),
            dec!(1.28),
            30,
            "TEST",
        ));

        let config = FxEngineConfig {
            max_spread_bps: 300, // Allow wider spread for test data
            ..Default::default()
        };
        FxEngine::new(provider, config)
    }

    #[tokio::test]
    async fn test_get_rate() {
        let engine = setup_engine();
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());

        let rate = engine.get_rate(&pair).await.unwrap();

        assert_eq!(rate.pair, pair);
        assert_eq!(rate.bid, dec!(0.91));
        assert_eq!(rate.ask, dec!(0.93));
    }

    #[tokio::test]
    async fn test_convert_simple() {
        let engine = setup_engine();
        let usd = Money::new(dec!(1000), Currency::usd());

        let eur = engine.convert_simple(&usd, Currency::eur()).await.unwrap();

        assert_eq!(eur.currency, Currency::eur());
        // Mid rate is 0.92, so 1000 USD = 920 EUR
        assert_eq!(eur.value, dec!(920));
    }

    #[tokio::test]
    async fn test_convert_with_rate_lock() {
        let engine = setup_engine();
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());

        // Create a lock
        let lock = engine
            .create_rate_lock(&pair, None, "BANK_A".to_string())
            .await
            .unwrap();

        // Convert using the lock
        let request = ConversionRequest::new(Money::new(dec!(1000), Currency::usd()), Currency::eur())
            .with_rate_lock(lock.clone());

        let conversion = engine.convert(request).await.unwrap();

        assert_eq!(conversion.rate_lock_id, Some(lock.id));
        assert_eq!(conversion.output.currency, Currency::eur());
    }

    #[tokio::test]
    async fn test_rate_not_available() {
        let engine = setup_engine();
        let pair = CurrencyPair::new(Currency::new("XYZ"), Currency::new("ABC"));

        let result = engine.get_rate(&pair).await;

        assert!(matches!(result, Err(FxError::RateNotAvailable(_))));
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let engine = setup_engine();
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());

        // First call fetches from provider
        let rate1 = engine.get_rate(&pair).await.unwrap();

        // Second call should hit cache
        let rate2 = engine.get_rate(&pair).await.unwrap();

        assert_eq!(rate1.mid, rate2.mid);

        // Verify cache has entry
        assert_eq!(engine.cache.len(), 1);
    }

    #[tokio::test]
    async fn test_spread_validation() {
        let provider = Arc::new(MockRateProvider::new("test"));

        // Add rate with wide spread (10%)
        provider.set_rate(FxRate::new(
            CurrencyPair::new(Currency::usd(), Currency::eur()),
            dec!(0.85),
            dec!(0.95),
            30,
            "TEST",
        ));

        let config = FxEngineConfig {
            max_spread_bps: 200, // 2% max
            ..Default::default()
        };

        let engine = FxEngine::new(provider, config);
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());

        let result = engine.get_rate(&pair).await;

        assert!(matches!(result, Err(FxError::SpreadTooWide { .. })));
    }
}
