//! `trios-igla-race` — facade crate.
//!
//! R-RING-FACADE-001: this file MUST NOT contain business logic — only
//! `pub mod` declarations and re-exports for the small surface that
//! external bins actually consume. Closes the Wave-5 debt called out
//! in #459.
//!
//! Anchor: `phi^2 + phi^-2 = 3` · L-R14 BPB target = 1.5.

#![forbid(unsafe_code)]

pub mod asha;
pub mod attn;
pub mod ema;
pub mod hive_automaton;
pub mod invariants;
pub mod lessons;
pub mod neon;
pub mod race;
pub mod rungs;
pub mod sampler;
pub mod status;
pub mod victory;

// External-bin surface (R5-honest: only re-export what is consumed
// by `bin/{honey_audit,ledger_check,qk_gain_check,seed_emit}.rs` or
// `main.rs`).
// L-R14 anchor: BPB target for IGLA RACE victory (1.5). Re-exported
// from `hive_automaton::BPB_VICTORY_TARGET` for back-compat with the
// `IGLA_TARGET_BPB` name that external bins import.
pub use hive_automaton::{BPB_VICTORY_TARGET as IGLA_TARGET_BPB, VICTORY_SEED_TARGET};
pub use victory::{check_victory, stat_strength, JEPA_PROXY_BPB_FLOOR, SeedResult, TtestReport, VictoryError, VictoryReport};
