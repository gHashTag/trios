//! # trios-igla-race-hack — Community/Outreach Plumbing
//!
//! GOLD III ring — CLI surface and external API. This crate provides
//! the user-facing interface to the IGLA RACE system.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     HACK LAYER                        │
//! │  (trios-igla-race-hack)                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │ cli               │ Command-line interface           │
//! │ status            │ Query race status                │
//! │ dashboard         │ Live dashboard streaming        │
//! │ api               │ Public HTTP API surface       │
//! └─────────────────────────────────────────────────────────────┘
//!         │
//!         ▼
//! ┌─────────────────────────────────────┐
//! │     ARENA + PIPELINE            │
//! │   (validation + execution)        │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Module Organization
//!
//! - `cli` — Command-line interface (main entry point)
//! - `status` — Status queries and reporting
//! - `dashboard` — Live dashboard streaming
//! - `api` — Public HTTP API surface
//!
//! ## External Dependencies
//!
//! - `trios-algorithm-arena` — Validation, victory checking
//! - `trios-igla-race-pipeline` — Trial execution

pub mod cli;
pub mod status;
pub mod dashboard;
pub mod api;

// Re-exports for public API
pub use cli::{run_cli, Cli};
pub use status::{RaceStatus, QueryStatus};
pub use dashboard::{Dashboard, DashboardEvent};
pub use api::{ApiServer, ApiConfig};

/// Hack version — increments on CLI surface changes
pub const HACK_VERSION: &str = "0.1.0";

/// IGLA RACE CLI name
pub const CLI_NAME: &str = "trios-igla-race";

/// BPB victory target
pub const IGLA_TARGET_BPB: f64 = 1.5;
