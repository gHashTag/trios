//! trios-igla-race-pipeline — GOLD I crate facade.
//!
//! Re-exports SR-00 typed primitives so SR-01..05 + BR-OUTPUT (and the
//! legacy `trios-igla-race` crate) can import a single canonical name
//! for every wire-format type in the E2E TTT pipeline.
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic — only
//! re-exports.

pub use trios_igla_race_pipeline_sr_00::*;
pub use trios_igla_race_pipeline_sr_01::{
    is_valid_transition, transition, FsmError, StrategyQueue,
};
pub use trios_igla_race_pipeline_sr_03::{
    BpbSink, BpbWriter, EmaPhiBand, WriteErr,
    PHI_BAND_ALPHA, PHI_BAND_HIGH, PHI_BAND_LOW, SCHEMA_SQL,
};
pub use trios_igla_race_pipeline_sr_04::{
    AshaRung, Gardener, GardenerAction, GardenerDecision, GardenerErr, GardenerSink,
    InvariantStatus, ARCHITECTURAL_FLOOR_BPB, DEFAULT_RUNGS, WARMUP_STEPS,
};
pub use trios_igla_race_pipeline_br_output::{
    check_victory, is_victory, stat_strength, IglaRacePipeline, MockedTrainer, PerSeedSummary,
    PipelineCfg, PipelineErr, RunSummary, SeedResult, Sinks, TrainerBackend, TtestReport,
    VictoryError, VictoryReport, BPB_VICTORY_TARGET, INV2_WARMUP_BLIND_STEPS,
    JEPA_PROXY_BPB_FLOOR, TTEST_ALPHA, TTEST_BASELINE_MU0, TTEST_EFFECT_SIZE_MIN,
    VICTORY_SEED_TARGET,
};
