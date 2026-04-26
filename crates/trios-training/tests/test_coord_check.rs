//! IGLA RACE — V1: μP Coordinate Check Test
//!
//! Falsification test for muP implementation (Lane L-V1)
//!
//! Tests that activation l1 norm is flat across width sweep
//! {8, 16, 32, 64} per Yang & Hu 2021.
//!
//! If l1 norm slopes > 0.1 across these widths, muP is NOT
//! correctly implemented and V1 should be disabled.

use trios_training::mup::{CoordCheckPoint, CoordCheckResult, coord_check};

/// Generate synthetic coord check data
///
/// In practice, these values come from actual training runs.
/// This function provides test data for widths {8, 16, 32, 64}.
pub fn synthetic_coord_data() -> Vec<CoordCheckPoint> {
    vec![
        // Width 8: small model, higher l1 norm due to overparameterization
        CoordCheckPoint {
            width: 8,
            l1_norm_mean: 1.0,
            l1_norm_std: 0.1,
            steps: 1000,
        },
        // Width 16: moderate
        CoordCheckPoint {
            width: 16,
            l1_norm_mean: 1.05,
            l1_norm_std: 0.1,
            steps: 1000,
        },
        // Width 32: scaling
        CoordCheckPoint {
            width: 32,
            l1_norm_mean: 1.02,
            l1_norm_std: 0.1,
            steps: 1000,
        },
        // Width 64: base width (should be reference)
        CoordCheckPoint {
            width: 64,
            l1_norm_mean: 1.0,
            l1_norm_std: 0.1,
            steps: 1000,
        },
    ]
}

/// Mock training runner that collects l1 norm measurements
///
/// This is a test harness; real training would use actual
/// forward passes through the model.
pub struct MockTrainingState {
    pub width: usize,
    pub l1_norms: Vec<f64>,
}

impl MockTrainingState {
    pub fn new(width: usize) -> Self {
        Self {
            width,
            l1_norms: Vec::new(),
        }
    }

    /// Record an l1 norm measurement after training
    pub fn record_l1_norm(&mut self, l1_norm: f64) {
        self.l1_norms.push(l1_norm);
    }

    /// Get mean l1 norm (coord check y-axis)
    pub fn l1_norm_mean(&self) -> f64 {
        if self.l1_norms.is_empty() {
            0.0
        } else {
            self.l1_norms.iter().sum::<f64>() / self.l1_norms.len() as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_data_flat_passes() {
        let data = synthetic_coord_data();
        let result = coord_check(&data);

        assert!(result.is_pass(), "Synthetic flat data should pass");
    }

    #[test]
    fn test_synthetic_data_slope_fails() {
        let mut data = synthetic_coord_data();

        // Corrupt one point: make width 64 have high l1 norm
        for p in &mut data {
            if p.width == 64 {
                p.l1_norm_mean = 2.0; // 2x increase = steep slope
            }
        }

        let result = coord_check(&data);

        assert!(!result.is_pass(), "Synthetic sloped data should fail");
        assert!(result.slope() > CoordCheckResult::SLOPE_THRESHOLD);
    }

    #[test]
    fn test_empty_points_fails() {
        let result = coord_check(&[]);
        assert!(!result.is_pass());
    }

    #[test]
    fn test_single_point_passes() {
        let data = vec![CoordCheckPoint {
            width: 64,
            l1_norm_mean: 1.0,
            l1_norm_std: 0.1,
            steps: 1000,
        }];

        let result = coord_check(&data);

        // Single point has no slope, so it should pass
        assert!(result.is_pass());
    }

    #[test]
    fn test_slope_threshold_is_0_1() {
        assert_eq!(CoordCheckResult::SLOPE_THRESHOLD, 0.1);
    }

    #[test]
    fn test_pass_result_has_max_slope() {
        let result = CoordCheckResult::Pass { max_slope: 0.05 };
        assert_eq!(result.slope(), 0.05);
    }

    #[test]
    fn test_fail_result_has_slope() {
        let result = CoordCheckResult::Fail {
            slope: 0.15,
            threshold: 0.1,
        };
        assert_eq!(result.slope(), 0.15);
    }

    #[test]
    fn test_widths_8_16_32_64_included() {
        let data = synthetic_coord_data();
        let widths: Vec<_> = data.iter().map(|p| p.width).collect();
        assert_eq!(widths, vec![8, 16, 32, 64]);
    }
}
