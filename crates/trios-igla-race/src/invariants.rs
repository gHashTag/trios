//! IGLA Invariant Runtime Bridge
//!
//! Connects Coq-proven invariants (trinity-clara/proofs/igla/*.v)
//! to Rust trial validation. Every trial calls `validate_config()`
//! before training begins — invalid configs are skipped at zero GPU cost.
//!
//! L-R14: coqc proofs/igla/*.v = GREEN is a prerequisite.
//! JSON source: assertions/igla_assertions.json
//! Refs: trios #143, TASK-COQ-001, TASK-5D

use std::fmt;

// ================================================================
// INV constants (extracted from Coq proofs)
// ================================================================

/// INV-1: φ-safe lr range [α_φ/φ^4, α_φ/φ^2]
pub const INV1_LR_SAFE_LO: f64 = 0.002;
pub const INV1_LR_SAFE_HI: f64 = 0.007;
pub const INV1_CHAMPION_LR: f64 = 0.004;
pub const INV1_SMOOTHNESS_L: f64 = 2.0;

/// INV-2: ASHA threshold = φ² + φ⁻² + φ⁻⁴ = 3.5
pub const INV2_BPB_PRUNE_THRESHOLD: f64 = 3.5;
pub const INV2_WARMUP_BLIND_STEPS: u64 = 4000;

/// INV-3: GF16 safe domain
pub const INV3_D_MODEL_MIN: usize = 256;

/// INV-4: NCA entropy band
pub const INV4_ENTROPY_LO: f64 = 1.5;
pub const INV4_ENTROPY_HI: f64 = 2.8;
pub const INV4_NCA_GRID: usize = 81; // 3^4 = (\u03c6²+\u03c6⁻²)^4
pub const INV4_NCA_K_STATES: usize = 9; // 3^2

// ================================================================
// Types
// ================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum GradientMode {
    /// Real MSE gradient from loss.rs — required by INV-1
    RealMSE,
    /// Constant proxy: loss_scale * 0.01 — TASK-5D bug
    ConstantProxy(f64),
}

#[derive(Debug, Clone)]
pub struct TrialConfig {
    pub lr: f64,
    pub d_model: usize,
    pub bpb_prune_threshold: f64,
    pub warmup_blind_steps: u64,
    pub use_gf16: bool,
    pub nca_grid: usize,
    pub nca_k_states: usize,
    pub grad_mode: GradientMode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InvError {
    /// INV-1: gradient mode is ConstantProxy, not RealMSE
    Inv1BadGradient,
    /// INV-1: lr outside φ-safe range [0.002, 0.007]
    Inv1LrOutOfBand(f64),
    /// INV-2: threshold < 3.5, kills champion trial
    Inv2ThresholdTooLow(f64),
    /// INV-3: GF16 used with d_model < 256
    Inv3UnsafeDomain(usize),
    /// INV-4: NCA grid != 81 or K != 9
    Inv4GridMismatch { grid: usize, k: usize },
}

impl fmt::Display for InvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InvError::Inv1BadGradient =>
                write!(f, "INV-1 VIOLATED: gradient=ConstantProxy. Fix TASK-5D: use real MSE from loss.rs"),
            InvError::Inv1LrOutOfBand(lr) =>
                write!(f, "INV-1 VIOLATED: lr={lr} outside \u03c6-safe [{INV1_LR_SAFE_LO}, {INV1_LR_SAFE_HI}]"),
            InvError::Inv2ThresholdTooLow(t) =>
                write!(f, "INV-2 VIOLATED: threshold={t} < 3.5 kills champion. Set bpb_prune_threshold=3.5"),
            InvError::Inv3UnsafeDomain(d) =>
                write!(f, "INV-3 VIOLATED: GF16 with d_model={d} < 256. Error > \u03c6^{{-6}} guaranteed"),
            InvError::Inv4GridMismatch { grid, k } =>
                write!(f, "INV-4 VIOLATED: NCA grid={grid} K={k}, expected 81/9 (3^4/3^2)"),
        }
    }
}

// ================================================================
// Individual invariant checks
// ================================================================

/// INV-1: gradient mode must be RealMSE
/// Coq: `bad_gradient_no_convergence_guarantee` proves ConstantProxy is unsound
pub fn inv1_check_gradient_mode(mode: &GradientMode) -> Result<(), InvError> {
    match mode {
        GradientMode::RealMSE => Ok(()),
        GradientMode::ConstantProxy(_) => Err(InvError::Inv1BadGradient),
    }
}

/// INV-1: lr must be in \u03c6-safe range
/// Coq: `champion_lr_in_safe_range` proves lr=0.004 \u2208 [0.002, 0.007]
pub fn inv1_check_lr(lr: f64) -> Result<(), InvError> {
    if lr < INV1_LR_SAFE_LO || lr > INV1_LR_SAFE_HI {
        Err(InvError::Inv1LrOutOfBand(lr))
    } else {
        Ok(())
    }
}

/// INV-2: prune threshold must be >= 3.5
/// Coq: `champion_survives_pruning` proves threshold=3.5 is safe
pub fn inv2_check_threshold(threshold: f64) -> Result<(), InvError> {
    if threshold < INV2_BPB_PRUNE_THRESHOLD {
        Err(InvError::Inv2ThresholdTooLow(threshold))
    } else {
        Ok(())
    }
}

/// INV-3: GF16 requires d_model >= 256
/// Coq: `gf16_safe_domain` proves error < \u03c6^{-6} only for d_model\u2265256
pub fn inv3_check_gf16_domain(use_gf16: bool, d_model: usize) -> Result<(), InvError> {
    if use_gf16 && d_model < INV3_D_MODEL_MIN {
        Err(InvError::Inv3UnsafeDomain(d_model))
    } else {
        Ok(())
    }
}

/// INV-4: NCA grid must be 81 (3^4), K must be 9 (3^2)
/// Coq: `nca_entropy_valid` proves entropy\u2208[1.5,2.8] only for these values
pub fn inv4_check_nca_grid(grid: usize, k: usize) -> Result<(), InvError> {
    if grid != INV4_NCA_GRID || k != INV4_NCA_K_STATES {
        Err(InvError::Inv4GridMismatch { grid, k })
    } else {
        Ok(())
    }
}

// ================================================================
// Master validator — call before every trial
// ================================================================

/// Validate a trial config against all 5 Coq invariants.
/// Invalid configs are skipped at zero GPU cost.
///
/// # Returns
/// `Ok(())` if all invariants pass — trial may proceed.
/// `Err(InvError)` if any invariant is violated — skip this config.
pub fn validate_config(cfg: &TrialConfig) -> Result<(), InvError> {
    // INV-1a: gradient mode
    inv1_check_gradient_mode(&cfg.grad_mode)?;
    // INV-1b: lr in \u03c6-safe band
    inv1_check_lr(cfg.lr)?;
    // INV-2: ASHA threshold
    inv2_check_threshold(cfg.bpb_prune_threshold)?;
    // INV-3: GF16 domain safety
    inv3_check_gf16_domain(cfg.use_gf16, cfg.d_model)?;
    // INV-4: NCA grid (only checked if NCA is used)
    if cfg.nca_grid > 0 {
        inv4_check_nca_grid(cfg.nca_grid, cfg.nca_k_states)?;
    }
    Ok(())
}

// ================================================================
// Tests
// ================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn champion_config() -> TrialConfig {
        TrialConfig {
            lr: INV1_CHAMPION_LR,
            d_model: 384,
            bpb_prune_threshold: INV2_BPB_PRUNE_THRESHOLD,
            warmup_blind_steps: INV2_WARMUP_BLIND_STEPS,
            use_gf16: false,
            nca_grid: INV4_NCA_GRID,
            nca_k_states: INV4_NCA_K_STATES,
            grad_mode: GradientMode::RealMSE,
        }
    }

    #[test]
    fn test_champion_config_valid() {
        assert!(validate_config(&champion_config()).is_ok());
    }

    #[test]
    fn test_inv1_constant_proxy_blocked() {
        let mut cfg = champion_config();
        cfg.grad_mode = GradientMode::ConstantProxy(0.01);
        assert_eq!(validate_config(&cfg), Err(InvError::Inv1BadGradient));
    }

    #[test]
    fn test_inv2_old_threshold_blocked() {
        let mut cfg = champion_config();
        cfg.bpb_prune_threshold = 2.65; // original bug value
        assert_eq!(validate_config(&cfg), Err(InvError::Inv2ThresholdTooLow(2.65)));
    }

    #[test]
    fn test_inv3_gf16_small_d_model_blocked() {
        let mut cfg = champion_config();
        cfg.use_gf16 = true;
        cfg.d_model = 128; // unsafe
        assert_eq!(validate_config(&cfg), Err(InvError::Inv3UnsafeDomain(128)));
    }

    #[test]
    fn test_inv4_wrong_grid_blocked() {
        let mut cfg = champion_config();
        cfg.nca_grid = 64; // not 3^4
        assert_eq!(validate_config(&cfg), Err(InvError::Inv4GridMismatch { grid: 64, k: 9 }));
    }

    #[test]
    fn test_inv1_lr_too_high_blocked() {
        let mut cfg = champion_config();
        cfg.lr = 0.05; // outside \u03c6-band
        assert_eq!(validate_config(&cfg), Err(InvError::Inv1LrOutOfBand(0.05)));
    }

    #[test]
    fn test_gf16_safe_with_d384() {
        let mut cfg = champion_config();
        cfg.use_gf16 = true;
        cfg.d_model = 384; // INV-3 safe
        assert!(validate_config(&cfg).is_ok());
    }
}
