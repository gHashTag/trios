//! trios-igla-race-hack — GOLD III crate facade.
//!
//! Re-exports SR-HACK-00 glossary types so downstream rings (SR-HACK-01..05)
//! and consumers (DMs, PR comments, leaderboard, Discord) import a single
//! canonical vocabulary.
//!
//! L-RING-FACADE-001: this file MUST NOT contain business logic — only
//! re-exports.

pub use trios_igla_race_hack_sr_hack_00::*;
