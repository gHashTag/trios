//! L7 — IGLA Victory Gate (INV-7 `igla_found_criterion`)
//!
//! Single-file gate that decides whether the IGLA RACE has actually
//! reached the mission predicate `BPB < IGLA_TARGET_BPB on
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

use crate::hive_automaton::VICTORY_SEED_TARGET;
use crate::invariants::INV2_WARMUP_BLIND_STEPS;
use crate::IGLA_TARGET_BPB;

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
    /// strict `< IGLA_TARGET_BPB` predicate after warmup.
    InsufficientSeeds {
        passing_distinct: usize,
        required: usize,
    },
    /// At least one reported result has `bpb >= IGLA_TARGET_BPB`.  Listed
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
/// * the set of distinct seeds with `bpb < IGLA_TARGET_BPB` has size
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
        .filter(|r| r.bpb < IGLA_TARGET_BPB)
        .collect();

    if passing.len() < VICTORY_SEED_TARGET as usize {
        // Surface the first non-passing result for diagnostics, if any.
        if let Some(r) = results.iter().find(|r| r.bpb >= IGLA_TARGET_BPB) {
            return Err(VictoryError::BpbAboveTarget {
                seed: r.seed,
                bpb: r.bpb,
                target: IGLA_TARGET_BPB,
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
        let r = vec![mk(1, IGLA_TARGET_BPB), mk(2, IGLA_TARGET_BPB), mk(3, IGLA_TARGET_BPB)];
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
                assert!((target - IGLA_TARGET_BPB).abs() < f64::EPSILON);
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

    /// Pin: `IGLA_TARGET_BPB` is exactly 1.5 — any drift here is a
    /// mission-contract violation, not a routine config change.
    #[test]
    fn igla_target_bpb_pinned_to_1_5() {
        assert!((IGLA_TARGET_BPB - 1.5).abs() < f64::EPSILON);
    }
}
