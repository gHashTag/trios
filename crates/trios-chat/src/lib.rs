//! # trios-chat — Trinity Secure Chat
//!
//! Privacy-first chat for users ↔ agent bots over `trios-mesh-node`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Parent EPIC: trinity-fpga#28
//! Builds on:   trinity-fpga#22 (✅ closed) · trios#629 (LANDED)
//!
//! ## Modules
//! | Module | Lane | Purpose |
//! |--------|------|---------|
//! | `identity` | L-CHAT-1 | Ed25519 + X25519 + ML-KEM-768 prekey bundle |
//! | `ratchet`  | L-CHAT-2 | Triple Ratchet (PQ-FS + PQ-PCS) — skeleton |
//! | `sealed`   | L-CHAT-4 | Sealed-sender envelope over mesh |
//! | `capability` | L-CHAT-6 | Capability tokens + signed tool manifests |
//! | `injection` | L-CHAT-6 | Dual-LLM filter + output validator (anti-prompt-injection) |
//! | `padding`  | L-CHAT-7 | Fixed-size padding classes |
//! | `r_chat`   | LAWS | R-CHAT-1..R-CHAT-12 constitutional laws |
//!
//! ## Honesty (R5)
//!
//! Every public function carries a doc-tag of its current state:
//! `[VERIFIED]` — has tests passing
//! `[DERIVED]`  — derived from another verified module
//! `[ASPIRATIONAL]` — skeleton only, lane in progress
//! `[CITED]` — implements a published spec; cite in doc

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod identity;
pub mod ratchet;
pub mod sealed;
pub mod capability;
pub mod injection;
pub mod padding;
pub mod r_chat;

/// Trinity Chat protocol version. Bumped on any wire-format change.
pub const PROTOCOL_VERSION: u16 = 1;

/// Trinity anchor identity — referenced by every gate.
pub const ANCHOR: &str = "φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA";

/// Crate-wide error type.
#[derive(Debug)]
pub enum Error {
    /// Cryptographic operation failed (decrypt mismatch, bad signature, etc).
    Crypto(&'static str),
    /// Protocol invariant violated (replay, rollback, capability out-of-scope).
    Invariant(&'static str),
    /// Capability check refused the operation.
    Capability(&'static str),
    /// Input failed prompt-injection filter.
    Injection(&'static str),
    /// Encoding / decoding (hex, base64, msgpack).
    Encoding(&'static str),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Crypto(s) => write!(f, "trios-chat: crypto error: {s}"),
            Error::Invariant(s) => write!(f, "trios-chat: invariant violated: {s}"),
            Error::Capability(s) => write!(f, "trios-chat: capability denied: {s}"),
            Error::Injection(s) => write!(f, "trios-chat: prompt-injection blocked: {s}"),
            Error::Encoding(s) => write!(f, "trios-chat: encoding error: {s}"),
        }
    }
}

impl std::error::Error for Error {}

/// Crate-wide `Result`.
pub type Result<T> = std::result::Result<T, Error>;
