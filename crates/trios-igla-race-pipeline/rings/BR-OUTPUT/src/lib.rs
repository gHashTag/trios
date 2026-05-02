//! BR-OUTPUT — `IglaRacePipeline` assembler ring.
//!
//! Bronze-tier ring under `trios-igla-race-pipeline` GOLD I crate. Ties
//! SR-00..04 into the canonical `run_e2e_ttt_o1()` end-to-end loop and
//! exposes the INV-7 victory gate (ported verbatim from the legacy
//! `trios-igla-race::victory` module).
//!
//! ## Surface
//!
//! - `IglaRacePipeline::new(cfg)` / `IglaRacePipeline::with_components(...)`
//! - `IglaRacePipeline::run_e2e_ttt_o1(&mut self, sinks, trainer) -> Result<VictoryReport, PipelineErr>`
//! - INV-7 victory gate: `SeedResult`, `VictoryReport`, `VictoryError`,
//!   `check_victory`, `is_victory`, `stat_strength`, `TtestReport`
//! - Pipeline types: `PipelineCfg`, `PipelineErr`, `TrainerBackend`,
//!   `MockedTrainer`, `Sinks`, `RunSummary`
//!
//! ## Honest disclosure (R5)
//!
//! - Real GPU training lives behind `TrainerBackend`. Until SR-02 ships
//!   its `PythonRunner`, `MockedTrainer` is the only honest backend —
//!   it emits a deterministic descending BPB curve keyed by `(seed,
//!   step)`, and every emitted `BpbSampleRow` is tagged via the writer
//!   chain so callers cannot mistake mocked output for a GPU run.
//! - `check_victory` here is byte-equivalent with the legacy
//!   `trios-igla-race::victory::check_victory`, including all
//!   constants: `BPB_VICTORY_TARGET = 1.5`, `VICTORY_SEED_TARGET = 3`,
//!   `INV2_WARMUP_BLIND_STEPS = 4000`, `JEPA_PROXY_BPB_FLOOR = 0.1`.
//!   The 19 falsifiers in `tests/victory_falsifiers.rs` are the
//!   verbatim port required by #459's acceptance criteria.
//!
//! ## Constitutional compliance
//!
//! - **R-RING-FACADE-001** — outer GOLD I `src/lib.rs` re-exports only.
//! - **R-RING-DEP-002** — Bronze-tier deps: SR-00..04 path deps plus
//!   `serde`, `chrono`, `thiserror`. Dev-only `tokio` for tests. NO
//!   sqlx, NO reqwest, NO subprocess — those belong to BR-IO.
//! - **R-RING-BR-004** — Bronze ring re-exposed via parent GOLD facade.
//! - **R-L6-PURE-007** — no `.py` here; trainer invocation is
//!   trait-gated and stubbed.
//! - **L13** — single-ring scope.
//! - **I5** — README.md, TASK.md, AGENTS.md, RING.md, Cargo.toml,
//!   src/lib.rs.
//!
//! Closes #459 · Part of #446 · Soul: Loop-Locksmith
//! Anchor: phi^2 + phi^-2 = 3

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use trios_igla_race_pipeline_sr_00::{
    BpbSampleRow, JobStatus, Scarab, Seed, StrategyId, WorkerId,
};
use trios_igla_race_pipeline_sr_01::{transition, FsmError};
use trios_igla_race_pipeline_sr_03::{BpbSink, BpbWriter, WriteErr};
use trios_igla_race_pipeline_sr_04::{
    Gardener, GardenerAction, GardenerDecision, GardenerErr, GardenerSink,
};

// ─────────────── INV-7 victory gate (verbatim port) ───────────────

/// Number of distinct seeds below `BPB_VICTORY_TARGET` required for
/// global victory. Mirrors `trios-igla-race::hive_automaton::VICTORY_SEED_TARGET`.
pub const VICTORY_SEED_TARGET: u32 = 3;

/// BPB threshold for victory (strict `<`).
/// Mirrors `trios-igla-race::hive_automaton::BPB_VICTORY_TARGET`.
pub const BPB_VICTORY_TARGET: f64 = 1.5;

/// Steps before which BPB measurements are blind.
/// Mirrors `trios-igla-race::invariants::INV2_WARMUP_BLIND_STEPS`.
pub const INV2_WARMUP_BLIND_STEPS: u64 = 4000;

/// Below this BPB after warmup, we suspect the JEPA-MSE-proxy artefact
/// (TASK-5D bug). The gate refuses such reports.
pub const JEPA_PROXY_BPB_FLOOR: f64 = 0.1;

/// Pre-registered Welch baseline μ₀ for `stat_strength`.
pub const TTEST_BASELINE_MU0: f64 = 1.55;

/// Pre-registered significance level for `stat_strength`.
pub const TTEST_ALPHA: f64 = 0.01;

/// Pre-registered minimum effect size.
pub const TTEST_EFFECT_SIZE_MIN: f64 = 0.05;

// Sanity at compile time.
const _: () = assert!((BPB_VICTORY_TARGET - 1.5).abs() < f64::EPSILON);
const _: () = assert!(VICTORY_SEED_TARGET == 3);

/// One observed seed result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeedResult {
    /// Seed used to drive the trial.
    pub seed: u64,
    /// Final BPB reported by the trial harness.
    pub bpb: f64,
    /// Step at which `bpb` was measured.
    pub step: u64,
    /// Commit SHA the trial ran against.
    pub sha: String,
}

/// Welch's two-sample t-test report (one-tailed, lower-than-baseline).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtestReport {
    /// t-statistic (negative when sample mean < baseline).
    pub t_statistic: f64,
    /// Welch–Satterthwaite degrees of freedom.
    pub df: f64,
    /// One-tailed p-value (lower tail).
    pub p_value: f64,
    /// Sample mean of winning seeds.
    pub sample_mean: f64,
    /// Sample standard deviation.
    pub sample_std: f64,
    /// Baseline μ₀.
    pub baseline_mu0: f64,
    /// Significance α.
    pub alpha: f64,
    /// Whether the test passed (p < α and t < 0).
    pub passed: bool,
}

/// Passing victory report — only `check_victory` may construct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VictoryReport {
    /// Distinct winning seeds, sorted ascending.
    pub winning_seeds: Vec<u64>,
    /// Lowest BPB among winning seeds.
    pub min_bpb: f64,
    /// Arithmetic mean of winning seeds' BPBs.
    pub mean_bpb: f64,
}

/// Reasons the gate refuses to declare victory.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum VictoryError {
    /// Fewer than `VICTORY_SEED_TARGET` distinct seeds passed.
    #[error("insufficient seeds: {passing_distinct} passing, {required} required")]
    InsufficientSeeds {
        /// Number of distinct seeds that passed.
        passing_distinct: usize,
        /// Number of distinct seeds required.
        required: usize,
    },
    /// At least one report has `bpb >= BPB_VICTORY_TARGET`.
    #[error("seed {seed} bpb {bpb} >= target {target}")]
    BpbAboveTarget {
        /// Offending seed.
        seed: u64,
        /// Reported BPB.
        bpb: f64,
        /// Target threshold.
        target: f64,
    },
    /// Same seed reported twice.
    #[error("duplicate seed {seed}")]
    DuplicateSeed {
        /// The duplicated seed.
        seed: u64,
    },
    /// `bpb < JEPA_PROXY_BPB_FLOOR` after warmup — TASK-5D bug.
    #[error("JEPA proxy detected on seed {seed} (bpb {bpb})")]
    JepaProxyDetected {
        /// Offending seed.
        seed: u64,
        /// Reported BPB.
        bpb: f64,
    },
    /// Reported step is below `INV2_WARMUP_BLIND_STEPS`.
    #[error("seed {seed} reported step {step} < warmup {warmup}")]
    BeforeWarmup {
        /// Offending seed.
        seed: u64,
        /// Reported step.
        step: u64,
        /// Warmup floor.
        warmup: u64,
    },
    /// `bpb` is non-finite.
    #[error("non-finite bpb on seed {seed} ({bpb})")]
    NonFiniteBpb {
        /// Offending seed.
        seed: u64,
        /// Reported BPB.
        bpb: f64,
    },
    /// Welch's t-test failed.
    #[error("Welch t-test failed: t={t_statistic}, p={p_value} (alpha={alpha})")]
    TtestFailed {
        /// Computed t-statistic.
        t_statistic: f64,
        /// Computed p-value.
        p_value: f64,
        /// Significance level used.
        alpha: f64,
    },
}

/// Adjudicate a victory claim. See `trios-igla-race::victory` for the
/// canonical contract — this is a verbatim port.
pub fn check_victory(results: &[SeedResult]) -> Result<VictoryReport, VictoryError> {
    // 1. duplicate seed detection
    let mut seen = HashSet::with_capacity(results.len());
    for r in results {
        if !seen.insert(r.seed) {
            return Err(VictoryError::DuplicateSeed { seed: r.seed });
        }
    }
    // 2. per-result soundness
    for r in results {
        if !r.bpb.is_finite() {
            return Err(VictoryError::NonFiniteBpb {
                seed: r.seed,
                bpb: r.bpb,
            });
        }
        if r.step < INV2_WARMUP_BLIND_STEPS {
            return Err(VictoryError::BeforeWarmup {
                seed: r.seed,
                step: r.step,
                warmup: INV2_WARMUP_BLIND_STEPS,
            });
        }
        if r.bpb < JEPA_PROXY_BPB_FLOOR {
            return Err(VictoryError::JepaProxyDetected {
                seed: r.seed,
                bpb: r.bpb,
            });
        }
    }
    // 3. distinct passing
    let passing: Vec<&SeedResult> = results
        .iter()
        .filter(|r| r.bpb < BPB_VICTORY_TARGET)
        .collect();
    if passing.len() < VICTORY_SEED_TARGET as usize {
        if let Some(r) = results.iter().find(|r| r.bpb >= BPB_VICTORY_TARGET) {
            return Err(VictoryError::BpbAboveTarget {
                seed: r.seed,
                bpb: r.bpb,
                target: BPB_VICTORY_TARGET,
            });
        }
        return Err(VictoryError::InsufficientSeeds {
            passing_distinct: passing.len(),
            required: VICTORY_SEED_TARGET as usize,
        });
    }
    // 4. assemble
    let mut winning_seeds: Vec<u64> = passing.iter().map(|r| r.seed).collect();
    winning_seeds.sort_unstable();
    winning_seeds.truncate(VICTORY_SEED_TARGET as usize);
    let bpbs: Vec<f64> = passing
        .iter()
        .take(VICTORY_SEED_TARGET as usize)
        .map(|r| r.bpb)
        .collect();
    let min_bpb = bpbs.iter().copied().fold(f64::INFINITY, f64::min);
    let mean_bpb = bpbs.iter().sum::<f64>() / bpbs.len() as f64;
    Ok(VictoryReport {
        winning_seeds,
        min_bpb,
        mean_bpb,
    })
}

/// Cheap predicate form for callers that only care whether victory is
/// reached.
pub fn is_victory(results: &[SeedResult]) -> bool {
    check_victory(results).is_ok()
}

/// Welch's t-test for statistical strength of a victory claim.
pub fn stat_strength(results: &[SeedResult]) -> Result<TtestReport, VictoryError> {
    if results.len() < 2 {
        return Err(VictoryError::InsufficientSeeds {
            passing_distinct: results.len(),
            required: 2,
        });
    }
    let n = results.len() as f64;
    let bpbs: Vec<f64> = results.iter().map(|r| r.bpb).collect();
    let sample_mean = bpbs.iter().sum::<f64>() / n;
    let var = bpbs
        .iter()
        .map(|x| (x - sample_mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let sample_std = var.sqrt();
    // One-sample t against fixed baseline (Welch reduces to one-sample
    // when σ_baseline → 0). df = n - 1.
    let se = sample_std / n.sqrt();
    let t_statistic = if se > 0.0 {
        (sample_mean - TTEST_BASELINE_MU0) / se
    } else {
        // perfect concentration; sign agrees with mean — baseline
        if sample_mean < TTEST_BASELINE_MU0 {
            f64::NEG_INFINITY
        } else if sample_mean > TTEST_BASELINE_MU0 {
            f64::INFINITY
        } else {
            0.0
        }
    };
    let df = n - 1.0;
    let p_value = t_cdf_lower_tail(t_statistic, df);
    let passed = t_statistic < 0.0 && p_value < TTEST_ALPHA;
    let report = TtestReport {
        t_statistic,
        df,
        p_value,
        sample_mean,
        sample_std,
        baseline_mu0: TTEST_BASELINE_MU0,
        alpha: TTEST_ALPHA,
        passed,
    };
    if !passed {
        return Err(VictoryError::TtestFailed {
            t_statistic,
            p_value,
            alpha: TTEST_ALPHA,
        });
    }
    Ok(report)
}

/// Lower-tail CDF of Student's t with `df` degrees of freedom.
/// Uses the regularised incomplete beta function (Numerical Recipes
/// 6.4). Adequate for df ≥ 1 and small sample sizes.
fn t_cdf_lower_tail(t: f64, df: f64) -> f64 {
    if !t.is_finite() {
        return if t < 0.0 { 0.0 } else { 1.0 };
    }
    let x = df / (df + t * t);
    let half_p = 0.5 * incomplete_beta(x, 0.5 * df, 0.5);
    if t < 0.0 {
        half_p
    } else {
        1.0 - half_p
    }
}

/// Regularised incomplete beta `I_x(a, b)`.
fn incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    // Continued fraction expansion (Numerical Recipes 6.4).
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let bt = (LnGamma::ln_gamma(a + b).0
        - LnGamma::ln_gamma(a).0
        - LnGamma::ln_gamma(b).0
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(x, a, b) / a
    } else {
        1.0 - bt * betacf(1.0 - x, b, a) / b
    }
}

fn betacf(x: f64, a: f64, b: f64) -> f64 {
    let max_iter = 200;
    let eps = 3e-7;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=max_iter {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;
        let aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + aa / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < eps {
            return h;
        }
    }
    h
}

// f64 doesn't expose ln_gamma; minimal implementation (Stirling
// continuation, Lanczos approximation g=7 n=9 — adequate for df ≥ 1).
trait LnGamma {
    fn ln_gamma(self) -> (f64, i32);
}
impl LnGamma for f64 {
    fn ln_gamma(self) -> (f64, i32) {
        // Lanczos g=7, n=9 coefficients.
        const G: f64 = 7.0;
        const C: [f64; 9] = [
            0.999_999_999_999_809_9,
            676.520_368_121_885_1,
            -1_259.139_216_722_402_8,
            771.323_428_777_653_2,
            -176.615_029_162_140_6,
            12.507_343_278_686_905,
            -0.138_571_095_265_720_1,
            9.984_369_578_019_572_e-6,
            1.505_632_735_149_311_6e-7,
        ];
        let x = self;
        if x < 0.5 {
            // reflection
            let (g, _) = LnGamma::ln_gamma(1.0 - x);
            let pi = std::f64::consts::PI;
            (pi.ln() - (pi * x).sin().abs().ln() - g, 1)
        } else {
            let z = x - 1.0;
            let mut a = C[0];
            for (i, ci) in C.iter().enumerate().skip(1) {
                a += ci / (z + i as f64);
            }
            let t = z + G + 0.5;
            let v = 0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + a.ln();
            (v, 1)
        }
    }
}

// ─────────────── Pipeline assembler ───────────────

/// Pipeline configuration — what knobs the assembler exposes to callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineCfg {
    /// Strategy id every spawned scarab is realising.
    pub strategy_id: StrategyId,
    /// Worker that owns the run.
    pub worker_id: WorkerId,
    /// Seeds the run will sweep, in order.
    pub seeds: Vec<Seed>,
    /// Steps per chunk emitted by the trainer (controls integration
    /// test runtime; production runs use 81_000 steps total).
    pub steps_per_chunk: i64,
    /// Total steps per seed (mock trainer stops after this many).
    pub total_steps: i64,
    /// Optional free-form trainer config attached to every Scarab.
    pub trainer_config: serde_json::Value,
}

impl PipelineCfg {
    /// Build a small, integration-test-sized config.
    pub fn smoke(seed: i64) -> Self {
        Self {
            strategy_id: StrategyId::new(),
            worker_id: WorkerId::new("test", 0),
            seeds: vec![Seed(seed)],
            steps_per_chunk: 1_000,
            total_steps: INV2_WARMUP_BLIND_STEPS as i64 + 2_000,
            trainer_config: serde_json::json!({"mock": true}),
        }
    }
}

/// Errors surfaced by `run_e2e_ttt_o1`.
#[derive(Debug, Error)]
pub enum PipelineErr {
    /// FSM transition rejected.
    #[error("fsm: {0}")]
    Fsm(#[from] FsmError),
    /// BPB write failed.
    #[error("bpb write: {0}")]
    BpbWrite(#[from] WriteErr),
    /// Gardener decision/apply failed.
    #[error("gardener: {0}")]
    Gardener(#[from] GardenerErr),
    /// Trainer backend produced an error.
    #[error("trainer: {0}")]
    Trainer(String),
    /// Sink reported an error.
    #[error("sink: {0}")]
    Sink(String),
    /// No seeds in `PipelineCfg::seeds`.
    #[error("config has no seeds")]
    EmptySeeds,
    /// Run exceeded `total_steps` without convergence — not an error
    /// per se, but caller asked for `run_e2e_ttt_o1` which expects a
    /// VictoryReport. Returned when `is_victory(results) == false`
    /// AFTER the sweep completes.
    #[error("run completed without victory ({passing}/{required} seeds passed)")]
    HonestNotYet {
        /// Distinct passing seeds at end of sweep.
        passing: usize,
        /// Required seeds.
        required: usize,
    },
}

/// Pluggable trainer backend.
///
/// One call to `step_one_chunk` advances the trainer by `chunk_size`
/// steps and returns one `BpbSampleRow` whose `step = last_step +
/// chunk_size`. Real SR-02 trainer ships the GPU implementation in
/// BR-IO; until then, `MockedTrainer` is the only honest backend.
pub trait TrainerBackend {
    /// Advance the seed by `chunk_size` steps, return one row.
    fn step_one_chunk<'a>(
        &'a mut self,
        scarab: &'a Scarab,
        last_step: i64,
        chunk_size: i64,
    ) -> Pin<Box<dyn Future<Output = Result<BpbSampleRow, String>> + Send + 'a>>;
}

/// Deterministic mock trainer — emits a smooth descending BPB curve.
///
/// `bpb(step) = floor + (start - floor) * exp(-decay * (step / 10_000))`
/// with `floor` configured per seed so the integration test can verify
/// the gardener / writer / FSM wiring without a GPU.
#[derive(Debug, Clone)]
pub struct MockedTrainer {
    /// Final BPB the curve approaches as `step → ∞`.
    pub floor: f64,
    /// Starting BPB at step 0.
    pub start: f64,
    /// Time constant of the descent.
    pub decay: f64,
}

impl MockedTrainer {
    /// Default curve: 1.35 floor, 2.0 start, 8.0 decay. Picked so the
    /// curve stays under every `DEFAULT_RUNGS` threshold (3.00 / 2.60
    /// / 2.30 / 1.85 at steps 1k / 3k / 9k / 27k) and clears
    /// `BPB_VICTORY_TARGET` post-warmup. The integration test relies
    /// on this shape.
    pub fn winning() -> Self {
        Self {
            floor: 1.35,
            start: 2.0,
            decay: 8.0,
        }
    }

    /// Curve that stays under every ASHA rung but never crosses the
    /// victory target — for `HonestNotYet` tests.
    pub fn losing() -> Self {
        Self {
            floor: 1.55,
            start: 2.0,
            decay: 8.0,
        }
    }

    /// Sample the curve at one step.
    pub fn sample(&self, step: i64) -> f64 {
        let t = step as f64 / 10_000.0;
        self.floor + (self.start - self.floor) * (-self.decay * t).exp()
    }
}

impl TrainerBackend for MockedTrainer {
    fn step_one_chunk<'a>(
        &'a mut self,
        scarab: &'a Scarab,
        last_step: i64,
        chunk_size: i64,
    ) -> Pin<Box<dyn Future<Output = Result<BpbSampleRow, String>> + Send + 'a>> {
        Box::pin(async move {
            let step = last_step + chunk_size.max(1);
            let bpb = self.sample(step);
            Ok(BpbSampleRow {
                job_id: scarab.job_id,
                step,
                bpb,
                ema: None,
                ts: Utc::now(),
            })
        })
    }
}

/// Sink bundle — pipeline writes BPB rows here and gardener decisions
/// here. Concrete sqlx adapters live in BR-IO; the integration test
/// uses `VecSink` and `VecGardenerSink` from this module.
pub struct Sinks<'a, B: BpbSink + ?Sized, G: GardenerSink + ?Sized> {
    /// BPB sample sink.
    pub bpb: &'a mut B,
    /// Gardener decision sink.
    pub gardener: &'a mut G,
}

/// Per-seed run summary — produced for every seed regardless of outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerSeedSummary {
    /// Seed.
    pub seed: Seed,
    /// Final BPB (last row written).
    pub final_bpb: f64,
    /// Final step.
    pub final_step: i64,
    /// Whether this seed was culled by the gardener.
    pub culled: bool,
    /// Final FSM status.
    pub final_status: JobStatus,
}

/// Result of a full sweep.
#[derive(Debug, Clone)]
pub struct RunSummary {
    /// Per-seed records, in input order.
    pub per_seed: Vec<PerSeedSummary>,
    /// Victory report if the sweep reached the gate.
    pub victory: Option<VictoryReport>,
}

/// The pipeline assembler.
pub struct IglaRacePipeline {
    cfg: PipelineCfg,
    gardener: Gardener,
}

impl IglaRacePipeline {
    /// Build with the default ASHA gardener.
    pub fn new(cfg: PipelineCfg) -> Self {
        Self {
            cfg,
            gardener: Gardener::new(),
        }
    }

    /// Build with a caller-supplied gardener (e.g. Gate-3 rung schedule).
    pub fn with_gardener(cfg: PipelineCfg, gardener: Gardener) -> Self {
        Self { cfg, gardener }
    }

    /// Borrow the configured pipeline knobs.
    pub fn cfg(&self) -> &PipelineCfg {
        &self.cfg
    }

    /// Run the canonical end-to-end loop.
    ///
    /// For every seed in `cfg.seeds`:
    /// 1. spawn `Scarab::queued`, transition `Queued → Running`.
    /// 2. while `step < cfg.total_steps`: trainer chunk → writer → gardener.
    /// 3. on `GardenerAction::Cull`: `apply_cull` and break.
    /// 4. otherwise: transition `Running → Done` after final chunk.
    /// 5. Append a `SeedResult` for the INV-7 victory gate.
    ///
    /// On completion, `check_victory` runs over the collected results;
    /// `Ok(report)` if 3+ distinct seeds cleared `BPB_VICTORY_TARGET`,
    /// `Err(PipelineErr::HonestNotYet)` otherwise.
    pub async fn run_e2e_ttt_o1<B, G, T>(
        &mut self,
        sinks: Sinks<'_, B, G>,
        trainer: &mut T,
    ) -> Result<VictoryReport, PipelineErr>
    where
        B: BpbSink + ?Sized,
        G: GardenerSink + ?Sized,
        T: TrainerBackend + ?Sized,
    {
        if self.cfg.seeds.is_empty() {
            return Err(PipelineErr::EmptySeeds);
        }
        let Sinks { bpb, gardener } = sinks;
        let mut summary = RunSummary {
            per_seed: Vec::with_capacity(self.cfg.seeds.len()),
            victory: None,
        };
        let mut seed_results = Vec::<SeedResult>::with_capacity(self.cfg.seeds.len());

        for &seed in &self.cfg.seeds {
            let mut scarab = Scarab::queued(
                self.cfg.strategy_id,
                seed,
                self.cfg.trainer_config.clone(),
            );
            // Queued → Running (worker picks up the job)
            scarab.worker_id = Some(self.cfg.worker_id.clone());
            scarab.started_at = Some(Utc::now());
            scarab.status = transition(scarab.status, JobStatus::Running)?;

            let mut writer = BpbWriter::for_scarab(&scarab);
            let mut last_step: i64 = 0;
            let mut last_row: Option<BpbSampleRow> = None;
            let mut culled = false;

            while last_step < self.cfg.total_steps {
                // One chunk: trainer step → writer → gardener.
                let raw_row = trainer
                    .step_one_chunk(&scarab, last_step, self.cfg.steps_per_chunk)
                    .await
                    .map_err(PipelineErr::Trainer)?;
                let stamped = writer.write_one(bpb, &raw_row).await?;
                let decision = self.gardener.decide(&scarab, &stamped)?;
                gardener
                    .put(&decision)
                    .await
                    .map_err(PipelineErr::Sink)?;
                last_step = stamped.step;
                last_row = Some(stamped.clone());

                if let GardenerAction::Cull { .. } = decision.action {
                    self.gardener.apply_cull(&mut scarab)?;
                    culled = true;
                    break;
                }

                // Track best-bpb on the scarab for completeness.
                let new_best = match scarab.best_bpb {
                    Some(b) if b <= stamped.bpb => b,
                    _ => stamped.bpb,
                };
                if Some(new_best) != scarab.best_bpb {
                    scarab.best_bpb = Some(new_best);
                    scarab.best_step = Some(stamped.step);
                }
            }

            if !culled {
                scarab.status = transition(scarab.status, JobStatus::Done)?;
                scarab.completed_at = Some(Utc::now());
            }

            let final_row = last_row.expect("at least one chunk per seed");
            summary.per_seed.push(PerSeedSummary {
                seed,
                final_bpb: final_row.bpb,
                final_step: final_row.step,
                culled,
                final_status: scarab.status,
            });
            // INV-7 only admits post-warmup, finite, non-proxy seeds —
            // we feed it the raw final row so the gate adjudicates.
            seed_results.push(SeedResult {
                seed: seed.0 as u64,
                bpb: final_row.bpb,
                step: final_row.step.max(0) as u64,
                sha: format!("mock:{}", scarab.job_id),
            });
        }

        match check_victory(&seed_results) {
            Ok(report) => {
                summary.victory = Some(report.clone());
                Ok(report)
            }
            Err(_) => {
                let passing = seed_results
                    .iter()
                    .filter(|r| r.bpb < BPB_VICTORY_TARGET)
                    .count();
                Err(PipelineErr::HonestNotYet {
                    passing,
                    required: VICTORY_SEED_TARGET as usize,
                })
            }
        }
    }
}

// ─────────────── unit tests (trait wiring + smoke) ───────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cfg_smoke_has_one_seed_and_post_warmup_total() {
        let c = PipelineCfg::smoke(42);
        assert_eq!(c.seeds.len(), 1);
        assert!(c.total_steps > INV2_WARMUP_BLIND_STEPS as i64);
    }

    #[test]
    fn mocked_trainer_descends_monotonically() {
        let mt = MockedTrainer::winning();
        let s0 = mt.sample(0);
        let s1 = mt.sample(5_000);
        let s2 = mt.sample(50_000);
        assert!(s0 > s1);
        assert!(s1 > s2);
        assert!((s2 - mt.floor).abs() < 0.5);
    }

    #[test]
    fn mocked_trainer_winning_clears_target_post_warmup() {
        let mt = MockedTrainer::winning();
        assert!(mt.sample(INV2_WARMUP_BLIND_STEPS as i64 + 2_000) < BPB_VICTORY_TARGET);
    }

    #[test]
    fn mocked_trainer_losing_never_clears_target() {
        let mt = MockedTrainer::losing();
        assert!(mt.sample(1_000_000) > BPB_VICTORY_TARGET);
    }

    #[test]
    fn check_victory_accepts_three_distinct() {
        let r = vec![
            SeedResult {
                seed: 1,
                bpb: 1.49,
                step: INV2_WARMUP_BLIND_STEPS + 1,
                sha: "a".into(),
            },
            SeedResult {
                seed: 2,
                bpb: 1.45,
                step: INV2_WARMUP_BLIND_STEPS + 1,
                sha: "b".into(),
            },
            SeedResult {
                seed: 3,
                bpb: 1.40,
                step: INV2_WARMUP_BLIND_STEPS + 1,
                sha: "c".into(),
            },
        ];
        let rep = check_victory(&r).expect("should pass");
        assert_eq!(rep.winning_seeds, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn pipeline_empty_seeds_errors() {
        let cfg = PipelineCfg {
            seeds: vec![],
            ..PipelineCfg::smoke(0)
        };
        let mut p = IglaRacePipeline::new(cfg);
        let mut bpb = test_sinks::VecBpbSink::default();
        let mut gd = test_sinks::VecGardenerSink::default();
        let mut tr = MockedTrainer::winning();
        let result = p
            .run_e2e_ttt_o1(
                Sinks {
                    bpb: &mut bpb,
                    gardener: &mut gd,
                },
                &mut tr,
            )
            .await;
        assert!(matches!(result, Err(PipelineErr::EmptySeeds)));
    }
}

/// In-memory sinks used by the integration test and unit tests. Kept
/// `pub` so downstream tests can re-use them.
pub mod test_sinks {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;

    /// In-memory `BpbSink` that records every row.
    #[derive(Default)]
    pub struct VecBpbSink {
        /// Captured rows.
        pub rows: Vec<BpbSampleRow>,
    }

    impl BpbSink for VecBpbSink {
        fn put<'a>(
            &'a mut self,
            row: &'a BpbSampleRow,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            self.rows.push(row.clone());
            Box::pin(async { Ok(()) })
        }
    }

    /// In-memory `GardenerSink` that records every decision.
    #[derive(Default)]
    pub struct VecGardenerSink {
        /// Captured decisions.
        pub decisions: Vec<GardenerDecision>,
    }

    impl GardenerSink for VecGardenerSink {
        fn put<'a>(
            &'a mut self,
            decision: &'a GardenerDecision,
        ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>> {
            self.decisions.push(decision.clone());
            Box::pin(async { Ok(()) })
        }
    }
}
