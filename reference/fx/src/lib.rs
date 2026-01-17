//! AtomicSettle FX Engine
//!
//! Foreign exchange rate engine for currency conversion and rate management.
//!
//! # Features
//!
//! - Multiple rate provider support with aggregation
//! - Rate caching with configurable TTL
//! - Rate locking for guaranteed conversion rates
//! - Spread calculation and management
//!
//! # Example
//!
//! ```rust,ignore
//! use atomicsettle_fx::{FxEngine, FxEngineConfig};
//! use atomicsettle_common::{Currency, CurrencyPair, Money};
//!
//! let engine = FxEngine::new(FxEngineConfig::default());
//!
//! // Get current rate
//! let rate = engine.get_rate(Currency::usd(), Currency::eur()).await?;
//!
//! // Convert amount
//! let usd = Money::from_str("1000.00", Currency::usd())?;
//! let eur = engine.convert(&usd, Currency::eur()).await?;
//! ```

pub mod engine;
pub mod provider;
pub mod cache;
pub mod conversion;
pub mod rate_lock;
pub mod error;

pub use engine::{FxEngine, FxEngineConfig};
pub use provider::{RateProvider, AggregatedRateProvider};
pub use cache::RateCache;
pub use conversion::Conversion;
pub use rate_lock::{RateLock, RateLockManager};
pub use error::FxError;
