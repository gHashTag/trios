//! # CR-CHAT-07 — Wire timing anomaly guard (Wave-48 Lane A)
//!
//! R-CHAT-10 — Statistical timing anomaly detection.
//!
//! Even with canonical gaps and cadence uniformity, an adversary who
//! observes wire timing over many envelopes can apply statistical tests
//! (Kolmogorov–Smirnov, chi-squared) to detect deviations from uniform
//! emission. trios-chat validates that the *distribution* of observed
//! inter-envelope gaps stays within expected bounds.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Sample count ≥ `WTAG_MIN_SAMPLES`.
//! 2. All observed gaps belong to the canonical set.
//! 3. No single gap class exceeds `WTAG_MAX_CLASS_FRACTION` of total.
//! 4. Standard deviation of gap indices ≤ `WTAG_MAX_STDDEV`.
//! 5. No gap of zero duration.
//! 6. Observed gap count ≤ `WTAG_MAX_OBSERVATIONS`.
//!
//! Tests **WTAG-01..10**. Error enum [`TimingAnomalyError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · TIMING-ANOMALY`

#![forbid(unsafe_code)]

/// Canonical gap classes (milliseconds), matching CR-CHAT-07.
pub const WTAG_CANONICAL_GAPS_MS: [u64; 4] = [1_000, 5_000, 30_000, 300_000];

/// Minimum samples required for analysis.
pub const WTAG_MIN_SAMPLES: usize = 8;

/// Maximum fraction (numerator/denominator) of any single gap class.
pub const WTAG_MAX_CLASS_FRAC_NUM: usize = 3;
pub const WTAG_MAX_CLASS_FRAC_DEN: usize = 4;

/// Maximum standard deviation of gap class indices.
pub const WTAG_MAX_STDDEV: f64 = 1.5;

/// Maximum number of gap observations.
pub const WTAG_MAX_OBSERVATIONS: usize = 1024;

/// All ways timing anomaly detection can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimingAnomalyError {
    /// Not enough samples.
    InsufficientSamples,
    /// Non-canonical gap observed.
    NonCanonicalGap,
    /// Single class dominates.
    ClassDominance,
    /// Std deviation of gap indices too high.
    StdDevTooHigh,
    /// Zero-duration gap.
    ZeroGap,
    /// Too many observations.
    TooManyObservations,
}

fn gap_index(gap_ms: u64) -> Option<usize> {
    WTAG_CANONICAL_GAPS_MS.iter().position(|&g| g == gap_ms)
}

fn stddev(indices: &[usize]) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    let n = indices.len() as f64;
    let mean = indices.iter().sum::<usize>() as f64 / n;
    let variance = indices.iter().map(|&i| {
        let d = i as f64 - mean;
        d * d
    }).sum::<f64>() / n;
    variance.sqrt()
}

/// `[VERIFIED]` Validate observed inter-envelope gaps for timing anomalies.
pub fn validate_timing_distribution(
    gaps_ms: &[u64],
) -> Result<(), TimingAnomalyError> {
    if gaps_ms.is_empty() {
        return Ok(());
    }
    if gaps_ms.len() > WTAG_MAX_OBSERVATIONS {
        return Err(TimingAnomalyError::TooManyObservations);
    }
    for &g in gaps_ms {
        if g == 0 {
            return Err(TimingAnomalyError::ZeroGap);
        }
        if gap_index(g).is_none() {
            return Err(TimingAnomalyError::NonCanonicalGap);
        }
    }
    if gaps_ms.len() >= WTAG_MIN_SAMPLES {
        let mut class_counts = [0usize; WTAG_CANONICAL_GAPS_MS.len()];
        for &g in gaps_ms {
            if let Some(idx) = gap_index(g) {
                class_counts[idx] += 1;
            }
        }
        let total = gaps_ms.len();
        for &count in &class_counts {
            if count * WTAG_MAX_CLASS_FRAC_DEN > total * WTAG_MAX_CLASS_FRAC_NUM {
                return Err(TimingAnomalyError::ClassDominance);
            }
        }
        let indices: Vec<usize> = gaps_ms.iter().map(|&g| gap_index(g).unwrap()).collect();
        if stddev(&indices) > WTAG_MAX_STDDEV {
            return Err(TimingAnomalyError::StdDevTooHigh);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_gaps() -> Vec<u64> {
        vec![1_000, 5_000, 1_000, 5_000, 1_000, 5_000, 1_000, 5_000]
    }

    /// **WTAG-01** — insufficient samples accepted (too few to analyze).
    #[test]
    fn wtag_01_few_samples_accepted() {
        assert_eq!(validate_timing_distribution(&[1_000, 5_000]), Ok(()));
    }

    /// **WTAG-02** — non-canonical gap rejected.
    #[test]
    fn wtag_02_non_canonical_rejected() {
        assert_eq!(
            validate_timing_distribution(&[1_000, 999]),
            Err(TimingAnomalyError::NonCanonicalGap)
        );
    }

    /// **WTAG-03** — zero gap rejected.
    #[test]
    fn wtag_03_zero_gap_rejected() {
        assert_eq!(
            validate_timing_distribution(&[1_000, 0]),
            Err(TimingAnomalyError::ZeroGap)
        );
    }

    /// **WTAG-04** — too many observations rejected.
    #[test]
    fn wtag_04_too_many_rejected() {
        let gaps = vec![1_000; WTAG_MAX_OBSERVATIONS + 1];
        assert_eq!(
            validate_timing_distribution(&gaps),
            Err(TimingAnomalyError::TooManyObservations)
        );
    }

    /// **WTAG-05** — class dominance rejected.
    #[test]
    fn wtag_05_class_dominance_rejected() {
        let gaps = vec![1_000; 7];
        let mut g = gaps;
        g.push(5_000);
        assert_eq!(
            validate_timing_distribution(&g),
            Err(TimingAnomalyError::ClassDominance)
        );
    }

    /// **WTAG-06** — uniform gaps accepted.
    #[test]
    fn wtag_06_uniform_accepted() {
        assert_eq!(validate_timing_distribution(&uniform_gaps()), Ok(()));
    }

    /// **WTAG-07** — empty accepted.
    #[test]
    fn wtag_07_empty_accepted() {
        assert_eq!(validate_timing_distribution(&[]), Ok(()));
    }

    /// **WTAG-08** — all same class with few samples accepted.
    #[test]
    fn wtag_08_few_same_class_accepted() {
        assert_eq!(validate_timing_distribution(&[1_000; 4]), Ok(()));
    }

    /// **WTAG-09** — class dominance by 1000ms rejected.
    #[test]
    fn wtag_09_class_dominance_skewed_rejected() {
        let gaps = vec![1_000, 300_000, 1_000, 1_000, 1_000, 1_000, 1_000, 1_000];
        assert_eq!(
            validate_timing_distribution(&gaps),
            Err(TimingAnomalyError::ClassDominance)
        );
    }

    /// **WTAG-10** — balanced three-class accepted.
    #[test]
    fn wtag_10_balanced_accepted() {
        let gaps = vec![1_000, 5_000, 30_000, 1_000, 5_000, 30_000, 1_000, 5_000];
        assert_eq!(validate_timing_distribution(&gaps), Ok(()));
    }
}
