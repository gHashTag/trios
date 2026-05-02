//! SR-03 — bpb-writer
//!
//! BPB + EMA(α_φ) + Neon write path. Provides the φ-band EMA filter
//! (Theorem 3.1 SAC-1, INV-8 PROVEN) and a [`BpbSink`] trait that any
//! Postgres adapter can implement to land [`BpbSampleRow`]s.
//!
//! Closes #451 · Part of #446 · Anchor: phi^2 + phi^-2 = 3
//!
//! ## INV-8 PROVEN — α_φ = φ⁻³ / 2
//!
//! The Coq.Reals theorem in `trios#330` pins the EMA decay to
//! `α_φ = φ⁻³ / 2 = 0.118033988749895…`. The constructor asserts the
//! INV-8 φ-band `0.10 ≤ α ≤ 0.13` and refuses to instantiate outside it
//! — any drift is a regression that MUST surface as a [`WriteErr::PhiBandOutOfRange`].
//!
//! ## Why a trait, not a hard sqlx dep
//!
//! SR-03 stays Bronze-tier: pure compute + a sink trait. The concrete
//! `tokio-postgres` / `sqlx` adapter ships in a separate BR-IO ring so
//! SR-04 gardener and BR-OUTPUT can mock the sink in unit tests.
//!
//! ## Rules
//!
//! - R1   — pure Rust
//! - L6   — async via the trait, no global runtime here
//! - L13  — I-SCOPE: only this ring
//! - R-RING-DEP-002 — deps = sr-00 + serde + serde_json + chrono + thiserror

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::future::Future;
use std::pin::Pin;

use thiserror::Error;
use trios_igla_race_pipeline_sr_00::{BpbSampleRow, JobId, Scarab};

// φ = (1 + √5) / 2  — the golden ratio
const PHI: f64 = 1.618_033_988_749_895;

/// Theorem 3.1 SAC-1 / INV-8: `α_φ = φ⁻³ / 2`.
///
/// Numerically: `0.118033988749895…`. This constant is the *only*
/// allowed EMA decay on the BPB write path. Any deviation is a φ-band
/// violation and surfaces as [`WriteErr::PhiBandOutOfRange`].
pub const PHI_BAND_ALPHA: f64 = 1.0 / (PHI * PHI * PHI * 2.0);

/// INV-8 φ-band lower bound (inclusive).
pub const PHI_BAND_LOW: f64 = 0.10;
/// INV-8 φ-band upper bound (inclusive).
pub const PHI_BAND_HIGH: f64 = 0.13;

/// Errors produced by the BPB write path.
#[derive(Debug, Error)]
pub enum WriteErr {
    /// EMA α drifted outside the proven INV-8 φ-band `[0.10, 0.13]`.
    #[error("INV-8 φ-band violated: α = {alpha}, expected α_φ = φ⁻³/2 ≈ 0.1180")]
    PhiBandOutOfRange {
        /// Observed alpha.
        alpha: f64,
    },
    /// Sink-side I/O failure (DB, network, encoding…).
    #[error("sink error: {0}")]
    Sink(String),
    /// Caller passed an [`BpbSampleRow`] whose `job_id` does not match
    /// the job currently being written.
    #[error("row job_id {row_job_id} does not match writer job_id {writer_job_id}")]
    JobIdMismatch {
        /// Row's job id.
        row_job_id: JobId,
        /// Expected writer's job id.
        writer_job_id: JobId,
    },
}

// ─────────────────── EMA filter (φ-band) ───────────────────────────

/// Exponential moving average with the φ-banded decay `α_φ = φ⁻³ / 2`.
///
/// Behaviour:
///
/// 1. The first `update(x)` initialises the EMA to `x` and returns `x`.
/// 2. Subsequent calls return `α·x + (1-α)·prev`.
/// 3. The constructor refuses any α outside `[0.10, 0.13]` (INV-8).
#[derive(Debug, Clone, Copy)]
pub struct EmaPhiBand {
    alpha: f64,
    state: Option<f64>,
}

impl EmaPhiBand {
    /// Build the canonical φ-band EMA (`α = α_φ`).
    pub fn new() -> Self {
        Self::with_alpha(PHI_BAND_ALPHA).expect("PHI_BAND_ALPHA is INV-8 proven and must lie in [0.10, 0.13]")
    }

    /// Build with a custom α — fails if α is outside the proven φ-band.
    ///
    /// Useful only for property tests that *intentionally* try to land
    /// out-of-band α and prove the writer rejects them.
    pub fn with_alpha(alpha: f64) -> Result<Self, WriteErr> {
        if !alpha.is_finite() || !(PHI_BAND_LOW..=PHI_BAND_HIGH).contains(&alpha) {
            return Err(WriteErr::PhiBandOutOfRange { alpha });
        }
        Ok(Self { alpha, state: None })
    }

    /// Current α.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }

    /// Feed one BPB observation; returns the new EMA value.
    pub fn update(&mut self, value: f64) -> f64 {
        let next = match self.state {
            None => value,
            Some(prev) => self.alpha * value + (1.0 - self.alpha) * prev,
        };
        self.state = Some(next);
        next
    }

    /// Current EMA state (`None` until the first `update`).
    pub fn state(&self) -> Option<f64> {
        self.state
    }
}

impl Default for EmaPhiBand {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────── BPB sink trait ────────────────────────────────

/// Sink that accepts one [`BpbSampleRow`] and returns when the row is
/// durable. Implementations land in BR-IO rings (sqlx / tokio-postgres
/// adapter) and are mocked here in unit tests.
pub trait BpbSink {
    /// Persist one row. MUST return `Ok(())` only after durability.
    fn put<'a>(
        &'a mut self,
        row: &'a BpbSampleRow,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

// ─────────────────── BpbWriter (composes EMA + Sink) ───────────────

/// `BpbWriter` is the canonical write path: it tags every row with the
/// φ-band EMA and forwards it to a [`BpbSink`].
///
/// One writer is bound to exactly one [`Scarab`] (one job).
#[derive(Debug)]
pub struct BpbWriter {
    job_id: JobId,
    ema: EmaPhiBand,
}

impl BpbWriter {
    /// Build a writer for the given scarab.
    pub fn for_scarab(scarab: &Scarab) -> Self {
        Self {
            job_id: scarab.job_id,
            ema: EmaPhiBand::new(),
        }
    }

    /// Build a writer for an explicit job id (test helper).
    pub fn for_job(job_id: JobId) -> Self {
        Self {
            job_id,
            ema: EmaPhiBand::new(),
        }
    }

    /// Bound job id.
    pub fn job_id(&self) -> JobId {
        self.job_id
    }

    /// Current EMA (None until first row).
    pub fn ema(&self) -> Option<f64> {
        self.ema.state()
    }

    /// Tag `row.ema` with the φ-band EMA and forward to the sink.
    ///
    /// O(1): one EMA update + one sink call. Returns `WriteErr::JobIdMismatch`
    /// if the row was generated for a different job.
    pub async fn write_one<S: BpbSink + ?Sized>(
        &mut self,
        sink: &mut S,
        row: &BpbSampleRow,
    ) -> Result<BpbSampleRow, WriteErr> {
        if row.job_id != self.job_id {
            return Err(WriteErr::JobIdMismatch {
                row_job_id: row.job_id,
                writer_job_id: self.job_id,
            });
        }
        let ema = self.ema.update(row.bpb);
        let stamped = BpbSampleRow {
            job_id: row.job_id,
            step: row.step,
            bpb: row.bpb,
            ema: Some(ema),
            ts: row.ts,
        };
        sink.put(&stamped)
            .await
            .map_err(WriteErr::Sink)?;
        Ok(stamped)
    }
}

// ─────────────────── Embedded SQL schema ───────────────────────────

/// Idempotent schema for the BPB write path.
///
/// Apply once per Neon database. SR-04 gardener and SR-02
/// trainer-runner both depend on these tables existing.
pub const SCHEMA_SQL: &str = include_str!("../schema.sql");

// ─────────────────── tests ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use trios_igla_race_pipeline_sr_00::{Seed, StrategyId};

    fn fixed_ts() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 5, 2, 9, 0, 0).unwrap()
    }

    fn sample(job_id: JobId, step: i64, bpb: f64) -> BpbSampleRow {
        BpbSampleRow {
            job_id,
            step,
            bpb,
            ema: None,
            ts: fixed_ts(),
        }
    }

    /// Mock sink — collects every row in memory.
    #[derive(Default)]
    struct VecSink(Vec<BpbSampleRow>);

    impl BpbSink for VecSink {
        fn put<'a>(
            &'a mut self,
            row: &'a BpbSampleRow,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async move {
                self.0.push(row.clone());
                Ok(())
            })
        }
    }

    /// Failing sink — for I/O error path.
    struct FailSink;
    impl BpbSink for FailSink {
        fn put<'a>(
            &'a mut self,
            _row: &'a BpbSampleRow,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            Box::pin(async { Err("disk on fire".into()) })
        }
    }

    #[test]
    fn phi_band_alpha_correct() {
        let alpha = PHI_BAND_ALPHA;
        // α_φ = φ⁻³ / 2 ≈ 0.118033988749895
        assert!(
            alpha > 0.117 && alpha < 0.119,
            "α_φ outside expected window: {}",
            alpha
        );
        // Inside φ-band [0.10, 0.13]
        assert!((PHI_BAND_LOW..=PHI_BAND_HIGH).contains(&alpha));
    }

    #[test]
    fn ema_first_update_initialises() {
        let mut ema = EmaPhiBand::new();
        assert_eq!(ema.update(2.5), 2.5);
        assert_eq!(ema.state(), Some(2.5));
    }

    #[test]
    fn ema_update_converges() {
        let mut ema = EmaPhiBand::new();
        let target = 1.85;
        // Push the same value 200 times — EMA must converge to it.
        for _ in 0..200 {
            ema.update(target);
        }
        let v = ema.state().unwrap();
        assert!(
            (v - target).abs() < 1e-6,
            "EMA did not converge: got {}",
            v
        );
    }

    #[test]
    fn inv8_violation_triggers_error() {
        // α = 0.5 is outside the φ-band.
        match EmaPhiBand::with_alpha(0.5) {
            Err(WriteErr::PhiBandOutOfRange { alpha }) => {
                assert!((alpha - 0.5).abs() < 1e-9);
            }
            other => panic!("expected PhiBandOutOfRange, got {:?}", other),
        }
    }

    #[test]
    fn inv8_lower_bound_inclusive() {
        assert!(EmaPhiBand::with_alpha(PHI_BAND_LOW).is_ok());
        assert!(EmaPhiBand::with_alpha(PHI_BAND_LOW - 1e-9).is_err());
    }

    #[test]
    fn inv8_upper_bound_inclusive() {
        assert!(EmaPhiBand::with_alpha(PHI_BAND_HIGH).is_ok());
        assert!(EmaPhiBand::with_alpha(PHI_BAND_HIGH + 1e-9).is_err());
    }

    #[tokio::test]
    async fn write_one_mock_success() {
        let scarab = Scarab::queued(StrategyId::new(), Seed(43), serde_json::json!({}));
        let mut writer = BpbWriter::for_scarab(&scarab);
        let mut sink = VecSink::default();
        let row = sample(scarab.job_id, 1000, 2.34);
        let stamped = writer.write_one(&mut sink, &row).await.unwrap();
        assert_eq!(stamped.bpb, 2.34);
        // First sample → EMA = bpb
        assert_eq!(stamped.ema, Some(2.34));
        assert_eq!(sink.0.len(), 1);
        assert_eq!(sink.0[0].ema, Some(2.34));
    }

    #[tokio::test]
    async fn write_one_o1_latency() {
        // Smoke: 1000 in-memory writes finish under 100 ms.
        // Per-call avg < 100 µs ⇒ p99 < 10 ms holds with the real Neon path
        // dominated by network, not by our O(1) work.
        let scarab = Scarab::queued(StrategyId::new(), Seed(1), serde_json::json!({}));
        let mut writer = BpbWriter::for_scarab(&scarab);
        let mut sink = VecSink::default();
        let start = std::time::Instant::now();
        for step in 0..1000 {
            let row = sample(scarab.job_id, step, 2.0 + (step as f64) * 1e-6);
            writer.write_one(&mut sink, &row).await.unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed.as_millis() < 100,
            "1000 writes took {:?} (expected <100ms)",
            elapsed
        );
        assert_eq!(sink.0.len(), 1000);
    }

    #[tokio::test]
    async fn write_one_propagates_sink_error() {
        let scarab = Scarab::queued(StrategyId::new(), Seed(1), serde_json::json!({}));
        let mut writer = BpbWriter::for_scarab(&scarab);
        let mut sink = FailSink;
        let row = sample(scarab.job_id, 1, 2.5);
        match writer.write_one(&mut sink, &row).await {
            Err(WriteErr::Sink(msg)) => assert!(msg.contains("fire")),
            other => panic!("expected Sink error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn write_one_rejects_mismatched_job_id() {
        let scarab = Scarab::queued(StrategyId::new(), Seed(1), serde_json::json!({}));
        let mut writer = BpbWriter::for_scarab(&scarab);
        let mut sink = VecSink::default();
        let other = JobId::new();
        let row = sample(other, 1, 2.0);
        match writer.write_one(&mut sink, &row).await {
            Err(WriteErr::JobIdMismatch {
                row_job_id,
                writer_job_id,
            }) => {
                assert_eq!(row_job_id, other);
                assert_eq!(writer_job_id, scarab.job_id);
            }
            other => panic!("expected JobIdMismatch, got {:?}", other),
        }
        assert!(sink.0.is_empty(), "sink must NOT be touched on mismatch");
    }

    #[test]
    fn bpb_row_serde_roundtrip() {
        let row = sample(JobId::new(), 1000, 2.34);
        let s = serde_json::to_string(&row).unwrap();
        let back: BpbSampleRow = serde_json::from_str(&s).unwrap();
        assert_eq!(row, back);
    }

    #[test]
    fn schema_sql_idempotent_keywords_present() {
        // Cheap idempotency check: every CREATE statement must use
        // `IF NOT EXISTS` or `OR REPLACE`. Detect any statement that
        // starts with bare `CREATE TABLE` without `IF NOT EXISTS`.
        for line in SCHEMA_SQL.lines() {
            let l = line.trim_start();
            if l.starts_with("CREATE TABLE") {
                assert!(
                    l.contains("IF NOT EXISTS"),
                    "non-idempotent CREATE TABLE: {}",
                    l
                );
            }
            if l.starts_with("CREATE INDEX") {
                assert!(
                    l.contains("IF NOT EXISTS"),
                    "non-idempotent CREATE INDEX: {}",
                    l
                );
            }
        }
    }

    #[test]
    fn schema_sql_mentions_required_tables() {
        for table in ["scarabs", "bpb_samples", "heartbeats"] {
            assert!(
                SCHEMA_SQL.contains(table),
                "schema.sql missing table '{}'",
                table
            );
        }
    }
}
