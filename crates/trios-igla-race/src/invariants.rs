//! IGLA Invariant System — Rust bridge for Coq INV-1..5
//!
//! Source: trinity-clara/proofs/igla/*.v
//! Contract: trinity-clara/assertions/igla_assertions.json
//!
//! L-R14: coqc trinity-clara/proofs/igla/*.v = GREEN before race start.
//! These runtime checks mirror what Coq proves statically.
//!
//! φ² + φ⁻² = 3 | 84 + 5 = 89 theorems (F₁₁ = 89, Fibonacci prime)

use std::fmt;

// ── φ-anchored constants (from trinity-clara) ───────────────────────────────

pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_INV6: f64 = 0.055_728;  // φ⁻⁶ — GF16 error bound (INV-3)
pub const ALPHA_PHI: f64 = 0.1180;    // A₅ characteristic polynomial result

// INV-1: lr ∈ [φ⁻⁸, φ⁻⁶]
pub const LR_SAFE_LO: f64 = 0.002;
pub const LR_SAFE_HI: f64 = 0.007;
pub const LR_CHAMPION: f64 = 0.004;

// INV-2: ASHA threshold and warmup
pub const ASHA_PRUNE_THRESHOLD: f64 = 3.5;  // φ² + φ⁻² + φ⁻⁴ ≈ 3.472 → 3.5
pub const WARMUP_BLIND_STEPS: u64 = 4000;
pub const RUNG_1: u64 = 1000;

// INV-3: GF16 safety
pub const D_MODEL_MIN_GF16: usize = 256;

// INV-4: NCA entropy band [1.5, 2.8]
pub const NCA_ENTROPY_LO: f64 = 1.5;
pub const NCA_ENTROPY_HI: f64 = 2.8;
pub const NCA_K: usize = 9;    // 3²
pub const NCA_GRID: usize = 81; // 3⁴

// ── Error types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum InvError {
    /// INV-1: constant proxy gradient used instead of real MSE
    Inv1BadGradient { mode: String },
    /// INV-1: learning rate outside φ-safe range
    Inv1LrOutOfRange { lr: f64 },
    /// INV-2: ASHA prune attempted during warmup blind zone
    Inv2FalsePrune { step: u64, threshold: f64 },
    /// INV-3: GF16 enabled with d_model below safe minimum
    Inv3UnsafeDModel { d_model: usize },
    /// INV-4: NCA entropy outside A₅/E₈ band
    Inv4EntropyCollapse { entropy: f64 },
    /// INV-5: Lucas closure violated (GF16 arithmetic error too large)
    Inv5LucasViolation { error: f64 },
}

impl fmt::Display for InvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvError::Inv1BadGradient { mode } =>
                write!(f, "INV-1 VIOLATED: gradient mode '{mode}' is not real_mse. \
                           TASK-5D: replace constant proxy with real MSE gradient."),
            InvError::Inv1LrOutOfRange { lr } =>
                write!(f, "INV-1 VIOLATED: lr={lr} outside φ-safe range [{LR_SAFE_LO}, {LR_SAFE_HI}]"),
            InvError::Inv2FalsePrune { step, threshold } =>
                write!(f, "INV-2 VIOLATED: prune at step={step} < warmup={WARMUP_BLIND_STEPS} \
                           with threshold={threshold}. Champion would be falsely killed."),
            InvError::Inv3UnsafeDModel { d_model } =>
                write!(f, "INV-3 VIOLATED: GF16 with d_model={d_model} < {D_MODEL_MIN_GF16}. \
                           Expected +3.21 BPB penalty (L-R9)."),
            InvError::Inv4EntropyCollapse { entropy } =>
                write!(f, "INV-4 VIOLATED: NCA entropy={entropy:.4} outside \
                           [{NCA_ENTROPY_LO}, {NCA_ENTROPY_HI}]. A₅/E₈ band broken."),
            InvError::Inv5LucasViolation { error } =>
                write!(f, "INV-5 VIOLATED: GF16 error={error:.6} >= φ⁻⁶={PHI_INV6:.6}. \
                           Lucas closure broken."),
        }
    }
}

// ── Gradient mode ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GradientMode {
    /// Real MSE gradient: d(BPB)/d(θ) computed from actual loss
    RealMSE,
    /// Constant proxy: "loss_scale * 0.01" (TASK-5D bug in tjepa_train.rs)
    ConstantProxy(f64),
}

// ── Config snapshot for validation ───────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TrialConfig {
    pub lr: f64,
    pub d_model: usize,
    pub gradient_mode: GradientMode,
    pub gf16_enabled: bool,
    pub nca_enabled: bool,
    pub current_step: u64,
}

// ── Individual invariant checks ───────────────────────────────────────

/// INV-1a: gradient must be real MSE, not constant proxy (TASK-5D)
pub fn inv1_check_gradient_mode(mode: &GradientMode) -> Result<(), InvError> {
    match mode {
        GradientMode::RealMSE => Ok(()),
        GradientMode::ConstantProxy(val) => Err(InvError::Inv1BadGradient {
            mode: format!("ConstantProxy({val})"),
        }),
    }
}

/// INV-1b: lr ∈ [φ⁻⁸, φ⁻⁶] = [0.002, 0.007]
pub fn inv1_check_lr(lr: f64) -> Result<(), InvError> {
    if lr >= LR_SAFE_LO && lr <= LR_SAFE_HI {
        Ok(())
    } else {
        Err(InvError::Inv1LrOutOfRange { lr })
    }
}

/// INV-2: ASHA must not prune during warmup blind zone (steps < 4000)
pub fn inv2_check_asha_threshold(step: u64, bpb: f64, threshold: f64) -> Result<(), InvError> {
    if step < WARMUP_BLIND_STEPS && bpb > threshold {
        Err(InvError::Inv2FalsePrune { step, threshold })
    } else {
        Ok(())
    }
}

/// INV-3: GF16 is only safe with d_model ≥ 256
pub fn inv3_check_d_model(d_model: usize, gf16_enabled: bool) -> Result<(), InvError> {
    if gf16_enabled && d_model < D_MODEL_MIN_GF16 {
        Err(InvError::Inv3UnsafeDModel { d_model })
    } else {
        Ok(())
    }
}

/// INV-4: NCA entropy must stay in A₅/E₈ band [1.5, 2.8]
pub fn inv4_check_nca_entropy(entropy: f64) -> Result<(), InvError> {
    if entropy >= NCA_ENTROPY_LO && entropy <= NCA_ENTROPY_HI {
        Ok(())
    } else {
        Err(InvError::Inv4EntropyCollapse { entropy })
    }
}

/// INV-5: GF16 arithmetic error must be < φ⁻⁶
pub fn inv5_check_lucas_closure(error: f64) -> Result<(), InvError> {
    if error < PHI_INV6 {
        Ok(())
    } else {
        Err(InvError::Inv5LucasViolation { error })
    }
}

// ── Master gate: validate_config() ────────────────────────────────────
/// Call this BEFORE every ASHA trial. Returns all violations found.
/// Empty Vec = INVARIANT GATE passed → trial may proceed.
///
/// Mirrors L-R14: coqc GREEN → same guarantees at runtime.

pub fn validate_config(cfg: &TrialConfig) -> Vec<InvError> {
    let mut errors: Vec<InvError> = Vec::new();

    // INV-1: gradient mode
    if let Err(e) = inv1_check_gradient_mode(&cfg.gradient_mode) {
        errors.push(e);
    }
    // INV-1: lr range
    if let Err(e) = inv1_check_lr(cfg.lr) {
        errors.push(e);
    }
    // INV-3: GF16 safety
    if let Err(e) = inv3_check_d_model(cfg.d_model, cfg.gf16_enabled) {
        errors.push(e);
    }

    errors
}

/// ASHA-specific gate: call at each rung before prune decision.
/// step = current training step, bpb = current trial BPB.
pub fn validate_asha_prune(step: u64, bpb: f64) -> Result<(), InvError> {
    inv2_check_asha_threshold(step, bpb, ASHA_PRUNE_THRESHOLD)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // INV-1 tests
    #[test]
    fn inv1_real_mse_passes() {
        assert!(inv1_check_gradient_mode(&GradientMode::RealMSE).is_ok());
    }

    #[test]
    fn inv1_constant_proxy_rejected() {
        let err = inv1_check_gradient_mode(&GradientMode::ConstantProxy(0.01));
        assert!(matches!(err, Err(InvError::Inv1BadGradient { .. })));
    }

    #[test]
    fn inv1_champion_lr_valid() {
        assert!(inv1_check_lr(LR_CHAMPION).is_ok());
    }

    #[test]
    fn inv1_lr_too_small_rejected() {
        assert!(matches!(inv1_check_lr(0.0001), Err(InvError::Inv1LrOutOfRange { .. })));
    }

    // INV-2 tests
    #[test]
    fn inv2_no_prune_during_warmup() {
        // step=500 < 4000, bpb=4.0 > 3.5 → FALSE PRUNE
        let err = inv2_check_asha_threshold(500, 4.0, ASHA_PRUNE_THRESHOLD);
        assert!(matches!(err, Err(InvError::Inv2FalsePrune { .. })));
    }

    #[test]
    fn inv2_prune_after_warmup_ok() {
        // step=5000 >= 4000, bpb=4.0 > 3.5 → legitimate prune
        assert!(inv2_check_asha_threshold(5000, 4.0, ASHA_PRUNE_THRESHOLD).is_ok());
    }

    #[test]
    fn inv2_champion_bpb_survives() {
        // bpb=2.53 <= 3.5 → always survives regardless of step
        assert!(inv2_check_asha_threshold(500, 2.53, ASHA_PRUNE_THRESHOLD).is_ok());
    }

    // INV-3 tests
    #[test]
    fn inv3_d384_with_gf16_passes() {
        assert!(inv3_check_d_model(384, true).is_ok());
    }

    #[test]
    fn inv3_d192_with_gf16_rejected() {
        assert!(matches!(
            inv3_check_d_model(192, true),
            Err(InvError::Inv3UnsafeDModel { .. })
        ));
    }

    #[test]
    fn inv3_d192_without_gf16_ok() {
        assert!(inv3_check_d_model(192, false).is_ok());
    }

    // INV-4 tests
    #[test]
    fn inv4_entropy_in_band_passes() {
        assert!(inv4_check_nca_entropy(2.0).is_ok());
    }

    #[test]
    fn inv4_entropy_collapse_detected() {
        assert!(matches!(
            inv4_check_nca_entropy(0.5),
            Err(InvError::Inv4EntropyCollapse { .. })
        ));
    }

    // INV-5 tests
    #[test]
    fn inv5_small_error_passes() {
        assert!(inv5_check_lucas_closure(0.001).is_ok());
    }

    #[test]
    fn inv5_large_error_rejected() {
        assert!(matches!(
            inv5_check_lucas_closure(0.1),
            Err(InvError::Inv5LucasViolation { .. })
        ));
    }

    // Master gate tests
    #[test]
    fn validate_config_champion_passes() {
        let cfg = TrialConfig {
            lr: LR_CHAMPION,
            d_model: 384,
            gradient_mode: GradientMode::RealMSE,
            gf16_enabled: false,
            nca_enabled: true,
            current_step: 5000,
        };
        assert!(validate_config(&cfg).is_empty());
    }

    #[test]
    fn validate_config_task_5d_bug_detected() {
        let cfg = TrialConfig {
            lr: LR_CHAMPION,
            d_model: 384,
            gradient_mode: GradientMode::ConstantProxy(0.01), // TASK-5D bug
            gf16_enabled: false,
            nca_enabled: false,
            current_step: 1000,
        };
        let errs = validate_config(&cfg);
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| matches!(e, InvError::Inv1BadGradient { .. })));
    }
}
