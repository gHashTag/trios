//! IGLA RACE — V1: muP / muTransfer Hyperparameter Transfer
//!
//! Implements μP (maximal update parametrization) per Yang & Hu 2021
//! (Tensor Programs V: https://arxiv.org/abs/2203.03466)
//!
//! Key insight: optimal learning rate and betas transfer from a small
//! proxy model to a larger target model without re-tuning when using
//! width-based parametrization.
//!
//! Falsification: coord_check plot must show l1 activation norm is
//! flat across width sweep {8, 16, 32, 64} — if it slopes, muP is
//! not implemented correctly and V1 should be disabled.

use serde::{Deserialize, Serialize};

/// Golden ratio φ — Trinity anchor
pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_SQUARED: f64 = PHI * PHI;

/// Base width for muP scaling (reference dimension)
pub const MUP_BASE_WIDTH: usize = 64;

/// muP scaling factor: lr scales as 1/√(d_model) per Yang & Hu 2021
///
/// For a model of width `d`, the learning rate should be:
///     lr(d) = lr(base) * √(base) / √(d)
///         = lr(base) / mup_lr_scale_factor(d)
///
/// At d=64 (base), scale_factor = 1.0 (no adjustment)
pub fn mup_lr_scale_factor(d_model: usize) -> f64 {
    let base = MUP_BASE_WIDTH as f64;
    let d = d_model.max(1) as f64;
    (d / base).sqrt()
}

/// Inverse scaling factor: multiply champion LR by this to get scaled LR
///
/// Usage: lr_scaled = lr_champion / mup_lr_scale_factor(d_model)
pub fn mup_lr_scale_from_champion(lr_champion: f64, d_model: usize) -> f64 {
    lr_champion / mup_lr_scale_factor(d_model)
}

/// μP parametrization wrapper for model dimensions
///
/// Width-based parametrization ensures that optimal hyperparameters
/// transfer across model scales.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MuParam {
    /// Base vocabulary size
    pub vocab_size: usize,

    /// Base hidden dimension (reference width)
    pub d_model_base: usize,

    /// Target hidden dimension
    pub d_model_target: usize,

    /// Champion LR at base width (typically 0.004)
    pub lr_champion: f64,
}

impl MuParam {
    /// Create new μP parametrization
    pub fn new(vocab_size: usize, d_model_base: usize, d_model_target: usize) -> Self {
        Self {
            vocab_size,
            d_model_base,
            d_model_target,
            lr_champion: 0.004, // INV-8 champion LR
        }
    }

    /// Create from base width to target
    pub fn from_base(vocab_size: usize, d_model_target: usize) -> Self {
        Self {
            vocab_size,
            d_model_base: MUP_BASE_WIDTH,
            d_model_target,
            lr_champion: 0.004,
        }
    }

    /// Get LR scaling factor for target width
    pub fn lr_scale(&self) -> f64 {
        mup_lr_scale_factor(self.d_model_target)
    }

    /// Get scaled LR for target model
    pub fn lr_scaled(&self) -> f64 {
        self.lr_champion / self.lr_scale()
    }

    /// Check if widths are in valid ratio (power of PHI-ish)
    ///
    /// μP transfer works best when width ratio is ≈ φ^k for integer k.
    /// This returns false if the ratio deviates significantly.
    pub fn valid_width_ratio(&self) -> bool {
        let ratio = self.d_model_target as f64 / self.d_model_base as f64;

        // Check if ratio is close to φ^k for some integer k ∈ [-2, 4]
        for k in -2i32..=4 {
            let expected = PHI.powi(k);
            if (ratio - expected).abs() / expected < 0.15 {
                return true;
            }
        }

        // Also allow exact powers of 2 for practical reasons
        let log2_ratio = ratio.log2();
        (log2_ratio - log2_ratio.round()).abs() < 0.1
    }

    /// Get width ratio as f64
    pub fn width_ratio(&self) -> f64 {
        self.d_model_target as f64 / self.d_model_base as f64
    }
}

impl Default for MuParam {
    fn default() -> Self {
        Self::from_base(729, 384) // 3^6 vocab, 384 hidden (champion config)
    }
}

/// Coordinate check data point
///
/// Used for falsification: activation l1 norm should be flat across widths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordCheckPoint {
    pub width: usize,
    pub l1_norm_mean: f64,
    pub l1_norm_std: f64,
    pub steps: usize,
}

/// Result of coordinate check test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordCheckResult {
    Pass { max_slope: f64 },
    Fail { slope: f64, threshold: f64 },
}

impl CoordCheckResult {
    /// Threshold for l1 norm slope (0.1 = 10% change across widths)
    pub const SLOPE_THRESHOLD: f64 = 0.1;

    /// Check if coord check passed
    pub fn is_pass(&self) -> bool {
        matches!(self, CoordCheckResult::Pass { .. })
    }

    /// Get slope value regardless of result
    pub fn slope(&self) -> f64 {
        match self {
            CoordCheckResult::Pass { max_slope } => *max_slope,
            CoordCheckResult::Fail { slope, .. } => *slope,
        }
    }
}

/// Run coordinate check on a set of width measurements
///
/// Falsification criterion: if l1 activation norm slopes > 0.1 across
/// widths {8, 16, 32, 64}, muP is NOT correctly implemented.
pub fn coord_check(points: &[CoordCheckPoint]) -> CoordCheckResult {
    if points.len() < 2 {
        return CoordCheckResult::Fail {
            slope: 1.0,
            threshold: CoordCheckResult::SLOPE_THRESHOLD,
        };
    }

    // Compute linear regression: l1_norm vs log(width)
    let n = points.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_xx = 0.0;

    for p in points {
        let x = (p.width as f64).ln();
        let y = p.l1_norm_mean;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_xx += x * x;
    }

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);

    // Normalize slope by mean y to get relative slope
    let mean_y = sum_y / n;
    let relative_slope = (slope / mean_y).abs();

    if relative_slope <= CoordCheckResult::SLOPE_THRESHOLD {
        CoordCheckResult::Pass {
            max_slope: relative_slope,
        }
    } else {
        CoordCheckResult::Fail {
            slope: relative_slope,
            threshold: CoordCheckResult::SLOPE_THRESHOLD,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mup_lr_scale_at_base() {
        // At base width 64, scale factor should be 1.0
        let scale = mup_lr_scale_factor(64);
        assert!((scale - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_mup_lr_scale_at_256() {
        // At width 256 (4x base), scale factor should be 2.0
        let scale = mup_lr_scale_factor(256);
        assert!((scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_lr_scaled_from_champion() {
        let mup = MuParam::from_base(729, 256);
        let lr = mup.lr_scaled();

        // Champion LR = 0.004, at 256 width should be ~0.002
        assert!((lr - 0.002).abs() < 0.0005);
    }

    #[test]
    fn test_valid_width_ratio_phi() {
        let mup = MuParam::new(729, 64, 104); // ~φ * 64
        assert!(mup.valid_width_ratio());
    }

    #[test]
    fn test_valid_width_ratio_powers_of_2() {
        let mup = MuParam::new(729, 64, 256); // 4x
        assert!(mup.valid_width_ratio());
    }

    #[test]
    fn test_invalid_width_ratio() {
        let mup = MuParam::new(729, 64, 200); // Not φ^k or 2^k
        // 200/64 = 3.125, not close to φ or power of 2
        assert!(!mup.valid_width_ratio());
    }

    #[test]
    fn test_coord_check_flat_passes() {
        let points = vec![
            CoordCheckPoint {
                width: 8,
                l1_norm_mean: 1.0,
                l1_norm_std: 0.1,
                steps: 1000,
            },
            CoordCheckPoint {
                width: 16,
                l1_norm_mean: 1.02,
                l1_norm_std: 0.1,
                steps: 1000,
            },
            CoordCheckPoint {
                width: 32,
                l1_norm_mean: 0.98,
                l1_norm_std: 0.1,
                steps: 1000,
            },
            CoordCheckPoint {
                width: 64,
                l1_norm_mean: 1.01,
                l1_norm_std: 0.1,
                steps: 1000,
            },
        ];

        let result = coord_check(&points);
        assert!(result.is_pass());
    }

    #[test]
    fn test_coord_check_slope_fails() {
        let points = vec![
            CoordCheckPoint {
                width: 8,
                l1_norm_mean: 1.0,
                l1_norm_std: 0.1,
                steps: 1000,
            },
            CoordCheckPoint {
                width: 64,
                l1_norm_mean: 2.0, // 2x increase = slope too high
                l1_norm_std: 0.1,
                steps: 1000,
            },
        ];

        let result = coord_check(&points);
        assert!(!result.is_pass());
        assert!(result.slope() > CoordCheckResult::SLOPE_THRESHOLD);
    }

    #[test]
    fn test_mup_default_champion_config() {
        let mup = MuParam::default();
        assert_eq!(mup.vocab_size, 729); // 3^6
        assert_eq!(mup.d_model_base, 64);
        assert_eq!(mup.d_model_target, 384);
        assert_eq!(mup.lr_champion, 0.004);
    }
}
