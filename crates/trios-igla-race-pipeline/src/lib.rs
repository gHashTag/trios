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
