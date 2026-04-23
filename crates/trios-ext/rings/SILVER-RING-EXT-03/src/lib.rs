//! SILVER-RING-EXT-03 — Comet Bridge / Types
//!
//! Provides WebSocket bridge between Chrome extension and trios-server (port 9005).
//! Handles EnvelopeT message processing and Chrome runtime messaging.
//! Depends on SILVER-RING-EXT-01 (DOM bridge) for UI updates.

pub mod comet;
pub mod types;

pub use comet::{
    comet_bridge_connect, comet_bridge_disconnect, comet_bridge_init, comet_is_connected,
    comet_send_chat, CometBridge,
};
pub use types::{Envelope, Payload};
