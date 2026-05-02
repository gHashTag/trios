//! SR-00 — scarab-types
//!
//! Dependency-free, serde-only typed primitives for the E2E TTT
//! pipeline. This ring is the bottom of the GOLD I dependency graph;
//! every other GOLD I ring (SR-01 strategy-queue, SR-02 trainer-runner,
//! SR-03 bpb-writer, SR-04 gardener, SR-05 railway-deployer, BR-OUTPUT
//! IglaRacePipeline) imports its wire format from here.
//!
//! Closes #448 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## Rules (ABSOLUTE)
//!
//! - R1  — Pure Rust only (serde + serde_json + uuid + chrono)
//! - L6  — No I/O, no async, no subprocess, no network, no tokio, no sqlx
//! - L13 — I-SCOPE: this ring only
//! - R-RING-DEP-002 — deps limited to `serde`, `serde_json`, `uuid`, `chrono`
//!
//! ## Public types
//!
//! | Type            | Wire-format role |
//! |-----------------|------------------|
//! | `JobId`         | UUID v4 newtype identifying one trainer job |
//! | `WorkerId`      | String newtype for `<machine>:<worker>` |
//! | `Seed`          | i64 newtype for the trainer RNG seed |
//! | `StrategyId`    | UUID v4 newtype identifying a strategy enqueued by SR-01 |
//! | `JobStatus`     | enum: Queued, Running, Done, Pruned, Errored |
//! | `Heartbeat`     | per-tick liveness from a worker |
//! | `BpbSampleRow`  | one BPB observation (1:1 with `bpb_samples` table) |
//! | `Scarab`        | composite trainer state record (1:1 with `scarabs` table) |

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ─────────────────────────────── newtypes ───────────────────────────

/// Unique identifier of a trainer job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub Uuid);

impl JobId {
    /// Generate a fresh v4 JobId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    /// Underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}
impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for JobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Worker identifier — typically `<machine_id>:<worker_index>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WorkerId(pub String);

impl WorkerId {
    /// Build from machine + worker index.
    pub fn new(machine: impl Into<String>, worker: u32) -> Self {
        Self(format!("{}:{}", machine.into(), worker))
    }
    /// Backing string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Trainer RNG seed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seed(pub i64);

impl Seed {
    /// Underlying integer.
    pub fn value(self) -> i64 {
        self.0
    }
}
impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Strategy identifier — one row in `strategy_queue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StrategyId(pub Uuid);

impl StrategyId {
    /// Generate a fresh v4 StrategyId.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
impl Default for StrategyId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for StrategyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

// ─────────────────────────────── enums ──────────────────────────────

/// Lifecycle state of a job.
///
/// Matches `scarabs.status` column type (`text NOT NULL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Enqueued, waiting for a worker to claim.
    Queued,
    /// Claimed and currently training.
    Running,
    /// Completed — final BPB recorded.
    Done,
    /// Killed by SR-04 gardener (ASHA cull).
    Pruned,
    /// Crashed / erred out.
    Errored,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Pruned => "pruned",
            JobStatus::Errored => "errored",
        };
        f.write_str(s)
    }
}

// ─────────────────────────────── rows ───────────────────────────────

/// Heartbeat — one liveness tick from a worker to LEAD.
///
/// Matches `heartbeats` table:
///
/// ```sql
/// CREATE TABLE heartbeats (
///   job_id     uuid        NOT NULL,
///   worker_id  text        NOT NULL,
///   ts         timestamptz NOT NULL,
///   step       bigint,
///   bpb        double precision,
///   PRIMARY KEY (job_id, ts)
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Job this heartbeat belongs to.
    pub job_id: JobId,
    /// Worker emitting the tick.
    pub worker_id: WorkerId,
    /// Timestamp (UTC).
    pub ts: DateTime<Utc>,
    /// Optional current step.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<i64>,
    /// Optional current BPB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bpb: Option<f64>,
}

/// One BPB observation row.
///
/// Matches `bpb_samples` table:
///
/// ```sql
/// CREATE TABLE bpb_samples (
///   job_id  uuid              NOT NULL,
///   step    bigint            NOT NULL,
///   bpb     double precision  NOT NULL,
///   ema     double precision,
///   ts      timestamptz       NOT NULL DEFAULT now(),
///   PRIMARY KEY (job_id, step)
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BpbSampleRow {
    /// Job this sample belongs to.
    pub job_id: JobId,
    /// Training step.
    pub step: i64,
    /// Raw BPB at this step.
    pub bpb: f64,
    /// Exponential moving average (optional, written by SR-03).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ema: Option<f64>,
    /// Timestamp.
    pub ts: DateTime<Utc>,
}

/// Composite trainer state record.
///
/// Matches `scarabs` table:
///
/// ```sql
/// CREATE TABLE scarabs (
///   job_id       uuid        PRIMARY KEY,
///   strategy_id  uuid        NOT NULL,
///   worker_id    text,
///   seed         bigint      NOT NULL,
///   status       text        NOT NULL,
///   created_at   timestamptz NOT NULL DEFAULT now(),
///   started_at   timestamptz,
///   completed_at timestamptz,
///   best_bpb     double precision,
///   best_step    bigint,
///   config       jsonb       NOT NULL
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scarab {
    /// Job identifier.
    pub job_id: JobId,
    /// Strategy this job is realising.
    pub strategy_id: StrategyId,
    /// Worker that picked the job up (None until claimed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<WorkerId>,
    /// RNG seed.
    pub seed: Seed,
    /// Lifecycle status.
    pub status: JobStatus,
    /// Insertion time.
    pub created_at: DateTime<Utc>,
    /// Time the worker claimed the job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Terminal time (success or failure).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    /// Best BPB seen so far.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_bpb: Option<f64>,
    /// Step at which `best_bpb` was reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_step: Option<i64>,
    /// Free-form trainer config (JSON object).
    pub config: serde_json::Value,
}

impl Scarab {
    /// Build a fresh `Queued` scarab with no worker.
    pub fn queued(strategy_id: StrategyId, seed: Seed, config: serde_json::Value) -> Self {
        Self {
            job_id: JobId::new(),
            strategy_id,
            worker_id: None,
            seed,
            status: JobStatus::Queued,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            best_bpb: None,
            best_step: None,
            config,
        }
    }
}

// ─────────────────────────────── tests ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn fixed_ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 9, 0, 0).unwrap()
    }

    #[test]
    fn job_id_roundtrip_and_unique() {
        let a = JobId::new();
        let b = JobId::new();
        assert_ne!(a, b);
        let s = serde_json::to_string(&a).unwrap();
        let back: JobId = serde_json::from_str(&s).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn worker_id_format() {
        let w = WorkerId::new("rp-acc1", 3);
        assert_eq!(w.as_str(), "rp-acc1:3");
        let s = serde_json::to_string(&w).unwrap();
        assert_eq!(s, "\"rp-acc1:3\"");
    }

    #[test]
    fn seed_serializes_as_int() {
        let s = serde_json::to_string(&Seed(42)).unwrap();
        assert_eq!(s, "42");
        let back: Seed = serde_json::from_str("42").unwrap();
        assert_eq!(back, Seed(42));
    }

    #[test]
    fn strategy_id_roundtrip() {
        let s = StrategyId::new();
        let j = serde_json::to_string(&s).unwrap();
        let back: StrategyId = serde_json::from_str(&j).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn job_status_serializes_snake_case() {
        for (st, expected) in [
            (JobStatus::Queued, "\"queued\""),
            (JobStatus::Running, "\"running\""),
            (JobStatus::Done, "\"done\""),
            (JobStatus::Pruned, "\"pruned\""),
            (JobStatus::Errored, "\"errored\""),
        ] {
            assert_eq!(serde_json::to_string(&st).unwrap(), expected);
        }
    }

    #[test]
    fn job_status_roundtrip_every_variant() {
        for st in [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Done,
            JobStatus::Pruned,
            JobStatus::Errored,
        ] {
            let s = serde_json::to_string(&st).unwrap();
            let back: JobStatus = serde_json::from_str(&s).unwrap();
            assert_eq!(st, back);
        }
    }

    #[test]
    fn heartbeat_roundtrip_full() {
        let hb = Heartbeat {
            job_id: JobId::new(),
            worker_id: WorkerId::new("m1", 0),
            ts: fixed_ts(),
            step: Some(1234),
            bpb: Some(2.18),
        };
        let s = serde_json::to_string(&hb).unwrap();
        let back: Heartbeat = serde_json::from_str(&s).unwrap();
        assert_eq!(hb, back);
    }

    #[test]
    fn heartbeat_omits_none_fields() {
        let hb = Heartbeat {
            job_id: JobId::new(),
            worker_id: WorkerId::new("m1", 0),
            ts: fixed_ts(),
            step: None,
            bpb: None,
        };
        let s = serde_json::to_string(&hb).unwrap();
        assert!(!s.contains("\"step\""), "step should be omitted when None: {}", s);
        assert!(!s.contains("\"bpb\""), "bpb should be omitted when None: {}", s);
    }

    #[test]
    fn bpb_sample_row_roundtrip() {
        let row = BpbSampleRow {
            job_id: JobId::new(),
            step: 1000,
            bpb: 2.34,
            ema: Some(2.40),
            ts: fixed_ts(),
        };
        let s = serde_json::to_string(&row).unwrap();
        let back: BpbSampleRow = serde_json::from_str(&s).unwrap();
        assert_eq!(row, back);
    }

    #[test]
    fn scarab_queued_constructor() {
        let sc = Scarab::queued(
            StrategyId::new(),
            Seed(43),
            serde_json::json!({"hidden":384,"lr":0.005}),
        );
        assert_eq!(sc.status, JobStatus::Queued);
        assert!(sc.worker_id.is_none());
        assert!(sc.started_at.is_none());
        assert!(sc.completed_at.is_none());
        assert!(sc.best_bpb.is_none());
        assert!(sc.best_step.is_none());
        assert_eq!(sc.seed, Seed(43));
    }

    #[test]
    fn scarab_full_roundtrip() {
        let sc = Scarab {
            job_id: JobId::new(),
            strategy_id: StrategyId::new(),
            worker_id: Some(WorkerId::new("m1", 2)),
            seed: Seed(42),
            status: JobStatus::Running,
            created_at: fixed_ts(),
            started_at: Some(fixed_ts()),
            completed_at: None,
            best_bpb: Some(2.19),
            best_step: Some(50_000),
            config: serde_json::json!({"hidden":384}),
        };
        let s = serde_json::to_string(&sc).unwrap();
        let back: Scarab = serde_json::from_str(&s).unwrap();
        assert_eq!(sc, back);
    }

    #[test]
    fn schema_field_parity_scarab() {
        // Compile-time-ish parity check: the SQL contract has these exact
        // column names. Field renames here MUST be paired with a schema
        // migration in `schema/scarabs_v1.sql`.
        let sc = Scarab::queued(
            StrategyId::new(),
            Seed(1),
            serde_json::json!({}),
        );
        let v = serde_json::to_value(&sc).unwrap();
        for k in [
            "job_id",
            "strategy_id",
            "seed",
            "status",
            "created_at",
            "config",
        ] {
            assert!(
                v.get(k).is_some(),
                "scarabs field '{}' missing from JSON serialization",
                k
            );
        }
    }

    #[test]
    fn schema_field_parity_bpb_sample() {
        let row = BpbSampleRow {
            job_id: JobId::new(),
            step: 1,
            bpb: 1.0,
            ema: None,
            ts: fixed_ts(),
        };
        let v = serde_json::to_value(&row).unwrap();
        for k in ["job_id", "step", "bpb", "ts"] {
            assert!(
                v.get(k).is_some(),
                "bpb_samples field '{}' missing",
                k
            );
        }
    }

    #[test]
    fn schema_field_parity_heartbeat() {
        let hb = Heartbeat {
            job_id: JobId::new(),
            worker_id: WorkerId::new("m1", 0),
            ts: fixed_ts(),
            step: Some(1),
            bpb: Some(1.0),
        };
        let v = serde_json::to_value(&hb).unwrap();
        for k in ["job_id", "worker_id", "ts"] {
            assert!(
                v.get(k).is_some(),
                "heartbeats field '{}' missing",
                k
            );
        }
    }
}
