//! AtomicSettle Coordinator
//!
//! The coordinator is the trusted entity that orchestrates settlement between participants.
//! It provides atomicity guarantees by managing locks and executing atomic commits.

pub mod coordinator;
pub mod config;
pub mod participant_manager;
pub mod settlement_processor;
pub mod lock_manager;
pub mod state;
pub mod metrics;

pub use coordinator::Coordinator;
pub use config::CoordinatorConfig;
