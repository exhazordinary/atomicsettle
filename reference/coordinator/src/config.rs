//! Coordinator configuration.

use std::time::Duration;

/// Lock configuration.
#[derive(Debug, Clone)]
pub struct LockConfig {
    /// Default lock duration.
    pub default_duration: Duration,
    /// Maximum lock duration.
    pub max_duration: Duration,
    /// Lock cleanup interval.
    pub cleanup_interval: Duration,
    /// Maximum concurrent locks per participant.
    pub max_concurrent_per_participant: usize,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self {
            default_duration: Duration::from_secs(30),
            max_duration: Duration::from_secs(60),
            cleanup_interval: Duration::from_secs(1),
            max_concurrent_per_participant: 1000,
        }
    }
}

/// Participant configuration.
#[derive(Debug, Clone)]
pub struct ParticipantConfig {
    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Heartbeat timeout (mark offline after this).
    pub heartbeat_timeout: Duration,
    /// Maximum reconnection attempts.
    pub max_reconnect_attempts: usize,
    /// Reconnection backoff base.
    pub reconnect_backoff_base: Duration,
}

impl Default for ParticipantConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            heartbeat_timeout: Duration::from_secs(15),
            max_reconnect_attempts: 10,
            reconnect_backoff_base: Duration::from_secs(1),
        }
    }
}

/// Settlement processing configuration.
#[derive(Debug, Clone)]
pub struct SettlementConfig {
    /// Maximum concurrent settlements.
    pub max_concurrent: usize,
    /// Validation timeout.
    pub validation_timeout: Duration,
    /// Lock acquisition timeout.
    pub lock_acquisition_timeout: Duration,
    /// Commit timeout.
    pub commit_timeout: Duration,
    /// Acknowledgment timeout.
    pub ack_timeout: Duration,
    /// Enable netting.
    pub netting_enabled: bool,
    /// Netting window duration.
    pub netting_window: Duration,
}

impl Default for SettlementConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10000,
            validation_timeout: Duration::from_millis(500),
            lock_acquisition_timeout: Duration::from_secs(10),
            commit_timeout: Duration::from_millis(200),
            ack_timeout: Duration::from_secs(60),
            netting_enabled: true,
            netting_window: Duration::from_millis(100),
        }
    }
}

/// TLS configuration.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to server certificate.
    pub cert_path: String,
    /// Path to server private key.
    pub key_path: String,
    /// Path to client CA certificate.
    pub client_ca_path: String,
    /// Enable client certificate verification.
    pub require_client_cert: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_path: "/etc/atomicsettle/server.crt".to_string(),
            key_path: "/etc/atomicsettle/server.key".to_string(),
            client_ca_path: "/etc/atomicsettle/client-ca.crt".to_string(),
            require_client_cert: true,
        }
    }
}

/// Main coordinator configuration.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Node ID (must be unique within cluster).
    pub node_id: Option<String>,
    /// Listen address.
    pub listen_addr: String,
    /// Listen port.
    pub listen_port: u16,
    /// Database URL.
    pub database_url: String,
    /// Lock configuration.
    pub lock_config: LockConfig,
    /// Participant configuration.
    pub participant_config: ParticipantConfig,
    /// Settlement configuration.
    pub settlement_config: SettlementConfig,
    /// TLS configuration.
    pub tls_config: Option<TlsConfig>,
    /// Enable metrics endpoint.
    pub metrics_enabled: bool,
    /// Metrics port.
    pub metrics_port: u16,
    /// Log level.
    pub log_level: String,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            node_id: None,
            listen_addr: "0.0.0.0".to_string(),
            listen_port: 8080,
            database_url: "postgres://localhost/atomicsettle".to_string(),
            lock_config: LockConfig::default(),
            participant_config: ParticipantConfig::default(),
            settlement_config: SettlementConfig::default(),
            tls_config: None,
            metrics_enabled: true,
            metrics_port: 9090,
            log_level: "info".to_string(),
        }
    }
}

impl CoordinatorConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(addr) = std::env::var("COORDINATOR_LISTEN_ADDR") {
            config.listen_addr = addr;
        }

        if let Ok(port) = std::env::var("COORDINATOR_LISTEN_PORT") {
            if let Ok(port) = port.parse() {
                config.listen_port = port;
            }
        }

        if let Ok(url) = std::env::var("DATABASE_URL") {
            config.database_url = url;
        }

        if let Ok(level) = std::env::var("LOG_LEVEL") {
            config.log_level = level;
        }

        config
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.listen_port == 0 {
            return Err("Listen port cannot be 0".to_string());
        }

        if self.database_url.is_empty() {
            return Err("Database URL cannot be empty".to_string());
        }

        if self.lock_config.default_duration > self.lock_config.max_duration {
            return Err("Default lock duration cannot exceed max duration".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CoordinatorConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_config() {
        let mut config = CoordinatorConfig::default();
        config.listen_port = 0;
        assert!(config.validate().is_err());
    }
}
