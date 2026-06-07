pub mod asha;
pub mod attn;
pub mod bpb;
pub mod ema;
pub mod gf16;
pub mod hive_automaton;
pub mod invariants;
pub mod lessons;
pub mod nca;
pub mod neon;
pub mod proxies;
pub mod race;
pub mod real_forward;
pub mod rungs;
pub mod sampler;
pub mod status;
pub mod victory;
pub mod victory_new;

/// IGLA Gate-3 target BPB threshold (pre-registered: igla_assertions.json).
/// Champion must achieve BPB < 1.50 on ≥3 distinct seeds.
pub const IGLA_TARGET_BPB: f64 = 1.5;

// ----------------------------------------------------------------------
// INV-7: Welch t-test and TtestReport exports (L-R14)
// ----------------------------------------------------------------------

pub use asha::{record_checkpoint, register_trial, AshaConfig, AshaRung};

pub use lessons::{
    generate_lesson, get_top_lessons, store_lesson, LessonType, Outcome, RungData, TrialConfig,
};

pub use neon::{spawn_heartbeat, DashboardMeta, LessonEntry, NeonDb};

pub use status::*;

pub use invariants::{validate_config, GradientMode, InvError, TrialConfig as InvTrialConfig};

pub use rungs::{
    check_inv12_rung_valid, check_inv12_rung_valid_usize, Rung, MAX_RUNG_EXP, RUNG_COUNT,
    RUNG_UNIT, TRINITY_BASE,
};

// Race exports (L11 internal)
pub use race::{run_trial, simulate_bpb, WorkerPool};

pub use victory::{
    check_victory, is_victory, stat_strength, SeedResult, TtestReport, VictoryError, VictoryReport,
    JEPA_PROXY_BPB_FLOOR,
};

pub use ema::{EmaError, EmaTracker, ALPHA_MAX_INCLUSIVE, ALPHA_MIN_EXCLUSIVE, ALPHA_PHI_INV_3};

pub use attn::{QkHead, QkHeadError, HEAD_DIM_PHI_FLOOR, NUM_HEADS_MAX, PHI_4};

pub use hive_automaton::{
    AbortReason, AgentAction, HaltCause, HiveAutomaton, Lane, State, World, BPB_VICTORY_TARGET,
    LANE_COUNT, SCHEMA_VERSION as HIVE_SCHEMA_VERSION, VICTORY_SEED_TARGET,
};
