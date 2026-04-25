//! The canonical bridge event type.

use serde::{Deserialize, Serialize};

use crate::channel::{Channel, Payload};
use crate::subscriber::SubscriberId;

/// An event published onto the Rainbow Bridge.
///
/// Honey-channel events (`channel == Channel::Green`) MUST carry a non-empty
/// `signature` (ed25519-over-payload-bytes). All other channels may omit it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RainbowEvent {
    /// Per-agent Lamport clock value. Must strictly advance per agent.
    pub lamport: u64,
    /// Emitting agent.
    pub agent: SubscriberId,
    /// Colour of the event. Must equal `channel_of_payload(&payload)`.
    pub channel: Channel,
    /// Typed payload.
    pub payload: Payload,
    /// Wall-clock emission timestamp (seconds since Unix epoch). Used ONLY
    /// for the heartbeat-staleness gate; never for event ordering.
    pub ts_unix_s: u64,
    /// ed25519 signature over the serialized payload bytes. Required only
    /// on the honey (green) channel.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signature: Vec<u8>,
}

impl RainbowEvent {
    /// Whether this event is considered signed (non-empty signature).
    #[must_use]
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }
}
