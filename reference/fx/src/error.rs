//! FX engine error types.

use atomicsettle_common::{Currency, CurrencyPair};
use thiserror::Error;

/// Errors that can occur in the FX engine.
#[derive(Debug, Error)]
pub enum FxError {
    /// Rate not available for the requested currency pair.
    #[error("Rate not available for {0}")]
    RateNotAvailable(CurrencyPair),

    /// Rate has expired and is no longer valid.
    #[error("Rate expired for {0}")]
    RateExpired(CurrencyPair),

    /// Rate lock not found or invalid.
    #[error("Invalid rate lock: {0}")]
    InvalidRateLock(String),

    /// Rate lock has expired.
    #[error("Rate lock expired: {0}")]
    RateLockExpired(String),

    /// Currency mismatch in conversion.
    #[error("Currency mismatch: expected {expected}, got {actual}")]
    CurrencyMismatch { expected: Currency, actual: Currency },

    /// No rate providers available.
    #[error("No rate providers available")]
    NoProvidersAvailable,

    /// Provider returned an error.
    #[error("Rate provider error: {0}")]
    ProviderError(String),

    /// Rate spread exceeds maximum allowed.
    #[error("Spread {spread_bps} bps exceeds maximum {max_bps} bps for {pair}")]
    SpreadTooWide {
        pair: CurrencyPair,
        spread_bps: u32,
        max_bps: u32,
    },

    /// Rate deviation between providers exceeds threshold.
    #[error("Rate deviation {deviation_bps} bps exceeds threshold for {pair}")]
    RateDeviation {
        pair: CurrencyPair,
        deviation_bps: u32,
    },
}

/// Result type for FX operations.
pub type FxResult<T> = Result<T, FxError>;
