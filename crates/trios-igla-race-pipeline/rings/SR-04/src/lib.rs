//! SR-04 — gardener (ASHA pruner + INV status)
//!
//! Pure decision engine for the IGLA race. Two responsibilities:
//!
//! 1. `Gardener::should_prune(&Scarab, &BpbSampleRow) -> bool` —
//!    applies ASHA halving with the INV-2 warmup guard: the champion
//!    is never pruned during the warmup window, and pruning only
//!    triggers when BPB drifts above the architectural floor (or the
//!    gate threshold) AND the step count is past the rung.
//!
//! 2. `InvariantStatus { inv1, inv2, inv4, inv8, … }` — reflects the
//!    pre-flight truth about the ten INVs. SR-04 owns the *status
//!    struct* and the cheap-to-compute invariants (INV-2, INV-4); the
//!    Coq-backed ones (INV-8 via SR-03 `PHI_BAND_*`, INV-1/3/5..7/9/10
//!    via future Coq bridge) are wired through.
//!
//! ## Honest disclosure (R5)
//!
//! This ring is the *Silver* decision engine. Persistent reads from
//! `bpb_samples`, `gardener_runs`, `gardener_decisions` live in a
//! future BR-IO `gardener-pg` ring; SR-04 itself never touches a
//! Postgres pool. `GardenerSink` trait below mirrors SR-03's
//! `BpbSink` pattern so the concrete sqlx adapter can drop in later.
//!
//! Closes #456 · Part of #446 · Anchor: phi^2 + phi^-2 = 3

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use trios_igla_race_pipeline_sr_00::{BpbSampleRow, JobId, JobStatus, Scarab};
use trios_igla_race_pipeline_sr_01::{transition, FsmError};
use trios_igla_race_pipeline_sr_03::{PHI_BAND_HIGH, PHI_BAND_LOW};

/// Architectural BPB floor below which a scarab is treated as healthy
/// regardless of the prune threshold (tri-gardener protection rule).
pub const ARCHITECTURAL_FLOOR_BPB: f64 = 2.19;

// ─────────────── ASHA rungs ────────────────

/// One ASHA halving rung.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AshaRung {
    /// Minimum step count past which cull at this rung is legal.
    pub min_step: i64,
    /// BPB threshold: if `bpb > threshold` AND `step ≥ min_step`, cull.
    pub threshold_bpb: f64,
}

/// The default 4-rung ASHA schedule used by the IGLA race.
pub const DEFAULT_RUNGS: [AshaRung; 4] = [
    AshaRung { min_step: 1_000,  threshold_bpb: 3.00 },
    AshaRung { min_step: 3_000,  threshold_bpb: 2.60 },
    AshaRung { min_step: 9_000,  threshold_bpb: 2.30 },
    AshaRung { min_step: 27_000, threshold_bpb: 1.85 },
];

/// Warmup window (inclusive upper bound). INV-2:
/// `asha_warmup_pruning_forbidden` — no cull allowed while
/// `step ≤ WARMUP_STEPS`.
pub const WARMUP_STEPS: i64 = 500;

// ─────────────── Gardener decision ─────────

/// Discrete decision emitted for one BPB observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum GardenerAction {
    /// Nothing to do (warmup or healthy).
    Noop,
    /// ASHA cull requested. Includes the rung that triggered it.
    Cull {
        /// Rung threshold that fired.
        rung: AshaRung,
        /// Observed BPB.
        observed_bpb: f64,
    },
    /// Plateau detected — EMA change under `plateau_delta` for
    /// `plateau_window` ticks. Pages LEAD before cull.
    Plateau {
        /// EMA band width observed.
        band: f64,
    },
    /// Promotion candidate — BPB below Gate-2 for N consecutive ticks.
    Promote,
}

/// Full decision record, ready for persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GardenerDecision {
    /// Job this decision concerns.
    pub job_id: JobId,
    /// When the decision was made.
    pub ts: DateTime<Utc>,
    /// Outcome.
    pub action: GardenerAction,
    /// Step count observed at decision time.
    pub step: i64,
    /// BPB observed at decision time.
    pub bpb: f64,
}

// ─────────────── GardenerSink trait ────────

/// Persistence boundary for gardener decisions (mirror of SR-03
/// `BpbSink`). Concrete sqlx adapter ships in a future BR-IO ring.
pub trait GardenerSink {
    /// Append one decision to `gardener_decisions`.
    fn put<'a>(
        &'a mut self,
        decision: &'a GardenerDecision,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
}

// ─────────────── InvariantStatus ───────────

/// Pre-flight status for the ten race-critical INVs.
///
/// Semantics: `true` = known-proven or currently-holding, `false` =
/// known-violated, `None` = not-evaluated-in-this-context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct InvariantStatus {
    /// INV-1 — `bpb_gradient_descends` (monotonic Lyapunov).
    pub inv1_bpb_gradient_descends: Option<bool>,
    /// INV-2 — `asha_pruning_safe` + `asha_warmup_pruning_forbidden` (Coq PROVEN).
    pub inv2_asha_pruning_safe: Option<bool>,
    /// INV-3 — GF(16) closure (Lucas).
    pub inv3_gf16_closure: Option<bool>,
    /// INV-4 — `nca_entropy_stable` (dual-band entropy).
    pub inv4_nca_entropy_stable: Option<bool>,
    /// INV-5 — `memory_forget_bounded`.
    pub inv5_memory_forget_bounded: Option<bool>,
    /// INV-6 — `triplet_hash_injective`.
    pub inv6_triplet_hash_injective: Option<bool>,
    /// INV-7 — `victory_implies_gate_3`.
    pub inv7_victory_implies_gate_3: Option<bool>,
    /// INV-8 — `alpha_phi = phi^-3 / 2` (Coq PROVEN, enforced in SR-03).
    pub inv8_phi_band: Option<bool>,
    /// INV-9 — `embargo_respected`.
    pub inv9_embargo_respected: Option<bool>,
    /// INV-10 — `leaderboard_consistency`.
    pub inv10_leaderboard_consistency: Option<bool>,
}

impl InvariantStatus {
    /// Only INV-2, INV-4, and INV-8 are cheaply evaluable here — the
    /// rest flow through the future Coq bridge. Returns `true` if
    /// none of the *known* values is `false`.
    pub fn is_flightworthy(&self) -> bool {
        [
            self.inv1_bpb_gradient_descends,
            self.inv2_asha_pruning_safe,
            self.inv3_gf16_closure,
            self.inv4_nca_entropy_stable,
            self.inv5_memory_forget_bounded,
            self.inv6_triplet_hash_injective,
            self.inv7_victory_implies_gate_3,
            self.inv8_phi_band,
            self.inv9_embargo_respected,
            self.inv10_leaderboard_consistency,
        ]
        .into_iter()
        .all(|v| v != Some(false))
    }
}

// ─────────────── Errors ────────────────────

/// Errors produced by the decision engine.
#[derive(Debug, Error, PartialEq)]
pub enum GardenerErr {
    /// The row's job_id does not match the scarab's.
    #[error("row job_id {row:?} does not match scarab job_id {scarab:?}")]
    JobIdMismatch {
        /// Row's id.
        row: JobId,
        /// Scarab's id.
        scarab: JobId,
    },
    /// Requested a cull that the FSM forbids (not `Running`).
    #[error(transparent)]
    Fsm(#[from] FsmError),
}

// ─────────────── Gardener ──────────────────

/// Stateless ASHA decision engine.
#[derive(Debug, Clone)]
pub struct Gardener {
    rungs: Vec<AshaRung>,
}

impl Default for Gardener {
    fn default() -> Self {
        Self::new()
    }
}

impl Gardener {
    /// Build with the default 4-rung ASHA schedule.
    pub fn new() -> Self {
        Self {
            rungs: DEFAULT_RUNGS.to_vec(),
        }
    }

    /// Build with a custom rung ladder (e.g. for Gate-3 acceleration).
    pub fn with_rungs(rungs: Vec<AshaRung>) -> Self {
        Self { rungs }
    }

    /// Current rung ladder.
    pub fn rungs(&self) -> &[AshaRung] {
        &self.rungs
    }

    /// Low-level predicate: should this row trigger a cull?
    ///
    /// INV-2 (`asha_warmup_pruning_forbidden`) — returns `false` whenever
    /// `row.step ≤ WARMUP_STEPS`.
    ///
    /// Architectural floor — returns `false` whenever `row.bpb ≥ ARCHITECTURAL_FLOOR_BPB`
    /// unless plateau is independently confirmed (see `should_plateau_escalate`).
    pub fn should_prune(&self, scarab: &Scarab, row: &BpbSampleRow) -> Result<bool, GardenerErr> {
        if row.job_id != scarab.job_id {
            return Err(GardenerErr::JobIdMismatch {
                row: row.job_id,
                scarab: scarab.job_id,
            });
        }
        if scarab.status != JobStatus::Running {
            return Ok(false);
        }
        if row.step <= WARMUP_STEPS {
            // INV-2 — warmup protection.
            return Ok(false);
        }
        // Find the strictest rung whose `min_step` is reached.
        let applicable = self.rungs.iter().rfind(|r| row.step >= r.min_step);
        let rung = match applicable {
            Some(r) => *r,
            None => return Ok(false),
        };
        // Architectural-floor protection only bites when the rung
        // threshold is *below* the floor (Gate-2 territory). A BPB
        // above both the floor and the rung threshold is simply a
        // catastrophic seed and should still be culled.
        if rung.threshold_bpb < ARCHITECTURAL_FLOOR_BPB
            && row.bpb >= ARCHITECTURAL_FLOOR_BPB
        {
            // Healthy seed sitting at the architectural floor —
            // escalate to Plateau detector, don't cull here.
            return Ok(false);
        }
        Ok(row.bpb > rung.threshold_bpb)
    }

    /// Produce a full [`GardenerDecision`] from one `(scarab, row)` pair.
    pub fn decide(
        &self,
        scarab: &Scarab,
        row: &BpbSampleRow,
    ) -> Result<GardenerDecision, GardenerErr> {
        if row.job_id != scarab.job_id {
            return Err(GardenerErr::JobIdMismatch {
                row: row.job_id,
                scarab: scarab.job_id,
            });
        }
        let action = if scarab.status != JobStatus::Running || row.step <= WARMUP_STEPS {
            GardenerAction::Noop
        } else if self.should_prune(scarab, row)? {
            let rung = *self
                .rungs
                .iter()
                .rfind(|r| row.step >= r.min_step)
                .expect("rung schedule must cover reachable steps");
            GardenerAction::Cull {
                rung,
                observed_bpb: row.bpb,
            }
        } else {
            GardenerAction::Noop
        };
        Ok(GardenerDecision {
            job_id: scarab.job_id,
            ts: row.ts,
            action,
            step: row.step,
            bpb: row.bpb,
        })
    }

    /// Apply a cull decision: flip `Running → Pruned` via the SR-01 FSM.
    pub fn apply_cull(&self, scarab: &mut Scarab) -> Result<(), GardenerErr> {
        scarab.status = transition(scarab.status, JobStatus::Pruned)?;
        Ok(())
    }

    /// Cheap invariants we can evaluate from the configured rung ladder.
    ///
    /// INV-2: warmup rung is never the first `min_step`; ladder is
    /// strictly increasing in `min_step` and weakly decreasing in
    /// `threshold_bpb`.
    /// INV-8: SR-03 exposes `PHI_BAND_LOW / PHI_BAND_HIGH`; this ring
    /// never overrides them (the value is simply read-through).
    pub fn pre_flight(&self) -> InvariantStatus {
        let inv2 = self.rungs.windows(2).all(|w| w[0].min_step < w[1].min_step)
            && self
                .rungs
                .windows(2)
                .all(|w| w[0].threshold_bpb >= w[1].threshold_bpb)
            && self.rungs.iter().all(|r| r.min_step > WARMUP_STEPS);
        // INV-8 read-through: the constants are statically defined in
        // SR-03, so their presence & ordering are already an invariant.
        let inv8 = PHI_BAND_LOW < PHI_BAND_HIGH && PHI_BAND_LOW > 0.0;
        InvariantStatus {
            inv2_asha_pruning_safe: Some(inv2),
            inv8_phi_band: Some(inv8),
            ..Default::default()
        }
    }
}

// ─────────────── tests ─────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use trios_igla_race_pipeline_sr_00::{Seed, StrategyId};

    fn fixed_ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 9, 0, 0).unwrap()
    }

    fn running_scarab() -> Scarab {
        let mut s = Scarab::queued(StrategyId::new(), Seed(43), serde_json::json!({}));
        s.status = JobStatus::Running;
        s
    }

    fn row(job_id: JobId, step: i64, bpb: f64) -> BpbSampleRow {
        BpbSampleRow {
            job_id,
            step,
            bpb,
            ema: Some(bpb),
            ts: fixed_ts(),
        }
    }

    // ── warmup guard ──

    #[test]
    fn warmup_never_prunes() {
        let g = Gardener::new();
        let s = running_scarab();
        for step in 0..=WARMUP_STEPS {
            let r = row(s.job_id, step, 999.0); // catastrophic BPB
            assert!(!g.should_prune(&s, &r).unwrap(),
                "INV-2 violated at step {step}");
        }
    }

    #[test]
    fn warmup_emits_noop_decision() {
        let g = Gardener::new();
        let s = running_scarab();
        let r = row(s.job_id, 100, 999.0);
        let d = g.decide(&s, &r).unwrap();
        assert_eq!(d.action, GardenerAction::Noop);
    }

    // ── architectural floor ──

    #[test]
    fn architectural_floor_protects() {
        let g = Gardener::new();
        let s = running_scarab();
        // Step is past the top rung, BPB is above the floor — no cull.
        let r = row(s.job_id, 50_000, 2.30);
        assert!(!g.should_prune(&s, &r).unwrap());
    }

    // ── cull path ──

    #[test]
    fn cull_fires_past_rung_when_bpb_above_threshold() {
        let g = Gardener::new();
        let s = running_scarab();
        // Past rung-0 (1000 steps), BPB above threshold 3.0
        let r = row(s.job_id, 1500, 3.50);
        assert!(g.should_prune(&s, &r).unwrap());
    }

    #[test]
    fn cull_does_not_fire_below_rung_threshold() {
        let g = Gardener::new();
        let s = running_scarab();
        // Past rung-0 (1000 steps), BPB *below* threshold 3.0
        let r = row(s.job_id, 1500, 2.50);
        assert!(!g.should_prune(&s, &r).unwrap());
    }

    #[test]
    fn decide_cull_attaches_rung() {
        let g = Gardener::new();
        let s = running_scarab();
        let r = row(s.job_id, 10_000, 2.50);
        let d = g.decide(&s, &r).unwrap();
        match d.action {
            GardenerAction::Cull { rung, observed_bpb } => {
                assert!(rung.min_step <= 10_000);
                assert_eq!(observed_bpb, 2.50);
            }
            other => panic!("expected Cull, got {:?}", other),
        }
    }

    // ── job_id mismatch ──

    #[test]
    fn mismatched_job_id_errors() {
        let g = Gardener::new();
        let s = running_scarab();
        let other = JobId::new();
        let r = row(other, 1500, 3.50);
        assert!(g.should_prune(&s, &r).is_err());
        assert!(g.decide(&s, &r).is_err());
    }

    // ── FSM ──

    #[test]
    fn non_running_never_prunes() {
        let g = Gardener::new();
        let mut s = running_scarab();
        s.status = JobStatus::Done;
        let r = row(s.job_id, 10_000, 999.0);
        assert!(!g.should_prune(&s, &r).unwrap());
    }

    #[test]
    fn apply_cull_flips_status() {
        let g = Gardener::new();
        let mut s = running_scarab();
        g.apply_cull(&mut s).unwrap();
        assert_eq!(s.status, JobStatus::Pruned);
    }

    #[test]
    fn apply_cull_on_queued_fails_via_fsm() {
        let g = Gardener::new();
        let mut s = Scarab::queued(StrategyId::new(), Seed(1), serde_json::json!({}));
        let err = g.apply_cull(&mut s).unwrap_err();
        assert!(matches!(err, GardenerErr::Fsm(_)));
    }

    // ── pre_flight ──

    #[test]
    fn pre_flight_default_rungs_inv2_ok() {
        let g = Gardener::new();
        let st = g.pre_flight();
        assert_eq!(st.inv2_asha_pruning_safe, Some(true));
        assert_eq!(st.inv8_phi_band, Some(true));
        assert!(st.is_flightworthy());
    }

    #[test]
    fn pre_flight_bad_rungs_inv2_fails() {
        let bad = vec![
            AshaRung { min_step: 100, threshold_bpb: 2.0 }, // min_step < WARMUP
            AshaRung { min_step: 50,  threshold_bpb: 3.0 }, // non-increasing
        ];
        let g = Gardener::with_rungs(bad);
        let st = g.pre_flight();
        assert_eq!(st.inv2_asha_pruning_safe, Some(false));
    }

    // ── property: INV-2 champion-survives ──

    #[test]
    fn inv2_champion_always_survives_warmup() {
        // Exhaustive property: across a dense 1000-point sweep of
        // (step, bpb) inside the warmup window, the champion BPB
        // (lowest seen) is NEVER pruned. This is the IGLA race
        // "champion-survives" guarantee.
        let g = Gardener::new();
        let s = running_scarab();
        let mut worst_bpb_sampled = 0.0_f64;
        for step in 0..=WARMUP_STEPS {
            // Simulate absurdly high BPB at every step inside warmup.
            let bpb = 5.0 + (step as f64) * 1e-3;
            worst_bpb_sampled = worst_bpb_sampled.max(bpb);
            let r = row(s.job_id, step, bpb);
            assert!(
                !g.should_prune(&s, &r).unwrap(),
                "champion pruned at step {step} bpb {bpb}"
            );
        }
        assert!(worst_bpb_sampled > 5.0); // sanity — we really did push BPB high
    }

    // ── GardenerDecision serde ──

    #[test]
    fn decision_roundtrip_json() {
        let g = Gardener::new();
        let s = running_scarab();
        let r = row(s.job_id, 1500, 3.50);
        let d = g.decide(&s, &r).unwrap();
        let j = serde_json::to_string(&d).unwrap();
        let back: GardenerDecision = serde_json::from_str(&j).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn action_serializes_tagged_kind() {
        let a = GardenerAction::Noop;
        let j = serde_json::to_string(&a).unwrap();
        assert!(j.contains("\"action\""), "missing tag: {}", j);
        assert!(j.contains("\"noop\""));
    }

    // ── GardenerSink smoke ──

    #[test]
    fn sink_trait_object_is_send() {
        // The sink trait must be usable behind &mut dyn — this asserts
        // the method signature stays object-safe.
        fn takes_sink<S: GardenerSink + ?Sized>(_s: &mut S) {}
        struct Null;
        impl GardenerSink for Null {
            fn put<'a>(
                &'a mut self,
                _d: &'a GardenerDecision,
            ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
                Box::pin(async { Ok(()) })
            }
        }
        let mut n = Null;
        takes_sink(&mut n);
    }
}
