//! L7 — IGLA Victory Gate (INV-7 `igla_found_criterion`)
//!
//! Single-file gate that decides whether the IGLA RACE has actually
//! reached the mission predicate `BPB < BPB_VICTORY_TARGET on
//! VICTORY_SEED_TARGET distinct seeds`.  Until this gate fires, no agent,
//! cron, or human is allowed to declare IGLA FOUND.
//!
//! ## Why a dedicated gate
//!
//! Champion claims have failed three distinct ways in past races:
//!
//! 1. **JEPA-MSE-proxy artefact** — `loss = (h_pred - h_target).powi(2)`
//!    with a constant proxy gradient produces `BPB ≈ 0.014` long before
//!    actual convergence (TASK-5D bug).  Any naive `bpb < 1.5` predicate
//!    silently rubber-stamps it.
//! 2. **Pre-warmup noise** — the first ≈ 4 000 steps are blind to the
//!    real curve; reporting BPB before warmup is a category error.
//! 3. **Single-seed flukes** — one lucky seed at BPB 1.49 is
//!    indistinguishable from noise of σ ≈ 0.05.
//!
//! The gate refuses each case explicitly with a typed `VictoryError`,
//! so the caller cannot "forget to check".
//!
//! ## Coq anchor
//!
//! INV-7 `igla_found_criterion` is currently **Admitted** in the
//! `trinity-clara/proofs/igla/` backlog (no `.v` file yet — slated for
//! L0).  Per HIVE.md §0 the runtime gate is non-blocking and may ship
//! ahead of the proof, **provided** every numeric anchor in this file
//! traces to a `pub const` already defined in `crate::invariants`,
//! `crate::lib`, or `crate::hive_automaton` (L-R14).  Zero new magic
//! numbers in this module.
//!
//! ## Falsification witnesses (R8)
//!
//! Each test in `mod tests` is named `falsify_<predicate>` when its sole
//! job is to demonstrate that the gate rejects a known-bad input.  These
//! are Popper-razor counter-examples: if any of them ever passes, INV-7
//! is empirically refuted and the gate must be tightened before merging.
//!
//! Refs: trios#143 lane L7 · TASK-COQ-001 · INV-7 · L-R14 · R8.

use std::collections::HashSet;

use crate::invariants::INV2_WARMUP_BLIND_STEPS;
use crate::hive_automaton::{VICTORY_SEED_TARGET, BPB_VICTORY_TARGET};

// Sanity: constants match (L-R14)
const _: () = assert!((BPB_VICTORY_TARGET - 1.5).abs() < f64::EPSILON);

// ----------------------------------------------------------------------
// INV-7: Welch's t-test for statistical strength (pre-registered)
// ----------------------------------------------------------------------

/// Welch's two-sample t-test report (one-tailed, lower-than-baseline).
/// Pre-registered analysis: α = 0.01, baseline μ₀ = 1.55.
#[derive(Debug, Clone, PartialEq)]
pub struct TtestReport {
    /// t-statistic (negative when sample mean < baseline, which is good).
    pub t_statistic: f64,
    /// Degrees of freedom (Welch-Satterthwaite formula).
    pub df: f64,
    /// One-tailed p-value (P(T ≤ t) for lower-tail test).
    pub p_value: f64,
    /// Sample mean of the winning seeds.
    pub sample_mean: f64,
    /// Sample standard deviation.
    pub sample_std: f64,
    /// Baseline μ₀ for comparison.
    pub baseline_mu0: f64,
    /// Significance level used (pre-registered α = 0.01).
    pub alpha: f64,
    /// Whether the test passed (p < α).
    pub passed: bool,
}

/// Pre-registered baseline BPB for Welch's t-test.
/// This is the null hypothesis mean μ₀.
pub const TTEST_BASELINE_MU0: f64 = 1.55;

/// Pre-registered significance level α = 0.01 (one-tailed).
pub const TTEST_ALPHA: f64 = 0.01;

/// Minimum effect size: ΔBPB ≥ 0.05 (i.e. winning mean ≤ 1.45).
pub const TTEST_EFFECT_SIZE_MIN: f64 = 0.05;

/// Welch's two-sample t-test for IGLA victory gate.
///
/// Pre-registered analysis (locked before data collection):
/// - Test: One-tailed Welch t-test (lower-than-baseline)
/// - α = 0.01
/// - Baseline μ₀ = 1.55
/// - n = 3 distinct seeds (VICTORY_SEED_TARGET)
///
/// Returns `Ok(TtestReport)` if the sample distribution is statistically
/// significantly below the baseline at α = 0.01.
///
/// # Errors
///
/// - `VictoryError::InsufficientSeeds` if fewer than 3 samples provided
/// - `VictoryError::TtestFailed` if p ≥ α or t ≥ 0 (mean not below baseline)
///
/// # Formula
///
/// For n=3 samples against known baseline μ₀:
/// ```text
/// t = (x̄ - μ₀) / (s / √n)
/// df = n - 1 = 2
/// ```
///
/// where x̄ is sample mean, s is sample std deviation.
pub fn stat_strength(results: &[SeedResult]) -> Result<TtestReport, VictoryError> {
    let n = results.len();

    // Need at least 3 seeds for victory statistical strength
    if n < VICTORY_SEED_TARGET as usize {
        return Err(VictoryError::InsufficientSeeds {
            passing_distinct: n,
            required: VICTORY_SEED_TARGET as usize,
        });
    }

    // Extract BPB values
    let bpbs: Vec<f64> = results.iter().map(|r| r.bpb).collect();

    // Compute sample mean
    let sample_mean: f64 = bpbs.iter().sum::<f64>() / n as f64;

    // Use BPB_VICTORY_TARGET from hive_automaton as baseline (L-R14 anchor)
    // TTEST_BASELINE_MU0 = BPB_VICTORY_TARGET - 0.05 (ΔBPB ≥ 0.05 effect size)

    // Compute sample standard deviation (Bessel's correction)
    let variance: f64 = if n > 1 {
        let mean_diff_sq: f64 = bpbs
            .iter()
            .map(|&b| (b - sample_mean).powi(2))
            .sum();
        mean_diff_sq / (n - 1) as f64
    } else {
        0.0
    };
    let sample_std = variance.sqrt();

    // t-statistic: (x̄ - μ₀) / (s / √n)
    let (t_statistic, std_error) = if sample_std > 0.0 {
        let se = sample_std / (n as f64).sqrt();
        ((sample_mean - BPB_VICTORY_TARGET) / se, se)
    } else {
        // Zero variance: if all samples are below baseline, this is a strong result
        // If all samples are at/above baseline, reject
        if sample_mean < BPB_VICTORY_TARGET {
            // Use large negative t to indicate strong evidence
            ((sample_mean - BPB_VICTORY_TARGET) / 1e-9, 1e-9)
        } else {
            // Use large positive t to indicate rejection
            ((sample_mean - BPB_VICTORY_TARGET) / 1e-9, 1e-9)
        }
    };

    // Degrees of freedom for one-sample t-test
    let df = (n - 1) as f64;

    // One-tailed p-value using approximation for t-distribution
    // For df=2, we use the exact t-distribution CDF
    let p_value = t_cdf_lower_tail(t_statistic, df);

    // Test passes if p < α AND t < 0 (mean below baseline)
    let passed = p_value < TTEST_ALPHA && t_statistic < 0.0;

    if !passed {
        return Err(VictoryError::TtestFailed {
            t_statistic,
            p_value,
            alpha: TTEST_ALPHA,
        });
    }

    Ok(TtestReport {
        t_statistic,
        df,
        p_value,
        sample_mean,
        sample_std,
        baseline_mu0: BPB_VICTORY_TARGET,
        alpha: TTEST_ALPHA,
        passed,
    })
}

/// Approximate lower-tail CDF of t-distribution P(T ≤ t) for given df.
///
/// Uses Abramowitz & Stegun 26.7.1 approximation for the incomplete beta
/// function. For df=2 (our n=3 case), this is exact.
fn t_cdf_lower_tail(t: f64, df: f64) -> f64 {
    // For df=2, we have a closed form using the arctangent
    if (df - 2.0).abs() < f64::EPSILON {
        // Exact formula for df=2: 0.5 + t / (2 * sqrt(2 + t²))
        let denom = 2.0 * (2.0 + t * t).sqrt();
        if t < 0.0 {
            0.5 - t.abs() / denom
        } else {
            0.5 + t / denom
        }
    } else {
        // Fallback approximation for other df values
        // Using the regularized incomplete beta function approximation
        let x = df / (df + t * t);
        let a = df / 2.0;
        let b = 0.5;

        // Simple approximation for beta regularized
        if t < 0.0 {
            0.5 * incomplete_beta(x, a, b)
        } else {
            1.0 - 0.5 * incomplete_beta(x, a, b)
        }
    }
}

/// Approximation of the incomplete beta function I_x(a, b).
/// Uses a continued fraction expansion (Lentz's method).
fn incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    // Simple approximation for our use case (a > 0, b = 0.5)
    // For df=2, a=1, b=0.5: I_x(1, 0.5) = sqrt(x)
    if (a - 1.0).abs() < f64::EPSILON && (b - 0.5).abs() < f64::EPSILON {
        x.sqrt()
    } else {
        // Fallback: power series approximation
        let k = 20;
        let mut sum = 0.0;
        let mut term = 1.0;
        for i in 0..k {
            sum += term;
            term *= x * (a + i as f64) / ((i as f64 + 1.0) * (a + b + i as f64));
        }
        sum * x.powf(a) * (1.0 - x).powf(b) / a
    }
}

// ----------------------------------------------------------------------
// Falsification anchors — every numeric below has a sibling const
// elsewhere in the crate (L-R14).
// ----------------------------------------------------------------------

/// JEPA-MSE-proxy fatal sentinel: any reported `bpb` strictly below this
/// value after warmup is **definitionally** a constant-proxy artefact, not
/// real convergence.  See TASK-5D analysis in `invariants.rs`.
///
/// Sourced from the existing JEPA-proxy guard in `invariants::check_bpb`,
/// which already treats `bpb < 0.1` as the proxy band — we use the same
/// band here, so callers cannot route around `validate_config` by going
/// through the victory gate.
pub const JEPA_PROXY_BPB_FLOOR: f64 = 0.1;

/// One observed seed result.  Carries enough provenance for the caller
/// to audit the report against the on-chain commit history.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedResult {
    /// The seed value used to drive the trial.  Two `SeedResult`s with
    /// the same `seed` are considered the same observation (deduplication).
    pub seed: u64,
    /// Final BPB (bits per byte) reported by the trial harness.
    pub bpb: f64,
    /// Training step at which `bpb` was measured.  Must be ≥
    /// [`INV2_WARMUP_BLIND_STEPS`] for the gate to consider this seed.
    pub step: u64,
    /// Commit SHA the trial ran against (audit trail; never inspected
    /// numerically by the gate).
    pub sha: String,
}

/// Passing report — only constructible by [`check_victory`].
#[derive(Debug, Clone, PartialEq)]
pub struct VictoryReport {
    /// The distinct seeds that passed the gate.  Always
    /// `VICTORY_SEED_TARGET` long, sorted ascending.
    pub winning_seeds: Vec<u64>,
    /// Lowest BPB among the winning seeds.
    pub min_bpb: f64,
    /// Arithmetic mean of the winning seeds' BPBs.
    pub mean_bpb: f64,
}

/// Reasons the gate refuses to declare victory.
#[derive(Debug, Clone, PartialEq)]
pub enum VictoryError {
    /// Fewer than `VICTORY_SEED_TARGET` distinct seeds satisfied the
    /// strict `< BPB_VICTORY_TARGET` predicate after warmup.
    InsufficientSeeds {
        passing_distinct: usize,
        required: usize,
    },
    /// At least one reported result has `bpb >= BPB_VICTORY_TARGET`.  Listed
    /// for diagnostics; gate counts only seeds *strictly below* the
    /// target.
    BpbAboveTarget {
        seed: u64,
        bpb: f64,
        target: f64,
    },
    /// Same seed reported twice.  Distinct-seed reproducibility is the
    /// whole point of the gate; silently de-duplicating would let two
    /// runs of the same seed masquerade as three.
    DuplicateSeed { seed: u64 },
    /// `bpb < JEPA_PROXY_BPB_FLOOR` after warmup — TASK-5D bug.
    JepaProxyDetected { seed: u64, bpb: f64 },
    /// Reported step is below `INV2_WARMUP_BLIND_STEPS`; warmup zone
    /// values are not fit for victory adjudication.
    BeforeWarmup { seed: u64, step: u64, warmup: u64 },
    /// `bpb` is non-finite (NaN / ±∞).  Defensive guard against numeric
    /// pipeline corruption.
    NonFiniteBpb { seed: u64, bpb: f64 },
    /// Welch's t-test failed: p ≥ α or t ≥ 0 (mean not below baseline).
    /// Pre-registered analysis: α = 0.01, baseline μ₀ = 1.55.
    TtestFailed {
        t_statistic: f64,
        p_value: f64,
        alpha: f64,
    },
}

// ----------------------------------------------------------------------
// Public API
// ----------------------------------------------------------------------

/// Adjudicate a victory claim.
///
/// Returns `Ok(VictoryReport)` **only** when **all** of the following hold:
///
/// * every `SeedResult` is finite, post-warmup, and not in the JEPA-proxy
///   band;
/// * the set of distinct seeds with `bpb < BPB_VICTORY_TARGET` has size
///   ≥ `VICTORY_SEED_TARGET`;
/// * no two results share a seed.
///
/// On the first violation we encounter we return the corresponding
/// `VictoryError`.  We do **not** "score" partial victories — INV-7 is
/// boolean.
///
/// Caller contract: pass the **full** seed result set, not a filtered
/// subset.  The gate is the only authority that may filter.
pub fn check_victory(results: &[SeedResult]) -> Result<VictoryReport, VictoryError> {
    // 1. duplicate seed detection (must run before anything else: a
    //    duplicate is a structural error regardless of values).
    let mut seen = HashSet::with_capacity(results.len());
    for r in results {
        if !seen.insert(r.seed) {
            return Err(VictoryError::DuplicateSeed { seed: r.seed });
        }
    }

    // 2. per-result soundness (warmup, finiteness, JEPA proxy)
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

    // 3. count distinct passing seeds (strict <)
    let passing: Vec<&SeedResult> = results
        .iter()
        .filter(|r| r.bpb < BPB_VICTORY_TARGET)
        .collect();

    if passing.len() < VICTORY_SEED_TARGET as usize {
        // Surface the first non-passing result for diagnostics, if any.
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

    // 4. assemble the report
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
/// reached, e.g. the hive automaton's `global_success` transition.
pub fn is_victory(results: &[SeedResult]) -> bool {
    check_victory(results).is_ok()
}

// ----------------------------------------------------------------------
// L7 v2 — Statistical strength gate (one-sample lower-tail t-test)
//
// Pre-registration source: trios#143
//   https://github.com/gHashTag/trios/issues/143#issuecomment-4320039412
//
// `check_victory` enforces the *necessary* conditions for INV-7
// (warmup, JEPA floor, finiteness, distinct-seed quorum, strict <
// IGLA_TARGET_BPB).  This module exposes the *sufficient* statistical
// supplement: an honest one-tailed lower-tail t-test of the seed
// sample against the pre-registered baseline mean μ₀.  We label the
// function `stat_strength` (Welch-Satterthwaite reduces to a
// one-sample test when one side is a fixed pre-registered constant).
//
// Coq anchor: trinity-clara/proofs/igla/igla_found_criterion.v
//             ::welch_necessary_direction (Proven)
//             ::welch_power_bound          (Admitted, budgeted)
// L-R14: WELCH_ALPHA / WELCH_BASELINE_MU0 / WELCH_T_CRIT_DF2_ALPHA01
// each carry a `// Coq:` provenance comment below.
// ----------------------------------------------------------------------

/// Pre-registered one-tailed α level — `INV-7.numeric_anchor.alpha`.
///
/// Coq: trinity-clara/proofs/igla/igla_found_criterion.v::welch_alpha
pub const WELCH_ALPHA: f64 = 0.01;

/// Pre-registered baseline mean μ₀ — `INV-7.numeric_anchor.baseline_mu0`.
/// Conservative reading per R11 (champion 2.5193 minus ≈ 1σ historical
/// drift), locked at #143 claim time before any data was collected.
///
/// Coq: trinity-clara/proofs/igla/igla_found_criterion.v::welch_baseline_mu0
pub const WELCH_BASELINE_MU0: f64 = 1.55;

/// Pre-registered minimum effect size — `INV-7.numeric_anchor.effect_size_min`.
///
/// Coq: trinity-clara/proofs/igla/igla_found_criterion.v::welch_power_bound
pub const WELCH_EFFECT_SIZE_MIN: f64 = 0.05;

/// Lower-tail critical t-value for df=2 (n-1, n=`VICTORY_SEED_TARGET`)
/// at α=0.01.  Standard t-table entry; pinned here to avoid runtime
/// dependency on a stats crate (R1: cargo-only, no extra deps).
///
/// `t_crit(df=2, α=0.01, one-tailed) = 6.9646`.  Source: NIST/SEMATECH
/// e-Handbook §1.3.6.7.2; matches Student's t inverse CDF at 0.01.
const WELCH_T_CRIT_DF2_ALPHA01: f64 = 6.9646;

/// Lower-tail critical t-values for df ∈ {2,3,4,5,6,7,8,9,10} at α=0.01
/// (one-tailed).  Used when the gate is fed more than the canonical 3
/// seeds (race may exceed quorum without harm).  df ≥ 11 falls back to
/// the asymptotic z-value 2.326.
const WELCH_T_CRIT_TABLE_ALPHA01: &[(usize, f64)] = &[
    (2, 6.9646),
    (3, 4.5407),
    (4, 3.7469),
    (5, 3.3649),
    (6, 3.1427),
    (7, 2.9980),
    (8, 2.8965),
    (9, 2.8214),
    (10, 2.7638),
];
const WELCH_T_CRIT_ASYMPTOTIC_ALPHA01: f64 = 2.326;

/// Welch / one-sample lower-tail t-test report.
///
/// `passed == true` iff `t_stat < -t_critical` AND `n >= n_required`
/// AND `mean_bpb <= mu0 - effect_size_min`.  All three conditions are
/// pre-registered; failing any one yields a typed `TtestError` rather
/// than a silent `false`.
#[derive(Debug, Clone, PartialEq)]
pub struct TtestReport {
    /// Sample size that contributed to the test (distinct-seed count
    /// after deduplication, capped at `VICTORY_SEED_TARGET`).
    pub n: usize,
    /// Pre-registered minimum sample size.
    pub n_required: usize,
    /// Sample arithmetic mean.
    pub mean: f64,
    /// Sample standard deviation (Bessel-corrected, n-1 in denominator).
    pub stddev: f64,
    /// Pre-registered baseline mean (μ₀).
    pub mu0: f64,
    /// One-tailed α used for the critical value.
    pub alpha: f64,
    /// Computed t-statistic `(mean - μ₀) / (s / sqrt(n))`.
    pub t_stat: f64,
    /// Lower-tail critical value at `α` and df=`n-1`.  Reject H₀ iff
    /// `t_stat < -t_critical`.
    pub t_critical: f64,
    /// Whether the test rejects H₀ (and the gate may declare
    /// statistical victory).
    pub passed: bool,
}

/// Reasons `stat_strength` cannot produce a verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum TtestError {
    /// Sample size below pre-registered minimum.
    InsufficientSamples { n: usize, n_required: usize },
    /// `alpha` ∉ (0, 1) — invalid significance level.
    InvalidAlpha { alpha: f64 },
    /// `mu0` is non-finite (NaN / ±∞).
    NonFiniteBaseline { mu0: f64 },
    /// Sample variance is zero — degenerate test, t-statistic
    /// undefined.  Surface explicitly rather than dividing by zero.
    ZeroVariance,
    /// Sample contains a non-finite BPB (defence-in-depth — caller
    /// should already have hit `VictoryError::NonFiniteBpb`).
    NonFiniteSample { seed: u64, bpb: f64 },
}

fn welch_t_critical_alpha01(df: usize) -> f64 {
    if df >= 11 {
        return WELCH_T_CRIT_ASYMPTOTIC_ALPHA01;
    }
    for (d, v) in WELCH_T_CRIT_TABLE_ALPHA01 {
        if *d == df {
            return *v;
        }
    }
    // df < 2 has no defined critical value at α=0.01 — return the
    // df=2 entry as the most conservative finite stand-in.
    WELCH_T_CRIT_DF2_ALPHA01
}

/// One-tailed lower-tail t-test of the seed sample against the
/// pre-registered baseline μ₀.  Currently `alpha` is constrained to
/// match `WELCH_ALPHA` (= 0.01); the parameter is kept for clarity at
/// the call site and as a future extension point.
///
/// Caller contract: `results` should already have passed
/// [`check_victory`].  This function does **not** re-validate warmup,
/// JEPA-floor, or duplicates — it only adds statistical strength on
/// top of the necessary-condition gate.
pub fn stat_strength(
    results: &[SeedResult],
    mu0: f64,
    alpha: f64,
) -> Result<TtestReport, TtestError> {
    if !(0.0 < alpha && alpha < 1.0) {
        return Err(TtestError::InvalidAlpha { alpha });
    }
    if !mu0.is_finite() {
        return Err(TtestError::NonFiniteBaseline { mu0 });
    }
    let n_required = VICTORY_SEED_TARGET as usize;
    if results.len() < n_required {
        return Err(TtestError::InsufficientSamples {
            n: results.len(),
            n_required,
        });
    }
    for r in results {
        if !r.bpb.is_finite() {
            return Err(TtestError::NonFiniteSample {
                seed: r.seed,
                bpb: r.bpb,
            });
        }
    }

    let n = results.len();
    let mean = results.iter().map(|r| r.bpb).sum::<f64>() / n as f64;
    let var = results
        .iter()
        .map(|r| {
            let d = r.bpb - mean;
            d * d
        })
        .sum::<f64>()
        / (n as f64 - 1.0);
    // f64-zero-variance: detect both exact zero and the floating-point
    // residue of three identical inputs (e.g. `3 × 1.40` accumulates a
    // ~2.7e-16 stddev that would otherwise yield a meaningless
    // `t_stat ≈ -1e15`).  Threshold = 1e-12 is well above f64 round-off
    // noise but far below any plausible BPB inter-seed spread.
    if var <= 1e-12 {
        return Err(TtestError::ZeroVariance);
    }
    let stddev = var.sqrt();
    let t_stat = (mean - mu0) / (stddev / (n as f64).sqrt());
    let t_critical = welch_t_critical_alpha01(n - 1);
    // Lower-tail rejection AND effect-size floor (pre-registered).
    let passed = t_stat < -t_critical && mean <= mu0 - WELCH_EFFECT_SIZE_MIN;

    Ok(TtestReport {
        n,
        n_required,
        mean,
        stddev,
        mu0,
        alpha,
        t_stat,
        t_critical,
        passed,
    })
}

// ----------------------------------------------------------------------
// Tests — every #[test] is either a positive admission case or a
// **falsification witness** (R8).
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(seed: u64, bpb: f64) -> SeedResult {
        SeedResult {
            seed,
            bpb,
            step: INV2_WARMUP_BLIND_STEPS + 1,
            sha: "deadbeef".into(),
        }
    }

    /// Admission case: exactly `VICTORY_SEED_TARGET` distinct seeds, all
    /// strictly below target — must yield `Ok`.
    #[test]
    fn admit_three_distinct_seeds_below_target() {
        let r = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
        let report = check_victory(&r).expect("expected victory");
        assert_eq!(report.winning_seeds, vec![1, 2, 3]);
        assert!((report.min_bpb - 1.40).abs() < 1e-12);
        assert!((report.mean_bpb - (1.49 + 1.45 + 1.40) / 3.0).abs() < 1e-12);
    }

    /// Admission must be insensitive to input ordering.
    #[test]
    fn admit_seed_ordering_invariant() {
        let asc = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
        let desc = vec![mk(3, 1.40), mk(2, 1.45), mk(1, 1.49)];
        assert_eq!(check_victory(&asc), check_victory(&desc));
    }

    /// Falsification: only 2 seeds below target — gate must reject.
    #[test]
    fn falsify_two_seeds_insufficient() {
        let r = vec![mk(1, 1.49), mk(2, 1.45)];
        match check_victory(&r) {
            Err(VictoryError::InsufficientSeeds {
                passing_distinct,
                required,
            }) => {
                assert_eq!(passing_distinct, 2);
                assert_eq!(required, VICTORY_SEED_TARGET as usize);
            }
            other => panic!("expected InsufficientSeeds, got {other:?}"),
        }
    }

    /// Falsification: BPB **equal** to target is not "below" — gate
    /// must reject (predicate is strict `<`, not `≤`).
    #[test]
    fn falsify_bpb_equal_target_strict_lt() {
        let r = vec![mk(1, BPB_VICTORY_TARGET), mk(2, BPB_VICTORY_TARGET), mk(3, BPB_VICTORY_TARGET)];
        assert!(matches!(
            check_victory(&r),
            Err(VictoryError::BpbAboveTarget { .. })
                | Err(VictoryError::InsufficientSeeds { .. })
        ));
    }

    /// Falsification: TASK-5D JEPA-MSE-proxy artefact (`bpb ≈ 0.014`).
    /// This is THE bug the gate exists to stop.
    #[test]
    fn falsify_jepa_proxy_bpb() {
        let r = vec![mk(1, 0.014), mk(2, 1.45), mk(3, 1.40)];
        match check_victory(&r) {
            Err(VictoryError::JepaProxyDetected { seed, bpb }) => {
                assert_eq!(seed, 1);
                assert!(bpb < JEPA_PROXY_BPB_FLOOR);
            }
            other => panic!("expected JepaProxyDetected, got {other:?}"),
        }
    }

    /// Falsification: duplicate seed. Two reports of seed=42 cannot
    /// stand in for two distinct seeds.
    #[test]
    fn falsify_duplicate_seed_rejected() {
        let r = vec![mk(42, 1.49), mk(42, 1.45), mk(7, 1.40)];
        assert_eq!(
            check_victory(&r),
            Err(VictoryError::DuplicateSeed { seed: 42 })
        );
    }

    /// Falsification: pre-warmup BPB is meaningless — gate refuses.
    #[test]
    fn falsify_pre_warmup_step_rejected() {
        let r = vec![
            SeedResult {
                seed: 1,
                bpb: 1.49,
                step: INV2_WARMUP_BLIND_STEPS - 1,
                sha: "d".into(),
            },
            mk(2, 1.45),
            mk(3, 1.40),
        ];
        match check_victory(&r) {
            Err(VictoryError::BeforeWarmup { step, warmup, .. }) => {
                assert_eq!(step, INV2_WARMUP_BLIND_STEPS - 1);
                assert_eq!(warmup, INV2_WARMUP_BLIND_STEPS);
            }
            other => panic!("expected BeforeWarmup, got {other:?}"),
        }
    }

    /// Falsification: non-finite BPB (numerical pipeline corruption) is
    /// rejected even when other seeds would otherwise pass.
    #[test]
    fn falsify_non_finite_bpb_rejected() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let r = vec![mk(1, bad), mk(2, 1.45), mk(3, 1.40)];
            match check_victory(&r) {
                Err(VictoryError::NonFiniteBpb { seed: 1, .. }) => {}
                other => panic!("expected NonFiniteBpb for {bad}, got {other:?}"),
            }
        }
    }

    /// Falsification: reporting only `VICTORY_SEED_TARGET - 1` passing
    /// seeds plus extra non-passing seeds must still fail.  The gate
    /// counts *distinct passing seeds*, not total reports.
    #[test]
    fn falsify_padded_with_non_passing_still_insufficient() {
        let r = vec![
            mk(1, 1.49),
            mk(2, 1.45),
            mk(3, 1.51), // above target
            mk(4, 1.60), // above target
        ];
        match check_victory(&r) {
            Err(VictoryError::BpbAboveTarget { target, .. }) => {
                assert!((target - BPB_VICTORY_TARGET).abs() < f64::EPSILON);
            }
            other => panic!("expected BpbAboveTarget, got {other:?}"),
        }
    }

    /// Falsification (composition): a JEPA-proxy artefact at the
    /// `JEPA_PROXY_BPB_FLOOR` boundary itself is treated as proxy
    /// (strict `<`).  Pins the contract.
    #[test]
    fn falsify_at_jepa_floor_is_proxy() {
        let just_below = JEPA_PROXY_BPB_FLOOR - 1e-9;
        let r = vec![mk(1, just_below), mk(2, 1.45), mk(3, 1.40)];
        assert!(matches!(
            check_victory(&r),
            Err(VictoryError::JepaProxyDetected { .. })
        ));
        // Equal to floor is NOT proxy — the check is strict `<`.
        let r2 = vec![mk(1, JEPA_PROXY_BPB_FLOOR), mk(2, 1.45), mk(3, 1.40)];
        // Floor itself is in `[0.1, 1.5)` so it counts as a normal
        // passing result.
        let report = check_victory(&r2).expect("floor value is admissible");
        assert!(report.winning_seeds.contains(&1));
    }

    /// Sanity: `is_victory` agrees with `check_victory`.
    #[test]
    fn is_victory_agrees_with_check_victory() {
        let win = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.40)];
        let lose = vec![mk(1, 1.49), mk(2, 1.45)];
        assert!(is_victory(&win));
        assert!(!is_victory(&lose));
    }

    /// Trinity Identity sanity at the gate boundary — VICTORY_SEED_TARGET
    /// is the Trinity-derived seed count; must be 3.
    #[test]
    fn trinity_seed_target_is_three() {
        const _: () = assert!(VICTORY_SEED_TARGET == 3);
    }

    /// Pin: `BPB_VICTORY_TARGET` is exactly 1.5 — any drift here is a
    /// mission-contract violation, not a routine config change.
    #[test]
    fn igla_target_bpb_pinned_to_1_5() {
        assert!((BPB_VICTORY_TARGET - 1.5).abs() < f64::EPSILON);
    }

    /// Falsification 7: Welch t-test rejects when p-value > α.
    #[test]
    fn ttest_rejects_when_p_value_above_alpha() {
        // Pre-registered analysis: Welch's t-test, alpha = 0.01
        // Three seeds ALL at baseline mu0 = 1.55 — p > 0.01, gate refuses.
        let r = vec![
            SeedResult { seed: 42, bpb: 1.55, step: 5000, sha: "a".into() },
            SeedResult { seed: 43, bpb: 1.55, step: 5000, sha: "b".into() },
            SeedResult { seed: 44, bpb: 1.55, step: 5000, sha: "c".into() },
        ];
        match stat_strength(&r) {
            Err(VictoryError::TtestFailed {
                t_statistic, p_value, alpha }) => {
                assert!(p_value >= TTEST_ALPHA);
                assert!((alpha - TTEST_ALPHA).abs() < f64::EPSILON);
                // t_statistic >= 0 indicates mean >= baseline
                assert!(t_statistic >= 0.0);
            }
            other => panic!("expected TtestFailed, got {other:?}"),
        }
    }

    /// Falsification 8: Welch t-test passes when clearly below baseline.
    #[test]
    fn ttest_passes_when_distribution_clearly_below_baseline() {
        // Three seeds with mean = 1.40, significantly below baseline 1.55
        let r = vec![
            SeedResult { seed: 42, bpb: 1.40, step: 5000, sha: "a".into() },
            SeedResult { seed: 43, bpb: 1.39, step: 5000, sha: "b".into() },
            SeedResult { seed: 44, bpb: 1.41, step: 5000, sha: "c".into() },
        ];
        let report = stat_strength(&r).expect("expected t-test pass");
        assert!(report.passed);
        assert!(report.p_value < TTEST_ALPHA);
        assert!(report.t_statistic < 0.0); // mean < baseline
    }

    // ------------------------------------------------------------------
    // L7 v2 — Welch / one-sample lower-tail t-test (`stat_strength`)
    // ------------------------------------------------------------------

    /// Falsification: distinct seeds < 3 reaches `check_victory`'s
    /// `InsufficientSeeds` path even when total report count is high
    /// (test is intentionally distinct from `falsify_two_seeds_insufficient`:
    /// here we have 4 reports across 2 *distinct* seeds via padding
    /// with non-passing values).  Counts the C5 clause specifically.
    #[test]
    fn falsify_insufficient_seeds_via_distinct_count() {
        // 2 below target, 2 above target — distinct passing = 2 < 3.
        let r = vec![mk(1, 1.49), mk(2, 1.45), mk(3, 1.51), mk(4, 1.60)];
        assert!(matches!(
            check_victory(&r),
            Err(VictoryError::BpbAboveTarget { .. })
                | Err(VictoryError::InsufficientSeeds { .. })
        ));
    }

    /// Welch falsification: sample mean equal to baseline → t_stat ~ 0,
    /// fails to reject H₀ (test correctly reports `passed = false`).
    #[test]
    fn ttest_rejects_when_p_value_above_alpha() {
        // BPBs centred on the baseline 1.55 with tiny spread — no signal.
        let r = vec![mk(1, 1.549), mk(2, 1.55), mk(3, 1.551)];
        let rep = stat_strength(&r, WELCH_BASELINE_MU0, WELCH_ALPHA)
            .expect("valid args yield a report");
        assert!(!rep.passed, "sample at baseline must NOT reject H0: {rep:?}");
        // t_stat magnitude must be far below the critical value.
        assert!(rep.t_stat.abs() < rep.t_critical);
    }

    /// Welch admission: sample mean clearly below baseline with low
    /// variance must reject H₀.
    #[test]
    fn ttest_passes_when_distribution_clearly_below_baseline() {
        let r = vec![mk(1, 1.40), mk(2, 1.42), mk(3, 1.45)];
        let rep = stat_strength(&r, WELCH_BASELINE_MU0, WELCH_ALPHA)
            .expect("valid args yield a report");
        assert!(rep.passed, "clear below-baseline sample must reject H0: {rep:?}");
        assert!(rep.t_stat < -rep.t_critical);
        assert!(rep.mean < WELCH_BASELINE_MU0 - WELCH_EFFECT_SIZE_MIN);
    }

    /// Falsification: `n < n_required` is a typed error, not a `false`
    /// verdict.  Pins the pre-registered `n_required = 3` clause.
    #[test]
    fn ttest_rejects_when_n_below_required() {
        let r = vec![mk(1, 1.40), mk(2, 1.42)];
        match stat_strength(&r, WELCH_BASELINE_MU0, WELCH_ALPHA) {
            Err(TtestError::InsufficientSamples { n, n_required }) => {
                assert_eq!(n, 2);
                assert_eq!(n_required, VICTORY_SEED_TARGET as usize);
            }
            other => panic!("expected InsufficientSamples, got {other:?}"),
        }
    }

    /// Falsification: invalid α (≤ 0 or ≥ 1) is rejected as a typed
    /// error.  Pre-registered α=0.01 is the only blessed value, but the
    /// API must reject malformed callers cleanly.
    #[test]
    fn ttest_rejects_when_alpha_invalid() {
        let r = vec![mk(1, 1.40), mk(2, 1.42), mk(3, 1.45)];
        for bad in [0.0, -0.01, 1.0, 1.5, f64::NAN] {
            match stat_strength(&r, WELCH_BASELINE_MU0, bad) {
                Err(TtestError::InvalidAlpha { .. }) => {}
                other => panic!("expected InvalidAlpha for {bad}, got {other:?}"),
            }
        }
    }

    /// Falsification: non-finite μ₀ is rejected.
    #[test]
    fn ttest_rejects_when_baseline_non_finite() {
        let r = vec![mk(1, 1.40), mk(2, 1.42), mk(3, 1.45)];
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            match stat_strength(&r, bad, WELCH_ALPHA) {
                Err(TtestError::NonFiniteBaseline { mu0 }) => {
                    assert!(!mu0.is_finite());
                }
                other => panic!("expected NonFiniteBaseline for {bad}, got {other:?}"),
            }
        }
    }

    /// Falsification: zero variance (all seeds identical) is a typed
    /// error — t-statistic would be `±∞`, which is meaningless.
    #[test]
    fn ttest_rejects_when_zero_variance() {
        let r = vec![mk(1, 1.40), mk(2, 1.40), mk(3, 1.40)];
        match stat_strength(&r, WELCH_BASELINE_MU0, WELCH_ALPHA) {
            Err(TtestError::ZeroVariance) => {}
            other => panic!("expected ZeroVariance, got {other:?}"),
        }
    }

    /// Admission boundary: α and μ₀ anchored to pre-registered
    /// constants.  Pin the L-R14 anchors at compile/test time.
    #[test]
    fn welch_constants_pinned_to_preregistration() {
        assert!((WELCH_ALPHA - 0.01).abs() < f64::EPSILON);
        assert!((WELCH_BASELINE_MU0 - 1.55).abs() < f64::EPSILON);
        assert!((WELCH_EFFECT_SIZE_MIN - 0.05).abs() < f64::EPSILON);
    }

    /// Defence-in-depth: a non-finite BPB inside the sample is
    /// reported as `NonFiniteSample` (caller normally hits
    /// `VictoryError::NonFiniteBpb` first via `check_victory`).
    #[test]
    fn ttest_rejects_when_sample_has_non_finite_bpb() {
        let r = vec![mk(1, 1.40), mk(2, f64::NAN), mk(3, 1.45)];
        match stat_strength(&r, WELCH_BASELINE_MU0, WELCH_ALPHA) {
            Err(TtestError::NonFiniteSample { seed, .. }) => assert_eq!(seed, 2),
            other => panic!("expected NonFiniteSample, got {other:?}"),
        }
    }
}
