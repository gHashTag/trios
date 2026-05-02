//! # trios-igla-race-pipeline — E2E Test-Time Training Pipeline
//!
//! GOLD I ring — core pipeline logic for IGLA RACE. This crate
//! contains the trial execution engine, ASHA hyperparameter optimization,
//! sampling utilities, and worker pool orchestration.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    PIPELINE LAYER                    │
//! │  (trios-igla-race-pipeline)                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │ trial_runner       │ Execute single trial (GPU path)    │
//! │ asha_scheduler     │ Successive Halving algorithm        │
//! │ lr_sampler        │ φ-band learning rate sampling      │
//! │ worker_pool       │ Parallel worker orchestration     │
//! │ telemetry         │ CSV/Neon event streaming       │
//! └─────────────────────────────────────────────────────────────┘
//!         │                        │
//!         ▼                        ▼
//! ┌──────────────┐    ┌───────────────────┐
//! │   ARENA     │    │     HACK         │
//! │ validation   │◄───│   CLI surface     │
//! └──────────────┘    └───────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - `trial` — Core trial execution logic
//! - `asha` — Successive Halving algorithm implementation
//! - `sampler` — φ-band learning rate sampling
//! - `worker` — Worker pool and trial distribution
//! - `telemetry` — Event streaming to CSV/Neon
//!
//! ## External Dependencies
//!
//! - `trios-algorithm-arena` — Validation, invariants, victory checking
//!
//! # Constants
//!
//! All φ-anchored constants are defined in `trios-algorithm-arena`
//! to maintain single source of truth.

pub mod trial;
pub mod asha;
pub mod sampler;
pub mod worker;
pub mod telemetry;

// Re-exports for convenience
pub use trios_algorithm_arena::invariants::TrialConfig;
pub use trial::{TrialResult, TrialError};
pub use asha::{AshaConfig, AshaRung, run_asha};
pub use sampler::{LrSampler, LrSampleError};
pub use worker::{WorkerPool, WorkerConfig, WorkerResult};
pub use telemetry::{TelemetrySink, TelemetryEvent};

/// Pipeline version — increments on breaking API changes
pub const PIPELINE_VERSION: &str = "0.1.0";
