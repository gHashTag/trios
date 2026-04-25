pub mod asha;
pub mod hive_automaton;
pub mod invariants;
pub mod lessons;
pub mod neon;
pub mod race;
pub mod rungs;
pub mod attn;
pub mod bpb;
pub mod ema;
pub mod gf16;
pub mod nca;
pub mod sampler;
pub mod status;
pub mod victory;

pub use asha::{AshaConfig, AshaRung, record_checkpoint, register_trial};

pub use lessons::{generate_lesson, get_top_lessons, store_lesson, LessonType, Outcome, TrialConfig, RungData};

pub use neon::{NeonDb, LessonEntry, DashboardMeta, spawn_heartbeat};

pub use status::*;

pub use invariants::{TrialConfig as InvTrialConfig, GradientMode, InvError, validate_config};

pub use race::{WorkerPool, WorkerState, RaceState, PoolStats, check_worker_pool_invariant,
               TRINITY_IDENTITY, VICTORY_SEEDS_REQUIRED, IGLA_TARGET_BPB,
               GF16_MIN_D_MODEL, ASHA_PRUNE_THRESHOLD_MIN, VALID_RUNGS};

pub use rungs::{check_inv12_rung_valid, check_inv12_rung_valid_usize, Rung, TRINITY_BASE, RUNG_UNIT, RUNG_COUNT, MAX_RUNG_EXP};

pub use victory::{
    check_victory, is_victory, stat_strength, inv7_check_victory_constants,
    SeedResult, VictoryError, VictoryReport, TtestReport, TtestError,
    JEPA_PROXY_BPB_FLOOR, WELCH_ALPHA, WELCH_BASELINE_MU0, WELCH_EFFECT_SIZE_MIN,
};

pub use ema::{EmaTracker, EmaError, ALPHA_PHI_INV_3, ALPHA_MIN_EXCLUSIVE, ALPHA_MAX_INCLUSIVE};

pub use nca::{
    assert_bands_distinct, validate_nca_entropy, validate_nca_entropy_canonical, NcaBandMode,
    NcaError, NcaReport,
};

pub use attn::{QkHead, QkHeadError, PHI_4, HEAD_DIM_PHI_FLOOR, NUM_HEADS_MAX};

pub use bpb::{BpbTracker, BpbError};

pub use hive_automaton::{
    AbortReason, AgentAction, HaltCause, HiveAutomaton, Lane, State, World,
    BPB_VICTORY_TARGET, LANE_COUNT, SCHEMA_VERSION as HIVE_SCHEMA_VERSION,
    VICTORY_SEED_TARGET,
};

pub const ASHA_KEEP_FRACTION: f64 = 0.33;
