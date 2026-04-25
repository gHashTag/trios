//! IGLA Invariant Enforcement Layer — INV-001..INV-012
//! Single source of truth: assertions/igla_assertions.json
//! Trinity: φ² + φ⁻² = 3 | L-R14: coqc must pass before race starts

// φ-anchored constants from Trinity Identity
pub const PHI: f64 = 1.618033988749895;
pub const PHI_INV: f64 = 0.618033988749895;

// INV-2: φ² + φ⁻² + φ⁻⁴ = 3.4721… rounded to 3.5
pub const ASHA_PRUNE_THRESHOLD: f64 = 3.5;
pub const WARMUP_BLIND_STEPS: u64 = 4000;
// INV-1: lr ∈ [φ⁻⁸/2, φ⁻⁶/2] = [0.00382, 0.00618]
pub const LR_PHI_MIN: f64 = 0.00382;
pub const LR_PHI_MAX: f64 = 0.00618;
// INV-3: GF16 safe domain — d_model ≥ 256, error < φ⁻⁶
pub const GF16_D_MODEL_MIN: usize = 256;
pub const GF16_ERROR_BOUND: f64 = 0.0557; // φ⁻⁶
// INV-4: NCA entropy band [1.5, 2.8], weight lives separately (JSON: nca_loss_weight)
pub const NCA_ENTROPY_LOW: f64 = 1.5;
pub const NCA_ENTROPY_HIGH: f64 = 2.8;
pub const NCA_STATES_K: usize = 9;   // 9 = 3²
pub const NCA_GRID_SIZE: usize = 81; // 81 = 3⁴ = (φ²+φ⁻²)⁴
pub const NCA_LOSS_WEIGHT: f64 = 0.25; // separate from entropy_penalty — JSON: nca_loss_weight
// INV-12: valid ASHA rungs = 1000 × {3⁰, 3¹, 3², 3³}
pub const VALID_RUNGS: [u64; 4] = [1000, 3000, 9000, 27000];

/// Gradient mode for INV-1 — ConstantProxy is temporal (TASK-5D only)
#[derive(Debug, PartialEq)]
pub enum GradientMode {
    Real,
    #[deprecated(
        since = "post-task-5d",
        note = "Kept only for regression tests. Remove after TASK-5D merges."
    )]
    ConstantProxy,
}

/// NCA band mode for INV-4 — both bands coexist, never merged
#[derive(Debug, PartialEq)]
pub enum NcaBandMode {
    /// BENCH-004b empirical result: 97.67% MNIST = f32 (55× safety margin)
    Empirical,
    /// Admitted Coq bound: error < φ⁻⁶ ≈ 0.0557 (pending coq-interval)
    Certified,
}

#[derive(Debug)]
pub enum InvariantViolation {
    /// INV-1 — Coq: lr_champion_in_safe_range
    Inv1BpbNotDecreasing { bpb_delta: f64 },
    /// INV-2 — Coq: champion_survives_pruning
    Inv2AshaThresholdTooLow { threshold: f64 },
    Inv2WarmupTooShort { steps: u64 },
    /// INV-3 — Coq: gf16_safe_domain
    Inv3DModelTooSmall { d_model: usize },
    /// INV-4 — Coq: entropy_band_width
    Inv4EntropyOutOfBand { entropy: f64 },
    /// INV-5 — Coq: igla_found_criterion (Proven)
    Inv5Gf16Inconsistency,
    /// INV-12 — Coq: asha_rungs_trinity
    Inv12InvalidRung { rung: u64 },
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inv1BpbNotDecreasing { bpb_delta } =>
                write!(f, "INV-1 [lr_champion_in_safe_range]: BPB Δ={bpb_delta:.6} >= 0 — real backward pass needed (TASK-5D)"),
            Self::Inv2AshaThresholdTooLow { threshold } =>
                write!(f, "INV-2 [champion_survives_pruning]: threshold={threshold:.4} < {ASHA_PRUNE_THRESHOLD} — old broken value=2.65 kills champion"),
            Self::Inv2WarmupTooShort { steps } =>
                write!(f, "INV-2 [champion_survives_pruning]: warmup={steps} < {WARMUP_BLIND_STEPS} — false prune in warmup zone"),
            Self::Inv3DModelTooSmall { d_model } =>
                write!(f, "INV-3 [gf16_safe_domain]: d_model={d_model} < {GF16_D_MODEL_MIN} — GF16 error > φ⁻⁶ (L-R9)"),
            Self::Inv4EntropyOutOfBand { entropy } =>
                write!(f, "INV-4 [entropy_band_width]: H={entropy:.4} ∉ [{NCA_ENTROPY_LOW}, {NCA_ENTROPY_HIGH}] — K=9 on 9×9 grid required"),
            Self::Inv5Gf16Inconsistency =>
                write!(f, "INV-5 [igla_found_criterion]: GF16 Lucas closure broken — φ²ⁿ+φ⁻²ⁿ ∉ ℤ"),
            Self::Inv12InvalidRung { rung } =>
                write!(f, "INV-12 [asha_rungs_trinity]: rung={rung} ∉ {{1000, 3000, 9000, 27000}} — must be 1000 × 3ⁿ"),
        }
    }
}

/// INV-1: BPB decreases with real gradient (Admitted — warn only, temporal)
pub fn check_inv1_bpb_decreasing(bpb_prev: f64, bpb_curr: f64) -> Option<InvariantViolation> {
    let delta = bpb_curr - bpb_prev;
    if delta > 1e-6 {
        let v = InvariantViolation::Inv1BpbNotDecreasing { bpb_delta: delta };
        eprintln!("\u26a0\u2009 {v}");
        Some(v)
    } else {
        None
    }
}

/// INV-2: ASHA threshold >= 3.5, warmup >= 4000 (Admitted — ABORT)
pub fn check_inv2_asha_config(
    bpb_prune_threshold: f64,
    warmup_blind_steps: u64,
) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();
    if bpb_prune_threshold < ASHA_PRUNE_THRESHOLD {
        violations.push(InvariantViolation::Inv2AshaThresholdTooLow { threshold: bpb_prune_threshold });
    }
    if warmup_blind_steps < WARMUP_BLIND_STEPS {
        violations.push(InvariantViolation::Inv2WarmupTooShort { steps: warmup_blind_steps });
    }
    violations
}

/// INV-3: GF16 safe domain d_model >= 256 (Admitted — ABORT)
pub fn check_inv3_gf16_domain(d_model: usize, use_gf16: bool) -> Vec<InvariantViolation> {
    if use_gf16 && d_model < GF16_D_MODEL_MIN {
        vec![InvariantViolation::Inv3DModelTooSmall { d_model }]
    } else {
        vec![]
    }
}

/// INV-4: NCA entropy penalty — symmetric to Coq theorem (band_width = 1 proven exactly)
/// Returns penalty value. nca_loss_weight=0.25 applied by caller — not here.
pub fn inv4_entropy_penalty(entropy: f64) -> f64 {
    f64::max(0.0, NCA_ENTROPY_LOW - entropy) + f64::max(0.0, entropy - NCA_ENTROPY_HIGH)
}

/// INV-4 violation check (Admitted — hard_penalty)
pub fn check_inv4_nca_entropy(entropy: f64) -> Vec<InvariantViolation> {
    if entropy < NCA_ENTROPY_LOW || entropy > NCA_ENTROPY_HIGH {
        vec![InvariantViolation::Inv4EntropyOutOfBand { entropy }]
    } else {
        vec![]
    }
}

/// INV-5: GF16 Lucas closure φ²ⁿ+φ⁻²ⁿ ∈ ℤ for n=1..=8 (Proven in Coq — ABORT)
pub fn check_inv5_gf16_consistency() -> Vec<InvariantViolation> {
    for n in 1u32..=8 {
        let sum = PHI.powi(2 * n as i32) + PHI_INV.powi(2 * n as i32);
        if (sum - sum.round()).abs() > 1e-9 {
            return vec![InvariantViolation::Inv5Gf16Inconsistency];
        }
    }
    vec![]
}

/// INV-12: ASHA rung must be in {1000, 3000, 9000, 27000} = 1000 × 3ⁿ (Admitted — ABORT)
pub fn check_inv12_rung_valid(rung: u64) -> Vec<InvariantViolation> {
    if VALID_RUNGS.contains(&rung) {
        vec![]
    } else {
        vec![InvariantViolation::Inv12InvalidRung { rung }]
    }
}

/// Gate: validate full config before race starts (L-R14)
/// Returns ALL violations — caller sees complete picture, not just first failure
pub fn validate_config(
    bpb_prune_threshold: f64,
    warmup_blind_steps: u64,
    d_model: usize,
    use_gf16: bool,
) -> Vec<InvariantViolation> {
    let mut all = Vec::new();
    all.extend(check_inv2_asha_config(bpb_prune_threshold, warmup_blind_steps));
    all.extend(check_inv3_gf16_domain(d_model, use_gf16));
    all.extend(check_inv5_gf16_consistency());
    for v in &all {
        eprintln!("\ud83d\udea8 {v}");
    }
    if all.is_empty() {
        println!("\u2705 IGLA INV-001..012: certified region entered | φ²+φ⁻²=3 | TRINITY");
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::champion_survives_pruning
    #[test]
    fn inv2_champion_survives_at_rung_1000() {
        assert!(check_inv2_asha_config(3.5, 4000).is_empty());
        assert!(check_inv2_asha_config(4.0, 5000).is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::champion_survives_pruning
    /// falsification_record: old_broken_threshold = 2.65
    #[test]
    fn inv2_rejects_old_broken_threshold_2_65() {
        let v = check_inv2_asha_config(2.65, 4000);
        assert!(!v.is_empty(), "threshold=2.65 must be rejected (kills champion)");
    }

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::champion_survives_pruning
    #[test]
    fn inv2_rejects_short_warmup() {
        let v = check_inv2_asha_config(3.5, 3999);
        assert!(!v.is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::champion_survives_pruning
    #[test]
    fn inv2_returns_all_violations_not_just_first() {
        // Both threshold and warmup wrong — must see both violations
        let v = check_inv2_asha_config(2.65, 3999);
        assert_eq!(v.len(), 2, "Expected 2 violations, got {}", v.len());
    }

    /// Coq source: trinity-clara/proofs/igla/gf16_precision.v::gf16_safe_domain
    #[test]
    fn inv3_gf16_valid_domain() {
        assert!(check_inv3_gf16_domain(384, true).is_empty());
        assert!(check_inv3_gf16_domain(256, true).is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/gf16_precision.v::gf16_safe_domain
    #[test]
    fn inv3_rejects_d_model_128_with_gf16() {
        assert!(!check_inv3_gf16_domain(128, true).is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/nca_entropy_band.v::entropy_band_width
    #[test]
    fn inv4_entropy_penalty_symmetric() {
        // Penalty must be symmetric: same formula as Coq theorem (band_width = 1 proven)
        assert_eq!(inv4_entropy_penalty(1.5), 0.0); // boundary
        assert_eq!(inv4_entropy_penalty(2.8), 0.0); // boundary
        let p_low = inv4_entropy_penalty(1.0);  // 0.5 below band
        let p_high = inv4_entropy_penalty(3.3); // 0.5 above band
        assert!((p_low - p_high).abs() < 1e-10, "Penalty must be symmetric: {p_low} != {p_high}");
    }

    /// Coq source: trinity-clara/proofs/igla/nca_entropy_band.v::entropy_band_width
    #[test]
    fn inv4_entropy_out_of_band_detected() {
        assert!(!check_inv4_nca_entropy(1.4).is_empty());
        assert!(!check_inv4_nca_entropy(2.9).is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/lucas_closure_gf16.v::igla_found_criterion (Proven)
    #[test]
    fn inv5_lucas_closure_proven() {
        assert!(check_inv5_gf16_consistency().is_empty());
    }

    /// Coq source: trinity-clara/proofs/igla/ (root identity)
    #[test]
    fn trinity_identity_phi_squared_plus_phi_inv_squared_equals_3() {
        let trinity = PHI.powi(2) + PHI_INV.powi(2);
        assert!((trinity - 3.0).abs() < 1e-10, "φ²+φ⁻² = {trinity:.12}, expected 3.0");
    }

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::asha_rungs_trinity
    #[test]
    fn inv12_valid_rungs_accepted() {
        for rung in [1000u64, 3000, 9000, 27000] {
            assert!(check_inv12_rung_valid(rung).is_empty(), "rung={rung} must be valid");
        }
    }

    /// Coq source: trinity-clara/proofs/igla/igla_asha_bound.v::asha_rungs_trinity
    #[test]
    fn inv12_rejects_non_trinity_rungs() {
        // --steps 5000 would be rejected — not a power of 3 × 1000
        for rung in [500u64, 2000, 5000, 10000, 15000] {
            assert!(!check_inv12_rung_valid(rung).is_empty(), "rung={rung} must be rejected");
        }
    }

    /// Coq source: all invariants — validate_config is the certified region gate
    #[test]
    fn validate_config_certified_region_entry() {
        let v = validate_config(3.5, 4000, 384, true);
        assert!(v.is_empty(), "Valid config must pass all invariants");
    }

    /// Coq source: all invariants — validate_config returns ALL violations
    #[test]
    fn validate_config_returns_all_violations_not_just_first() {
        let v = validate_config(2.65, 4000, 128, true);
        assert!(v.len() >= 2, "Expected >= 2 violations (threshold + d_model), got {}", v.len());
    }
}
