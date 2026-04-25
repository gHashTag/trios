//! Layer 3 — Honey-Merkle append-only log.
//!
//! `root_{n+1} = blake3(root_n || event_bytes)`. Simple, deterministic,
//! reproducible from the `assertions/rainbow_state.jsonl` file.

use crate::event::RainbowEvent;

/// A 32-byte blake3 digest.
pub type MerkleRoot = [u8; 32];

/// Append-only Merkle chain.
#[derive(Debug, Clone, Default)]
pub struct MerkleChain {
    root: MerkleRoot,
    length: usize,
}

impl MerkleChain {
    /// Start from the zero root.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Current Merkle root (32 bytes).
    #[must_use]
    pub fn root(&self) -> MerkleRoot {
        self.root
    }

    /// Current Merkle root as a hex string (convenience for `jsonl` logs).
    #[must_use]
    pub fn root_hex(&self) -> String {
        hex_encode(&self.root)
    }

    /// Number of events absorbed so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Whether no events have been absorbed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Absorb one event. Returns the new root.
    pub fn append(&mut self, ev: &RainbowEvent) -> MerkleRoot {
        let bytes = serde_json::to_vec(ev).expect("RainbowEvent always serialises");
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.root);
        hasher.update(&bytes);
        self.root = *hasher.finalize().as_bytes();
        self.length += 1;
        self.root
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{Channel, Payload};

    fn sample() -> RainbowEvent {
        RainbowEvent {
            lamport: 1,
            agent: "alpha".into(),
            channel: Channel::Red,
            payload: Payload::Claim { lane: "L13".into() },
            ts_unix_s: 0,
            signature: vec![],
        }
    }

    #[test]
    fn empty_starts_at_zero_root() {
        let m = MerkleChain::new();
        assert!(m.is_empty());
        assert_eq!(m.root(), [0u8; 32]);
    }

    #[test]
    fn append_changes_root_deterministically() {
        let mut a = MerkleChain::new();
        let mut b = MerkleChain::new();
        let ev = sample();
        a.append(&ev);
        b.append(&ev);
        assert_eq!(a.root(), b.root());
        assert_ne!(a.root(), [0u8; 32]);
    }
}
