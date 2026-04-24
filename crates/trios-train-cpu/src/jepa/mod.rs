//! TASK-5A.5 — T-JEPA Public API
//!
//! Ternary Joint Embedding Predictive Architecture.
//! Spec: .trinity/specs/issue143-task5a-jepa-design.md
//! Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/
//! Issue: https://github.com/gHashTag/trios/issues/143 (TASK-5A)

pub mod ema;
pub mod loss;
pub mod masking;
pub mod predictor;

pub use ema::{EmaConfig, EmaTarget, ema_update};
pub use loss::{JepaLoss, JepaLossConfig, compute_jepa_loss, l2_normalize, l2_normalized};
pub use masking::{MaskConfig, MaskResult, get_masked, get_unmasked, mask_spans};
pub use predictor::{PredictionOutput, Predictor, PredictorConfig};

/// T-JEPA training configuration
#[derive(Debug, Clone)]
pub struct JepaConfig {
    pub seed: u64,
    pub d_model: usize,
    pub mask_ratio: f64,
    pub min_span: usize,
    pub max_span: usize,
    pub num_spans: usize,
    pub ema_start: f64,
    pub ema_end: f64,
    /// Weight in multi-objective: L = 0.5*NTP + 0.25*JEPA + 0.25*NCA
    pub jepa_weight: f64,
    pub predictor_lr_mult: f64,
}

impl Default for JepaConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            d_model: 384,
            mask_ratio: 0.3,
            min_span: 3,
            max_span: 9,
            num_spans: 2,
            ema_start: 0.996,
            ema_end: 1.0,
            jepa_weight: 0.25,
            predictor_lr_mult: 0.1,
        }
    }
}

/// T-JEPA training result summary
#[derive(Debug, Clone)]
pub struct JepaResult {
    pub steps_completed: usize,
    pub final_loss: f64,
    pub final_variance: f64,
    pub loss_monotone: bool,
    pub ema_verified: bool,
    /// True if variance > 0.01 (no collapse)
    pub converged: bool,
}

impl JepaResult {
    pub fn is_success(&self) -> bool {
        self.converged && self.ema_verified
    }
}

pub fn mask_config_from_jepa(cfg: &JepaConfig) -> MaskConfig {
    MaskConfig {
        ratio: cfg.mask_ratio,
        min_span: cfg.min_span,
        max_span: cfg.max_span,
        num_spans: cfg.num_spans,
    }
}

pub fn ema_config_from_jepa(cfg: &JepaConfig, total_steps: usize) -> EmaConfig {
    EmaConfig { start: cfg.ema_start, end: cfg.ema_end, ramp_steps: total_steps }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jepa_config_defaults() {
        let cfg = JepaConfig::default();
        assert!((cfg.mask_ratio - 0.3).abs() < 1e-9);
        assert_eq!(cfg.min_span, 3);
        assert_eq!(cfg.max_span, 9);
        assert!((cfg.ema_start - 0.996).abs() < 1e-9);
        assert!((cfg.jepa_weight - 0.25).abs() < 1e-9);
    }
}
