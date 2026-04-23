//! Comet Bridge Module
//!
//! Provides WebSocket bridge between Chrome extension and trios-server (port 9005).
//! Handles EnvelopeT message processing and Chrome runtime messaging.

pub mod comet;
pub mod types;

pub use types::{Envelope, Payload};
