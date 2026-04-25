//! Colour channels (ROY G BIV) and the payload-to-channel total function.
//!
//! Exactly seven variants — enforced by a `match` with no wildcard arm
//! and by the Coq lemma `seven_channels_total` in
//! `trinity-clara/proofs/igla/rainbow_bridge_consistency.v`.

use serde::{Deserialize, Serialize};

/// The seven colour channels of the Rainbow Bridge, in ROY G BIV order.
///
/// Adding an eighth variant is forbidden (R7 + ONE SHOT §8); it would
/// require updating `rainbow_bridge_consistency.v::seven_channels_total`
/// and the INV-8 JSON entry in lockstep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    /// 🔴 RED — lane claim.
    Red,
    /// 🟠 ORANGE — heartbeat on a WORKing lane.
    Orange,
    /// 🟡 YELLOW — DONE announcement.
    Yellow,
    /// 🟢 GREEN — honey deposit (MUST be ed25519-signed).
    Green,
    /// 🔵 BLUE — CRDT state delta.
    Blue,
    /// 🟣 INDIGO — observed R-rule violation.
    Indigo,
    /// 🟪 VIOLET — global VICTORY (BPB < 1.50 × 3 seeds).
    Violet,
}

impl Channel {
    /// Canonical ROY G BIV ordering. Kept as an explicit array (not derived
    /// from the enum) so the length check in tests is independent of any
    /// `strum`-style macro and mirrors the Coq `all_channels` list directly.
    pub const ALL: [Channel; super::CHANNEL_COUNT] = [
        Channel::Red,
        Channel::Orange,
        Channel::Yellow,
        Channel::Green,
        Channel::Blue,
        Channel::Indigo,
        Channel::Violet,
    ];
}

/// The seven payload variants. Each one has exactly one legal channel
/// via [`channel_of_payload`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    /// Lane claim (🔴 red).
    Claim {
        /// Lane identifier, e.g. `"L13"`.
        lane: String,
    },
    /// Heartbeat (🟠 orange).
    Heartbeat {
        /// Lane the heartbeat refers to.
        lane: String,
    },
    /// Lane DONE (🟡 yellow).
    Done {
        /// Lane identifier.
        lane: String,
        /// Commit SHA on `main`.
        sha: String,
    },
    /// Honey deposit (🟢 green). MUST be signed.
    Honey {
        /// Honey jar line, free-form JSON string.
        line: String,
    },
    /// CRDT delta (🔵 blue).
    State {
        /// Opaque CRDT delta bytes (automerge, base64-encoded).
        delta_b64: String,
    },
    /// R-rule violation (🟣 indigo).
    Violation {
        /// R-rule identifier, e.g. `"R7"`.
        rule: String,
        /// Free-form description.
        detail: String,
    },
    /// Global VICTORY (🟪 violet).
    Victory {
        /// Final BPB value that triggered victory.
        bpb: f64,
        /// Number of distinct passing seeds (≥ 3).
        seeds_distinct: usize,
    },
}

/// Total function: every payload has exactly one legal channel.
///
/// Coq: `rainbow_bridge_consistency.v::channel_of_payload_total`.
#[must_use]
pub fn channel_of_payload(p: &Payload) -> Channel {
    match p {
        Payload::Claim { .. } => Channel::Red,
        Payload::Heartbeat { .. } => Channel::Orange,
        Payload::Done { .. } => Channel::Yellow,
        Payload::Honey { .. } => Channel::Green,
        Payload::State { .. } => Channel::Blue,
        Payload::Violation { .. } => Channel::Indigo,
        Payload::Victory { .. } => Channel::Violet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_seven_channels() {
        // Mirror: rainbow_bridge_consistency.v::seven_channels_total.
        assert_eq!(Channel::ALL.len(), super::super::CHANNEL_COUNT);
        assert_eq!(Channel::ALL.len(), 7);
    }

    #[test]
    fn channel_of_payload_is_total_and_injective_on_colour() {
        let samples = [
            Payload::Claim { lane: "L13".into() },
            Payload::Heartbeat { lane: "L13".into() },
            Payload::Done { lane: "L13".into(), sha: "abc".into() },
            Payload::Honey { line: "{}".into() },
            Payload::State { delta_b64: "".into() },
            Payload::Violation { rule: "R7".into(), detail: "".into() },
            Payload::Victory { bpb: 1.49, seeds_distinct: 3 },
        ];
        let channels: Vec<Channel> = samples.iter().map(channel_of_payload).collect();
        assert_eq!(channels, Channel::ALL.to_vec());
    }
}
