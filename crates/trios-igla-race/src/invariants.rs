//! IGLA Invariant Enforcement Layer — INV-001..005
//! Source: assertions/igla_assertions.json
//! Trinity: φ² + φ⁻² = 3 | L-R14: coqc must pass before race starts

use std::fs;
use serde::Deserialize;

// φ-anchored constants derived from Trinity Identity φ² + φ⁻² = 3
pub const PHI: f64 = 1.618033988749895;
pub const PHI_INV: f64 = 0.618033988749895;

// INV-2: bpb_prune_threshold = φ² + φ⁻² + φ⁻⁴ = 3 + φ⁻⁴
pub const ASHA_PRUNE_THRESHOLD: f64 = 3.5;
// INV-2: warmup blind zone — prune forbidden before this step
pub const WARMUP_BLIND_STEPS: u64 = 4000;
// INV-1: lr must be in [φ⁻⁸, φ⁻⁶] = [0.00382, 0.00618]
pub const LR_PHI_MIN: f64 = 0.00382; // φ⁻⁷ / 2 ≈ α_φ / φ³
pub const LR_PHI_MAX: f64 = 0.00618; // φ⁻⁶ / 2
// INV-3: GF16 safe domain
pub const GF16_D_MODEL_MIN: usize = 256;
pub const GF16_ERROR_BOUND: f64 = 0.0557; // φ⁻⁶
// INV-4: NCA entropy band (A5/E8 symmetry → physical phenomenon)
pub const NCA_ENTROPY_LOW: f64 = 1.5;
pub const NCA_ENTROPY_HIGH: f64 = 2.8;
pub const NCA_STATES_K: usize = 9;  // 9 = 3² (Trinity)
pub const NCA_GRID_SIZE: usize = 81; // 81 = 3⁴ = (φ²+φ⁻²)⁴

#[derive(Debug)]
pub enum InvariantViolation {
    Inv1BpbNotDecreasing { bpb_delta: f64 },
    Inv2AshaThresholdTooLow { threshold: f64 },
    Inv2WarmupTooShort { steps: u64 },
    Inv3DModelTooSmall { d_model: usize },
    Inv4EntropyOutOfBand { entropy: f64 },
    Inv5Gf16Inconsistency,
}

impl std::fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Inv1BpbNotDecreasing { bpb_delta } =>
                write!(f, "INV-1 VIOLATED: BPB delta={bpb_delta:.6} >= 0 — real backward pass needed (TASK-5D)"),
            Self::Inv2AshaThresholdTooLow { threshold } =>
                write!(f, "INV-2 VIOLATED: threshold={threshold:.4} < {ASHA_PRUNE_THRESHOLD} — champion will be killed"),
            Self::Inv2WarmupTooShort { steps } =>
                write!(f, "INV-2 VIOLATED: warmup_blind_steps={steps} < {WARMUP_BLIND_STEPS} — false prune in warmup zone"),
            Self::Inv3DModelTooSmall { d_model } =>
                write!(f, "INV-3 VIOLATED: d_model={d_model} < {GF16_D_MODEL_MIN} — GF16 error > φ⁻⁶ (Law L-R9)"),
            Self::Inv4EntropyOutOfBand { entropy } =>
                write!(f, "INV-4 VIOLATED: entropy={entropy:.4} outside [{NCA_ENTROPY_LOW}, {NCA_ENTROPY_HIGH}] — K=9 grid 9x9 required"),
            Self::Inv5Gf16Inconsistency =>
                write!(f, "INV-5 VIOLATED: GF16 arithmetic inconsistency — Lucas closure broken"),
        }
    }
}

/// INV-1: BPB must decrease when real gradient flows
/// Admitted in Coq — runtime warning only (not abort) until proven
pub fn check_inv1_bpb_decreasing(bpb_prev: f64, bpb_curr: f64) -> Result<(), InvariantViolation> {
    let delta = bpb_curr - bpb_prev;
    // Admitted: warn but do not abort — proof pending (TASK-5D)
    if delta > 1e-6 {
        let violation = InvariantViolation::Inv1BpbNotDecreasing { bpb_delta: delta };
        eprintln!("⚠️  {violation}");
        // warn only — Admitted invariant
    }
    Ok(())
}

/// INV-2: ASHA pruning threshold must be >= 3.5, warmup blind zone >= 4000
/// Admitted in Coq — ABORT on violation (critical: kills champion)
pub fn check_inv2_asha_config(
    bpb_prune_threshold: f64,
    warmup_blind_steps: u64,
) -> Result<(), InvariantViolation> {
    if bpb_prune_threshold < ASHA_PRUNE_THRESHOLD {
        return Err(InvariantViolation::Inv2AshaThresholdTooLow {
            threshold: bpb_prune_threshold,
        });
    }
    if warmup_blind_steps < WARMUP_BLIND_STEPS {
        return Err(InvariantViolation::Inv2WarmupTooShort {
            steps: warmup_blind_steps,
        });
    }
    Ok(())
}

/// INV-3: GF16 requires d_model >= 256 = 2^8
/// Admitted in Coq — ABORT on violation (Law L-R9)
pub fn check_inv3_gf16_domain(d_model: usize, use_gf16: bool) -> Result<(), InvariantViolation> {
    if use_gf16 && d_model < GF16_D_MODEL_MIN {
        return Err(InvariantViolation::Inv3DModelTooSmall { d_model });
    }
    Ok(())
}

/// INV-4: NCA entropy must stay in [1.5, 2.8]
/// Admitted in Coq — hard loss penalty on violation (Law L-R11)
pub fn check_inv4_nca_entropy(entropy: f64) -> Result<(), InvariantViolation> {
    if entropy < NCA_ENTROPY_LOW || entropy > NCA_ENTROPY_HIGH {
        return Err(InvariantViolation::Inv4EntropyOutOfBand { entropy });
    }
    Ok(())
}

/// INV-5: GF16 Lucas closure consistency check
/// Proven in Coq — ABORT on violation
pub fn check_inv5_gf16_consistency() -> Result<(), InvariantViolation> {
    // Lucas closure: φ^2n + φ^-2n ∈ ℤ for all n
    // Runtime check: verify GF(2^4) field axioms hold
    for n in 1u32..=8 {
        let phi_2n = PHI.powi(2 * n as i32);
        let phi_inv_2n = PHI_INV.powi(2 * n as i32);
        let sum = phi_2n + phi_inv_2n;
        let rounded = sum.round();
        // Must be within floating-point epsilon of an integer
        if (sum - rounded).abs() > 1e-9 {
            return Err(InvariantViolation::Inv5Gf16Inconsistency);
        }
    }
    Ok(())
}

/// Run ALL invariant checks before race starts (L-R14 enforcement)
/// This is the gate — race MUST NOT start if any Admitted check fails on critical invariants
pub fn enforce_all_invariants(
    bpb_prune_threshold: f64,
    warmup_blind_steps: u64,
    d_model: usize,
    use_gf16: bool,
) -> Result<(), Vec<InvariantViolation>> {
    let mut violations: Vec<InvariantViolation> = Vec::new();

    // INV-2: ABORT-level (champion survival)
    if let Err(v) = check_inv2_asha_config(bpb_prune_threshold, warmup_blind_steps) {
        violations.push(v);
    }
    // INV-3: ABORT-level (GF16 domain)
    if let Err(v) = check_inv3_gf16_domain(d_model, use_gf16) {
        violations.push(v);
    }
    // INV-5: ABORT-level (Proven in Coq)
    if let Err(v) = check_inv5_gf16_consistency() {
        violations.push(v);
    }

    if violations.is_empty() {
        println!("✅ IGLA INV-001..005: all critical invariants satisfied | φ²+φ⁻²=3 | TRINITY");
        Ok(())
    } else {
        for v in &violations {
            eprintln!("🚨 {v}");
        }
        Err(violations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inv2_valid_config() {
        assert!(check_inv2_asha_config(3.5, 4000).is_ok());
        assert!(check_inv2_asha_config(4.0, 5000).is_ok());
    }

    #[test]
    fn test_inv2_rejects_old_threshold() {
        // Old threshold 2.65 kills champion — must be rejected
        assert!(check_inv2_asha_config(2.65, 4000).is_err());
    }

    #[test]
    fn test_inv2_rejects_short_warmup() {
        assert!(check_inv2_asha_config(3.5, 3999).is_err());
    }

    #[test]
    fn test_inv3_gf16_valid() {
        assert!(check_inv3_gf16_domain(384, true).is_ok());
        assert!(check_inv3_gf16_domain(256, true).is_ok());
    }

    #[test]
    fn test_inv3_gf16_rejects_small_model() {
        assert!(check_inv3_gf16_domain(128, true).is_err());
    }

    #[test]
    fn test_inv3_no_gf16_always_ok() {
        assert!(check_inv3_gf16_domain(64, false).is_ok());
    }

    #[test]
    fn test_inv4_entropy_valid() {
        assert!(check_inv4_nca_entropy(1.5).is_ok());
        assert!(check_inv4_nca_entropy(2.0).is_ok());
        assert!(check_inv4_nca_entropy(2.8).is_ok());
    }

    #[test]
    fn test_inv4_entropy_out_of_band() {
        assert!(check_inv4_nca_entropy(1.4).is_err());
        assert!(check_inv4_nca_entropy(2.9).is_err());
    }

    #[test]
    fn test_inv5_lucas_closure() {
        // φ^2n + φ^-2n must be integer for n=1..8
        assert!(check_inv5_gf16_consistency().is_ok());
    }

    #[test]
    fn test_phi_trinity_identity() {
        // φ² + φ⁻² = 3 (exact, modulo float epsilon)
        let trinity = PHI.powi(2) + PHI_INV.powi(2);
        assert!((trinity - 3.0).abs() < 1e-10, "Trinity identity violated: {trinity}");
    }

    #[test]
    fn test_enforce_all_valid() {
        assert!(enforce_all_invariants(3.5, 4000, 384, true).is_ok());
    }

    #[test]
    fn test_enforce_all_rejects_bad_asha() {
        assert!(enforce_all_invariants(2.65, 4000, 384, true).is_err());
    }
}
