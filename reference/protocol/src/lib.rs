//! AtomicSettle Protocol Messages
//!
//! Protocol buffer message types for the AtomicSettle protocol.
//! These types are generated from .proto files and extended with
//! additional Rust implementations.

// TODO: Include generated protobuf code when proto files are added
// include!(concat!(env!("OUT_DIR"), "/atomicsettle.rs"));

pub mod messages;

pub use messages::*;
