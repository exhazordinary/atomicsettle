//! AtomicSettle Simulator
//!
//! Test environment for banks and developers to test integration.

use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod bank;
mod scenario;
mod controller;
mod metrics;

use controller::SimulationController;
use scenario::Scenario;

/// AtomicSettle Simulator CLI
#[derive(Parser, Debug)]
#[command(name = "simulator")]
#[command(about = "AtomicSettle test and simulation environment")]
struct Args {
    /// Number of simulated banks to create
    #[arg(short, long, default_value = "3")]
    banks: usize,

    /// Scenario to run
    #[arg(short, long)]
    scenario: Option<String>,

    /// Enable web visualizer
    #[arg(long)]
    visualizer: bool,

    /// Visualizer port
    #[arg(long, default_value = "8888")]
    visualizer_port: u16,

    /// Simulation speed multiplier
    #[arg(long, default_value = "1.0")]
    speed: f64,

    /// Random seed for reproducibility
    #[arg(long)]
    seed: Option<u64>,

    /// Run duration in seconds (0 = infinite)
    #[arg(long, default_value = "0")]
    duration: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let args = Args::parse();

    info!("Starting AtomicSettle Simulator");
    info!("Banks: {}", args.banks);
    info!("Speed: {}x", args.speed);

    // Create simulation controller
    let mut controller = SimulationController::new(args.banks, args.speed, args.seed);

    // Initialize simulated banks
    controller.initialize().await?;

    info!("Simulator initialized with {} banks", args.banks);

    // Run scenario if specified
    if let Some(scenario_name) = &args.scenario {
        info!("Running scenario: {}", scenario_name);

        let scenario = Scenario::load(scenario_name)?;
        controller.run_scenario(scenario).await?;
    } else {
        // Interactive mode
        info!("Running in interactive mode");
        info!("Press Ctrl+C to stop");

        // Run until stopped
        let duration = if args.duration > 0 {
            Some(std::time::Duration::from_secs(args.duration))
        } else {
            None
        };

        controller.run(duration).await?;
    }

    // Print metrics
    let metrics = controller.get_metrics();
    info!("Simulation complete");
    info!("Total settlements: {}", metrics.total_settlements);
    info!("Successful: {}", metrics.successful_settlements);
    info!("Failed: {}", metrics.failed_settlements);
    info!("Average latency: {}ms", metrics.average_latency_ms());

    Ok(())
}
