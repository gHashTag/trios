//! # trios-algorithm-arena — Rust Spec/Verifier Wrapper
//!
//! GOLD II ring — validation, invariants, and verification logic.
//! This crate bridges Coq-proven invariants to runtime validation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    ARENA LAYER                       │
//! │  (trios-algorithm-arena)                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │ invariants         │ INV-001..005 validation      │
//! │ rungs             │ φ-anchored rung schedule    │
//! │ victory           │ BPB victory detection        │
//! │ verifier          │ Welch t-test, L14 enforcement  │
//! └─────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! ┌──────────────────┐
//! │   PIPELINE      │
//! │   execution     │
//! └──────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - `invariants` — All INV constants and validation logic
//! - `rungs` — Rung schedule and traversal
//! - `victory` — Victory detection and reporting
//! - `verifier` — Welch t-test, INV-14 enforcement
//!
//! ## Trinity Constants (φ-anchored)
//!
//! All constants are traceable to the Trinity identity:
//! φ² + φ⁻² = 3
//!
//! See `invariants.rs` for Coq proofs mapping.

pub mod invariants;
pub mod rungs;
pub mod victory;
pub mod verifier;

// Re-exports
pub use invariants::*;
pub use rungs::{Rung, iter_rungs, RungIter};
pub use victory::{VictoryReport, check_victory, is_victory, TtestReport, welch_ttest, VictoryError};
pub use verifier::{Verifier, L14Validator, L14Error};

/// Arena version — increments on invariant set changes
pub const ARENA_VERSION: &str = "0.1.0";
