//! Rung schedule — φ-anchored ASHA rung progression.
//!
//! Defines the rung structure for ASHA hyperparameter optimization.
//! Each rung represents a checkpoint in the training process.

use super::invariants::{INV2_WARMUP_BLIND_STEPS, PHI, PHI_SQ, PHI_INV_SQ, PHI_INV_FOURTH, INV2_BPB_PRUNE_THRESHOLD};

/// Base step count for rung calculation.
/// Derived from Trinity identity: 3² = φ² + φ⁻²
pub const TRINITY_BASE: u64 = 9;

/// Step unit per rung.
pub const RUNG_UNIT: u64 = 4000;  // INV-2 warmup

/// Number of rungs in ASHA schedule.
pub const RUNG_COUNT: u32 = 9;  // 3² = (φ²+φ⁻²)²

/// Maximum rung exponent (used for step calculation).
pub const MAX_RUNG_EXP: u64 = 3;

/// Single rung in ASHA schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rung {
    /// Rung index (0-indexed)
    pub index: u32,
    /// Number of training steps to reach this rung
    pub steps: u64,
    /// Number of trials to promote to this rung
    pub trials: usize,
}

impl Rung {
    /// Create a new rung.
    pub fn new(index: u32) -> Self {
        // Step calculation: base * unit * φ^(index/3)
        // This creates a φ-anchored exponential schedule
        let phi_factor = PHI.powf((index as f64) / 3.0);
        let steps = (TRINITY_BASE as f64 * RUNG_UNIT as f64 * phi_factor) as u64;

        // Trial promotion: halving at each rung
        let trials = if index == 0 {
            64  // Initial trials
        } else {
            // Successive halving: 64, 32, 16, 8, 4, 2, 1
            64 / (1 << index)
        };

        Self { index, steps, trials }
    }

    /// Check if a trial should be promoted to this rung.
    ///
    /// A trial is promoted if:
    /// - It has reached this rung's step count
    /// - Its BPB is below the prune threshold
    pub fn should_promote(&self, steps_completed: u64, bpb: f64) -> bool {
        steps_completed >= self.steps && bpb < INV2_BPB_PRUNE_THRESHOLD
    }
}

/// Iterator over ASHA rungs.
pub struct RungIter {
    current: u32,
    max: u32,
}

impl RungIter {
    /// Create a new rung iterator.
    pub fn new() -> Self {
        Self {
            current: 0,
            max: RUNG_COUNT,
        }
    }

    /// Create a rung iterator with custom max.
    pub fn with_max(max: u32) -> Self {
        Self { current: 0, max }
    }
}

impl Iterator for RungIter {
    type Item = Rung;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.max {
            let rung = Rung::new(self.current);
            self.current += 1;
            Some(rung)
        } else {
            None
        }
    }
}

impl Default for RungIter {
    fn default() -> Self {
        Self::new()
    }
}

/// Iterate over all rungs.
///
/// Returns an iterator that yields all rungs in order.
pub fn iter_rungs() -> impl Iterator<Item = Rung> {
    RungIter::new()
}

/// Get all rungs as a vector.
pub fn all_rungs() -> Vec<Rung> {
    iter_rungs().collect()
}

/// Check if a rung index is valid.
pub fn check_inv12_rung_valid(index: u32) -> bool {
    index < RUNG_COUNT
}

/// Check if a rung index is valid (usize version).
pub fn check_inv12_rung_valid_usize(index: usize) -> bool {
    (index as u32) < RUNG_COUNT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rung_steps_increase() {
        let rungs: Vec<_> = iter_rungs().collect();
        for window in rungs.windows(2) {
            assert!(window[1].steps > window[0].steps,
                    "Rung steps should increase: {:?}",
                    window);
        }
    }

    #[test]
    fn test_rung_trials_decrease() {
        let rungs: Vec<_> = iter_rungs().collect();
        for window in rungs.windows(2) {
            assert!(window[1].trials <= window[0].trials,
                    "Rung trials should decrease: {:?}",
                    window);
        }
    }

    #[test]
    fn test_rung_count() {
        assert_eq!(all_rungs().len() as u32, RUNG_COUNT);
    }

    #[test]
    fn test_first_rung_steps() {
        let first = Rung::new(0);
        assert_eq!(first.steps, TRINITY_BASE * RUNG_UNIT);
    }

    #[test]
    fn test_check_inv12_rung_valid() {
        assert!(check_inv12_rung_valid(0));
        assert!(check_inv12_rung_valid(RUNG_COUNT - 1));
        assert!(!check_inv12_rung_valid(RUNG_COUNT));
    }
}
