//! AtomicSettle Participant Library
//!
//! This library provides the client interface for banks and financial institutions
//! to connect to the AtomicSettle network and send/receive settlements.

pub mod client;
pub mod config;
pub mod connection;
pub mod handler;

pub use client::ParticipantClient;
pub use config::ParticipantConfig;
