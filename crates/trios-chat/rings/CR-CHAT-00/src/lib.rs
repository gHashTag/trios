//! CR-CHAT-00 — Chat wire-format primitives.
//!
//! Anchor: `phi^2 + phi^-2 = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Bottom-of-graph for `trios-chat`. Pure data + serde. No I/O.
//!
//! Every other CR-CHAT-* and BR-IO-CHAT-* ring imports the types here:
//!
//! * `SessionId` — 32-byte opaque session identity.
//! * `Counter`   — strictly-monotone ratchet counter (R-CHAT-3 forward
//!   secrecy invariant lives downstream in CR-CHAT-02).
//! * `DestHash`  — 16-byte routing hint (R-CHAT-3 sealed sender).
//! * `EnvelopeMeta` — non-secret header travelling alongside ciphertext.
//! * `Error` / `Result` — crate-wide error pair.
//! * [`chat_laws`] — the canonical 12-row R-CHAT law table.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------
// IDs
// ---------------------------------------------------------------------

/// Opaque 32-byte session identity. Two parties holding the same
/// `SessionId` belong to the same chat session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub [u8; 32]);

impl SessionId {
    /// Construct from a 32-byte array.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow as a byte slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hex-encode (lowercase, 64 chars).
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for b in &self.0 {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }
}

/// Strictly-monotone ratchet counter. Wraps a `u64` so the tighter
/// `next()` API is unambiguous.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Counter(pub u64);

impl Counter {
    /// Counter of zero — the start-of-session value.
    pub const ZERO: Counter = Counter(0);

    /// Successor counter. Panics on overflow (matches Coq
    /// `ratchet_no_replay` totality assumption).
    pub fn next(self) -> Self {
        Counter(self.0.checked_add(1).expect("counter overflow"))
    }

    /// Raw value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// 16-byte routing hint stored next to a sealed envelope. Per
/// **R-CHAT-3** the mesh routes on this hash; it MUST NOT leak the
/// recipient's public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DestHash(pub [u8; 16]);

impl DestHash {
    /// Construct from a 16-byte array.
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

// ---------------------------------------------------------------------
// EnvelopeMeta
// ---------------------------------------------------------------------

/// Non-secret header travelling alongside ciphertext on the wire and
/// at rest. Specifically does **not** include sender identity — that's
/// the whole point of sealed-sender (R-CHAT-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeMeta {
    /// Which session the envelope belongs to.
    pub session: SessionId,
    /// Strictly-monotone ratchet position.
    pub counter: Counter,
    /// Routing hint (16 B SHA-256 prefix of recipient's static key).
    pub dest: DestHash,
    /// Length of the padding class (R-CHAT-9 fixed-size buckets).
    pub padded_len: u32,
}

// ---------------------------------------------------------------------
// Error / Result
// ---------------------------------------------------------------------

/// Crate-wide error enum.
#[derive(Debug, Error)]
pub enum Error {
    /// A protocol invariant was violated (replay, fork, wrong epoch, …).
    #[error("invariant violated: {0}")]
    Invariant(&'static str),

    /// AEAD failure (tampered ciphertext, wrong key, etc.).
    #[error("aead: decryption failed")]
    Aead,

    /// Persistence-layer failure (only emitted from CR-CHAT-05 and
    /// BR-IO-CHAT-*; CR-CHAT-00 just defines the shape).
    #[error("persist: {0}")]
    Persist(String),

    /// Wire-format failure (serde, length, etc.).
    #[error("wire: {0}")]
    Wire(&'static str),
}

/// Crate-wide `Result` shorthand.
pub type Result<T> = core::result::Result<T, Error>;

// ---------------------------------------------------------------------
// R-CHAT law table
// ---------------------------------------------------------------------

/// One R-CHAT law: stable id, short title, and a compact summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatLaw {
    /// Stable id, e.g. `"R-CHAT-1"`.
    pub id: &'static str,
    /// Short slogan-style title.
    pub title: &'static str,
    /// One-line plain-language summary.
    pub summary: &'static str,
}

/// The 12 canonical Trinity Secure Chat laws. Adding / removing a law
/// is a cross-repo wire-format break (see `AGENTS.md`).
pub const fn chat_laws() -> &'static [ChatLaw] {
    &[
        ChatLaw { id: "R-CHAT-1",  title: "NO PLAINTEXT AT REST",
                  summary: "Persistence layers only ever see sealed envelopes." },
        ChatLaw { id: "R-CHAT-2",  title: "HYBRID PQ",
                  summary: "Every key agreement combines X25519 with a PQ KEM (ML-KEM-768)." },
        ChatLaw { id: "R-CHAT-3",  title: "SEALED SENDER",
                  summary: "Routing happens on dest_hash; sender identity is encrypted." },
        ChatLaw { id: "R-CHAT-4",  title: "DENIABLE AUTH",
                  summary: "Authentication uses MAC-then-encrypt, leaving no signature trail." },
        ChatLaw { id: "R-CHAT-5",  title: "AGENT KEY != USER KEY",
                  summary: "Bot identities live on a disjoint keyring from human users." },
        ChatLaw { id: "R-CHAT-6",  title: "TOOLS ARE SIGNED PROMPTS",
                  summary: "An agent only invokes a tool whose payload was signed by the publisher." },
        ChatLaw { id: "R-CHAT-7",  title: "DUAL-LLM ISOLATION",
                  summary: "Untrusted text crosses a sandbox before reaching the action LLM." },
        ChatLaw { id: "R-CHAT-8",  title: "SESSION-SCOPED CAPABILITY",
                  summary: "Every action token is bound to one session and one verb." },
        ChatLaw { id: "R-CHAT-9",  title: "FIXED-SIZE PADDING",
                  summary: "Every envelope is padded to one of a small set of length classes." },
        ChatLaw { id: "R-CHAT-10", title: "ZERO BACKGROUND CHATTER",
                  summary: "No background pings, presence, or read-receipts; the wire is silent." },
        ChatLaw { id: "R-CHAT-11", title: "COQ-VERIFIED INVARIANTS",
                  summary: "Every wire invariant has a Coq theorem (Defined or budgeted Admitted)." },
        ChatLaw { id: "R-CHAT-12", title: "R5+R7 (HONESTY MODE)",
                  summary: "Every claim is tagged [VERIFIED] / [CITED] / [DERIVED] / [ASPIRATIONAL]." },
    ]
}

// ---------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_hex_roundtrip() {
        let s = SessionId::new([0xAB; 32]);
        let h = s.to_hex();
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(&h[..2], "ab");
    }

    #[test]
    fn counter_zero_and_next() {
        assert_eq!(Counter::ZERO.get(), 0);
        assert_eq!(Counter::ZERO.next(), Counter(1));
        assert_eq!(Counter(7).next(), Counter(8));
    }

    #[test]
    #[should_panic(expected = "counter overflow")]
    fn counter_overflow_panics() {
        let _ = Counter(u64::MAX).next();
    }

    #[test]
    fn dest_hash_size_is_16() {
        let d = DestHash::new([0u8; 16]);
        assert_eq!(d.0.len(), 16);
    }

    #[test]
    fn envelope_meta_serde_roundtrip() {
        let m = EnvelopeMeta {
            session: SessionId::new([1u8; 32]),
            counter: Counter(42),
            dest: DestHash::new([2u8; 16]),
            padded_len: 1024,
        };
        let j = serde_json::to_string(&m).unwrap();
        let back: EnvelopeMeta = serde_json::from_str(&j).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn error_invariant_renders() {
        let e = Error::Invariant("fixture");
        assert_eq!(format!("{e}"), "invariant violated: fixture");
    }

    #[test]
    fn error_aead_renders() {
        assert_eq!(format!("{}", Error::Aead), "aead: decryption failed");
    }

    #[test]
    fn law_table_has_exactly_twelve_rows() {
        assert_eq!(chat_laws().len(), 12);
    }

    #[test]
    fn law_ids_are_canonical_and_unique() {
        let laws = chat_laws();
        for (i, l) in laws.iter().enumerate() {
            assert_eq!(l.id, format!("R-CHAT-{}", i + 1));
        }
        let ids: std::collections::HashSet<&str> = laws.iter().map(|l| l.id).collect();
        assert_eq!(ids.len(), laws.len());
    }
}
