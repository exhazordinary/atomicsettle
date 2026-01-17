//! AtomicSettle Coordinator Binary
//!
//! The coordinator node orchestrates cross-border settlements between participants.

use std::sync::Arc;

use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use atomicsettle_coordinator::{Coordinator, CoordinatorConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Starting AtomicSettle Coordinator");

    // Load configuration
    let config = CoordinatorConfig::from_env();
    if let Err(e) = config.validate() {
        error!(error = %e, "Invalid configuration");
        return Err(anyhow::anyhow!("Configuration error: {}", e));
    }

    // Generate node ID if not provided
    let node_id = config
        .node_id
        .clone()
        .unwrap_or_else(|| format!("coordinator-{}", uuid::Uuid::new_v4()));

    info!(node_id = %node_id, "Node ID assigned");

    // Create coordinator
    let coordinator = Arc::new(Coordinator::new(config.clone(), node_id.clone()));

    // Set up graceful shutdown
    let coordinator_clone = coordinator.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Shutdown signal received");
        if let Err(e) = coordinator_clone.stop().await {
            error!(error = %e, "Error during shutdown");
        }
    });

    // Start coordinator
    coordinator.start().await?;

    info!(
        node_id = %node_id,
        listen_addr = %config.listen_addr,
        listen_port = %config.listen_port,
        "Coordinator running"
    );

    // Keep running until shutdown
    loop {
        if !coordinator.is_accepting_requests() {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    info!("Coordinator shutdown complete");
    Ok(())
}
