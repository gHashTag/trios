//! `trios-phd` — Rust-native PhD pipeline (Flos Aureus v6.0).
//!
//! This crate is the proof ↔ runtime ↔ monograph bridge described in #62 / #109 / #30.
//! It is intentionally a **skeleton**: it covers the audit primitives required by
//! the L-R14 gate (every numeric constant in `docs/phd/**.tex` must trace to either
//! a CODATA/PDG entry or a Coq theorem in `assertions/igla_assertions.json`) but
//! deliberately does **not** invoke `tectonic` yet — that is a follow-up PR per the
//! "honest scope" reconciliation in `docs/phd/BRIDGE_AUDIT.md`.
//!
//! ## Public surface
//!
//! - [`cite::CodataLink`] / [`cite::CoqLink`] — the only sanctioned ways to introduce
//!   a numeric constant into the monograph. Each carries the source name + value +
//!   tolerance, and the build-time audit asserts the value matches the reference.
//! - [`audit::Assertions`] — typed view of `assertions/igla_assertions.json` (single
//!   source of truth, reconciled with `trinity-clara/assertions/igla_assertions.json`).
//! - [`audit::run_audit`] — top-level audit entry point used by both `cargo test` and
//!   `cargo run -p trios-phd -- audit`.
//!
//! ## Honesty contract
//!
//! Status of each invariant is reported verbatim from the JSON (`Proven` /
//! `Admitted`). A lint check enforces that no `.tex` file calls `\coqbox{INV-X}` for
//! an `Admitted` invariant without an accompanying `\admittedbox{...}` macro
//! (mission rule R5). See [`audit::run_audit`] for the implementation.

pub mod cite;
pub mod audit;

/// Golden ratio φ — the only physical constant allowed by mission rule R6
/// (zero free parameters, only `{φ, π, e, n ∈ ℤ}`).
///
/// `φ² + φ⁻² = 3` — Trinity Identity (Theorem 1, Ch.3 Golden Cut).
pub const PHI: f64 = 1.618_033_988_749_894_8;

/// `φ⁻¹` derived directly from [`PHI`].
pub const PHI_INV: f64 = 1.0 / PHI;
