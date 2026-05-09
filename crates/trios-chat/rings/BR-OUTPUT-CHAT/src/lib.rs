//! # BR-OUTPUT-CHAT — Trinity Secure Chat assembler ring
//!
//! Bronze-tier re-export ring that assembles the Trinity Secure Chat stack
//! from CR-CHAT-* (Silver) rings into a single public surface.
//!
//! ## Wiring map
//!
//! | Sub-module    | Source ring        | Lane       |
//! |---------------|--------------------|------------|
//! | (root types)  | CR-CHAT-00         | errors     |
//! | `identity`    | CR-CHAT-01         | L-CHAT-1   |
//! | `sealed`      | CR-CHAT-01         | L-CHAT-4   |
//! | `ratchet`     | CR-CHAT-02         | L-CHAT-2   |
//! | `group`       | CR-CHAT-03         | L-CHAT-3   |
//! | `padding`     | CR-CHAT-04         | L-CHAT-7   |
//! | `persist`     | CR-CHAT-05 (trait) | L-CHAT-9   |
//! | `capability`  | CR-CHAT-06         | L-CHAT-6   |
//! | `injection`   | CR-CHAT-06         | L-CHAT-6   |
//! | `r_chat`      | CR-CHAT-LAWS       | constitution |
//!
//! Concrete persistence (SeaORM → Postgres) lives in the sibling
//! `trios-chat-br-io-chat-05` ring; consumers depend on it directly when
//! they need real I/O.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA`
//!
//! Parent EPIC: trinity-fpga#28 · Builds on: trinity-fpga#22 · trios#629.
//!
//! ## Honesty (R5)
//! - [VERIFIED] all re-exports are tested in their source rings.
//! - [DERIVED] this ring is pure re-exports — no logic of its own.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

// ---------------------------------------------------------------------------
// CR-CHAT-00 — error type & Result alias
// ---------------------------------------------------------------------------
pub use trios_chat_cr_chat_00::{Error, Result};

// ---------------------------------------------------------------------------
// CR-CHAT-LAWS — constitutional laws
// ---------------------------------------------------------------------------
pub mod r_chat {
    //! L-CHAT-LAWS — constitutional laws (R-CHAT-1..12).
    pub use trios_chat_cr_chat_laws::{laws_hash, R_CHAT_LAWS};
}

// ---------------------------------------------------------------------------
// CR-CHAT-01 — identity + sealed envelope
// ---------------------------------------------------------------------------
pub mod identity {
    //! L-CHAT-1 — Ed25519 + X25519 + ML-KEM-768 prekey bundle.
    pub use trios_chat_cr_chat_01::identity::*;
}

pub mod sealed {
    //! L-CHAT-4 — sealed-sender envelope.
    pub use trios_chat_cr_chat_01::sealed::*;
}

// ---------------------------------------------------------------------------
// CR-CHAT-02 — ratchet
// ---------------------------------------------------------------------------
pub mod ratchet {
    //! L-CHAT-2 — Double Ratchet (DH-step + skipped-keys cap).
    pub use trios_chat_cr_chat_02::*;
}

// ---------------------------------------------------------------------------
// CR-CHAT-03 — group MLS skeleton
// ---------------------------------------------------------------------------
pub mod group {
    //! L-CHAT-3 — MLS-style group skeleton.
    pub use trios_chat_cr_chat_03::*;
}

// ---------------------------------------------------------------------------
// CR-CHAT-04 — padding
// ---------------------------------------------------------------------------
pub mod padding {
    //! L-CHAT-7 — fixed-size padding classes.
    pub use trios_chat_cr_chat_04::*;
}

// ---------------------------------------------------------------------------
// CR-CHAT-05 — persistence trait surface
// ---------------------------------------------------------------------------
pub mod persist {
    //! L-CHAT-9 — persistence trait (SeaORM impl in BR-IO-CHAT-05).
    pub use trios_chat_cr_chat_05::*;
}

// ---------------------------------------------------------------------------
// CR-CHAT-06 — capability + injection
// ---------------------------------------------------------------------------
pub mod capability {
    //! L-CHAT-6a — capability tokens & signed tool manifests.
    pub use trios_chat_cr_chat_06::capability::*;
}

pub mod injection {
    //! L-CHAT-6b — dual-LLM isolation + output validator.
    pub use trios_chat_cr_chat_06::injection::*;
}

// ---------------------------------------------------------------------------
// Crate-wide constants (formerly in trios-chat src/lib.rs).
// ---------------------------------------------------------------------------

/// Trinity Chat protocol version. Bumped on any wire-format change.
pub const PROTOCOL_VERSION: u16 = 1;

/// Trinity anchor identity — referenced by every gate.
pub const ANCHOR: &str = "φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_v1() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }

    #[test]
    fn anchor_present() {
        assert!(ANCHOR.contains("TRINITY"));
        assert!(ANCHOR.contains("ZERO-METADATA"));
    }

    #[test]
    fn re_export_smoke_laws() {
        // 12 laws reachable through the assembler.
        assert_eq!(r_chat::R_CHAT_LAWS.len(), 12);
    }

    #[test]
    fn re_export_smoke_padding() {
        // padding constants reachable through the assembler.
        assert!(padding::CLASSES.iter().all(|&c| c > 0));
    }
}
