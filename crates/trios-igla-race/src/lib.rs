pub mod asha;
pub mod hive_automaton;
pub mod invariants;
pub mod lessons;
pub mod neon;
pub mod rungs;
pub mod sampler;
pub mod status;

pub use asha::{AshaConfig, AshaRung, record_checkpoint, register_trial};

pub use lessons::{generate_lesson, get_top_lessons, store_lesson, LessonType, Outcome, TrialConfig, RungData};

pub use neon::{NeonDb, LessonEntry, DashboardMeta, spawn_heartbeat};

pub use status::*;

pub use invariants::{InvTrialConfig, GradientMode, InvError, validate_config};

pub use rungs::{check_inv12_rung_valid, check_inv12_rung_valid_usize, Rung, TRINITY_BASE, RUNG_UNIT, RUNG_COUNT, MAX_RUNG_EXP};

pub use hive_automaton::{
    AbortReason, AgentAction, HaltCause, HiveAutomaton, Lane, State, World,
    BPB_VICTORY_TARGET, LANE_COUNT, SCHEMA_VERSION as HIVE_SCHEMA_VERSION,
    VICTORY_SEED_TARGET,
};

pub const IGLA_TARGET_BPB: f64 = 1.5;
pub const ASHA_KEEP_FRACTION: f64 = 0.33;
