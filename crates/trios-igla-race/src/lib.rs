//! IGLA Race — ASHA-based hyperparameter search for finding models < 1.5 BPB
//!
//! Architecture:
//! - ASHA rungs: 1k → 3k → 9k → 27k (3^k Trinity progression)
//! - Neon PostgreSQL: centralized leaderboard and failure memory
//! - Failure Memory: automatic lesson generation from pruned trials

pub mod asha;
pub mod lessons;
pub mod neon;
pub mod status;

pub use asha::*;
pub use lessons::*;
pub use neon::*;
pub use status::*;

/// IGLA target BPB threshold
pub const IGLA_TARGET_BPB: f64 = 1.5;

/// ASHA reduction factor (top 33% promoted)
pub const ASHA_KEEP_FRACTION: f64 = 0.33;
