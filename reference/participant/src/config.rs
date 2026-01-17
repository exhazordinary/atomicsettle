//! Participant client configuration.

use std::time::Duration;

/// Configuration for participant client.
#[derive(Debug, Clone)]
pub struct ParticipantConfig {
    /// URL of the coordinator to connect to.
    pub coordinator_url: String,
    /// Protocol version to use.
    pub protocol_version: String,
    /// Path to client certificate.
    pub cert_path: Option<String>,
    /// Path to client private key.
    pub key_path: Option<String>,
    /// Path to CA certificate for coordinator.
    pub ca_cert_path: Option<String>,
    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Connection timeout.
    pub connection_timeout: Duration,
    /// Request timeout.
    pub request_timeout: Duration,
    /// Reconnection settings.
    pub reconnect_enabled: bool,
    /// Maximum reconnection attempts.
    pub max_reconnect_attempts: usize,
    /// Initial reconnection delay.
    pub reconnect_delay: Duration,
    /// Maximum reconnection delay.
    pub max_reconnect_delay: Duration,
}

impl Default for ParticipantConfig {
    fn default() -> Self {
        Self {
            coordinator_url: "https://coordinator.atomicsettle.local:8080".to_string(),
            protocol_version: "1.0".to_string(),
            cert_path: None,
            key_path: None,
            ca_cert_path: None,
            heartbeat_interval: Duration::from_secs(5),
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            reconnect_enabled: true,
            max_reconnect_attempts: 10,
            reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(30),
        }
    }
}

impl ParticipantConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("COORDINATOR_URL") {
            config.coordinator_url = url;
        }

        if let Ok(cert) = std::env::var("CLIENT_CERT_PATH") {
            config.cert_path = Some(cert);
        }

        if let Ok(key) = std::env::var("CLIENT_KEY_PATH") {
            config.key_path = Some(key);
        }

        if let Ok(ca) = std::env::var("CA_CERT_PATH") {
            config.ca_cert_path = Some(ca);
        }

        config
    }

    /// Validate configuration.
    pub fn validate(&self) -> Result<(), String> {
        if self.coordinator_url.is_empty() {
            return Err("Coordinator URL cannot be empty".to_string());
        }

        if self.heartbeat_interval.is_zero() {
            return Err("Heartbeat interval cannot be zero".to_string());
        }

        Ok(())
    }
}
