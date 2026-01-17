//! FX rate caching with TTL support.

use atomicsettle_common::{CurrencyPair, FxRate};
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tracing::debug;

/// Cached rate entry.
#[derive(Debug, Clone)]
struct CacheEntry {
    rate: FxRate,
    cached_at: DateTime<Utc>,
    ttl: Duration,
}

impl CacheEntry {
    fn new(rate: FxRate, ttl: Duration) -> Self {
        Self {
            rate,
            cached_at: Utc::now(),
            ttl,
        }
    }

    fn is_valid(&self) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.cached_at);

        // Valid if within TTL and rate hasn't expired
        age < self.ttl && self.rate.is_valid()
    }
}

/// Configuration for rate cache.
#[derive(Debug, Clone)]
pub struct RateCacheConfig {
    /// Default TTL for cached rates.
    pub default_ttl: Duration,
    /// Maximum number of entries.
    pub max_entries: usize,
    /// Whether to auto-clean expired entries.
    pub auto_clean: bool,
}

impl Default for RateCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::milliseconds(100), // 100ms default for production
            max_entries: 10000,
            auto_clean: true,
        }
    }
}

/// Thread-safe rate cache with TTL.
pub struct RateCache {
    cache: DashMap<String, CacheEntry>,
    config: RateCacheConfig,
}

impl RateCache {
    /// Create a new rate cache with default configuration.
    pub fn new() -> Self {
        Self::with_config(RateCacheConfig::default())
    }

    /// Create a new rate cache with custom configuration.
    pub fn with_config(config: RateCacheConfig) -> Self {
        Self {
            cache: DashMap::new(),
            config,
        }
    }

    /// Get a rate from cache if valid.
    pub fn get(&self, pair: &CurrencyPair) -> Option<FxRate> {
        let key = Self::cache_key(pair);

        if let Some(entry) = self.cache.get(&key) {
            if entry.is_valid() {
                debug!(pair = %pair, "Cache hit");
                return Some(entry.rate.clone());
            } else {
                debug!(pair = %pair, "Cache entry expired");
                // Remove expired entry
                drop(entry);
                self.cache.remove(&key);
            }
        }

        debug!(pair = %pair, "Cache miss");
        None
    }

    /// Insert a rate into cache.
    pub fn insert(&self, rate: FxRate) {
        self.insert_with_ttl(rate, self.config.default_ttl);
    }

    /// Insert a rate with custom TTL.
    pub fn insert_with_ttl(&self, rate: FxRate, ttl: Duration) {
        let key = Self::cache_key(&rate.pair);

        // Check capacity
        if self.cache.len() >= self.config.max_entries {
            self.evict_expired();
        }

        let entry = CacheEntry::new(rate, ttl);
        self.cache.insert(key, entry);
    }

    /// Remove a rate from cache.
    pub fn remove(&self, pair: &CurrencyPair) {
        let key = Self::cache_key(pair);
        self.cache.remove(&key);
    }

    /// Clear all cached rates.
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get the number of entries in cache.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Evict expired entries.
    pub fn evict_expired(&self) {
        self.cache.retain(|_, entry| entry.is_valid());
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let total = self.cache.len();
        let valid = self.cache.iter().filter(|e| e.is_valid()).count();

        CacheStats {
            total_entries: total,
            valid_entries: valid,
            expired_entries: total - valid,
        }
    }

    fn cache_key(pair: &CurrencyPair) -> String {
        format!("{}/{}", pair.base.code(), pair.quote.code())
    }
}

impl Default for RateCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub expired_entries: usize,
}

/// Shared rate cache.
pub type SharedRateCache = Arc<RateCache>;

#[cfg(test)]
mod tests {
    use super::*;
    use atomicsettle_common::Currency;
    use rust_decimal_macros::dec;
    use std::thread::sleep;
    use std::time::Duration as StdDuration;

    fn make_rate(base: &str, quote: &str) -> FxRate {
        FxRate::new(
            CurrencyPair::new(Currency::new(base), Currency::new(quote)),
            dec!(0.91),
            dec!(0.93),
            30,
            "TEST",
        )
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = RateCache::new();
        let rate = make_rate("USD", "EUR");
        let pair = rate.pair.clone();

        cache.insert(rate.clone());

        let cached = cache.get(&pair).unwrap();
        assert_eq!(cached.pair, pair);
        assert_eq!(cached.mid, rate.mid);
    }

    #[test]
    fn test_cache_miss() {
        let cache = RateCache::new();
        let pair = CurrencyPair::new(Currency::usd(), Currency::eur());

        assert!(cache.get(&pair).is_none());
    }

    #[test]
    fn test_cache_expiry() {
        let config = RateCacheConfig {
            default_ttl: Duration::milliseconds(50),
            ..Default::default()
        };
        let cache = RateCache::with_config(config);
        let rate = make_rate("USD", "EUR");
        let pair = rate.pair.clone();

        cache.insert(rate);

        // Should be valid immediately
        assert!(cache.get(&pair).is_some());

        // Wait for expiry
        sleep(StdDuration::from_millis(60));

        // Should be expired now
        assert!(cache.get(&pair).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = RateCache::new();
        cache.insert(make_rate("USD", "EUR"));
        cache.insert(make_rate("GBP", "USD"));

        assert_eq!(cache.len(), 2);

        cache.clear();

        assert_eq!(cache.len(), 0);
    }
}
