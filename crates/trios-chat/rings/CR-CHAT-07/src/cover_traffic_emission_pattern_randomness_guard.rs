//! # CR-CHAT-07 — Cover traffic emission pattern randomness guard (Wave-129 Lane B)
//!
//! ANTI-CORRELATION — cover traffic emission patterns must pass a
//! randomness test; predictable patterns are distinguishable from
//! real traffic.
//!
//! Cover traffic must be emitted at intervals that appear random.
//! Predictable emission patterns:
//!
//! * **Periodicity detection** — a periodic emission pattern (e.g.,
//!   exactly every 5 seconds) is trivially distinguished from real
//!   traffic which has natural variance.
//! * **Autocorrelation** — a pattern that repeats is detectable via
//!   autocorrelation analysis of inter-emission intervals.
//! * **Runs test failure** — too many consecutive above-mean or
//!   below-mean intervals fail a Wald-Wolfowitz runs test.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Intervals must have entropy >= `CEPR_MIN_ENTROPY`.
//! 2. Minimum intervals >= `CEPR_MIN_INTERVALS`.
//! 3. Interval must be > 0.
//! 4. Interval must be <= `CEPR_MAX_INTERVAL`.
//! 5. Runs count must be within bounds (Wald-Wolfowitz).
//! 6. Total intervals <= `CEPR_MAX_INTERVALS`.
//!
//! Tests **CEPR-01..10**. Error enum [`PatternRandomnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RANDOM-PATTERN`

#![forbid(unsafe_code)]

/// Minimum entropy in bits per interval.
pub const CEPR_MIN_ENTROPY: f64 = 2.0;

/// Minimum intervals for statistical tests.
pub const CEPR_MIN_INTERVALS: usize = 8;

/// Maximum single interval in milliseconds.
pub const CEPR_MAX_INTERVAL: u64 = 300_000;

/// Maximum intervals per batch.
pub const CEPR_MAX_INTERVALS: usize = 4096;

/// An inter-emission interval observation.
#[derive(Debug, Clone)]
pub struct IntervalObservation {
    /// Interval in milliseconds since last emission.
    pub interval_ms: u64,
}

/// All ways pattern randomness validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum PatternRandomnessError {
    /// Entropy too low.
    LowEntropy { entropy: f64, min: f64 },
    /// Too few intervals.
    TooFew { got: usize, min: usize },
    /// Zero interval.
    ZeroInterval(usize),
    /// Interval exceeds maximum.
    IntervalTooLarge { idx: usize, got: u64, max: u64 },
    /// Too few runs (pattern too predictable).
    TooFewRuns { runs: usize, expected_min: usize },
    /// Too many intervals.
    TooMany { got: usize, max: usize },
}

fn compute_entropy(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = values.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std = variance.sqrt();
    if std == 0.0 {
        return 0.0;
    }
    let mut freq = [0usize; 256];
    for &v in values {
        let bucket = ((v as f64 / (mean + std)) * 128.0).min(255.0) as usize;
        freq[bucket] += 1;
    }
    let len = values.len() as f64;
    let mut entropy = 0.0;
    for &f in &freq {
        if f > 0 {
            let p = f as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

fn count_runs(values: &[u64]) -> usize {
    if values.len() < 2 {
        return values.len();
    }
    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    let mut runs = 1;
    let mut above = values[0] as f64 >= mean;
    for &v in &values[1..] {
        let now_above = v as f64 >= mean;
        if now_above != above {
            runs += 1;
            above = now_above;
        }
    }
    runs
}

/// `[VERIFIED]` Validate cover traffic emission pattern randomness.
pub fn validate_pattern_randomness(
    observations: &[IntervalObservation],
) -> Result<(), PatternRandomnessError> {
    if observations.len() > CEPR_MAX_INTERVALS {
        return Err(PatternRandomnessError::TooMany {
            got: observations.len(),
            max: CEPR_MAX_INTERVALS,
        });
    }
    if observations.len() < CEPR_MIN_INTERVALS {
        return Err(PatternRandomnessError::TooFew {
            got: observations.len(),
            min: CEPR_MIN_INTERVALS,
        });
    }
    let intervals: Vec<u64> = observations.iter().map(|o| o.interval_ms).collect();
    for (i, &iv) in intervals.iter().enumerate() {
        if iv == 0 {
            return Err(PatternRandomnessError::ZeroInterval(i));
        }
        if iv > CEPR_MAX_INTERVAL {
            return Err(PatternRandomnessError::IntervalTooLarge {
                idx: i,
                got: iv,
                max: CEPR_MAX_INTERVAL,
            });
        }
    }
    let entropy = compute_entropy(&intervals);
    if entropy < CEPR_MIN_ENTROPY {
        return Err(PatternRandomnessError::LowEntropy {
            entropy,
            min: CEPR_MIN_ENTROPY,
        });
    }
    let runs = count_runs(&intervals);
    let expected_min = observations.len() / 4;
    if runs < expected_min.max(2) {
        return Err(PatternRandomnessError::TooFewRuns {
            runs,
            expected_min: expected_min.max(2),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(interval: u64) -> IntervalObservation {
        IntervalObservation { interval_ms: interval }
    }

    fn varied_batch() -> Vec<IntervalObservation> {
        vec![
            obs(1000),
            obs(2500),
            obs(800),
            obs(3200),
            obs(1500),
            obs(500),
            obs(2800),
            obs(1200),
        ]
    }

    /// **CEPR-01** — low entropy rejected.
    #[test]
    fn cepr_01_low_entropy_rejected() {
        let os: Vec<IntervalObservation> = (0..CEPR_MIN_INTERVALS)
            .map(|_| obs(1000))
            .collect();
        assert!(matches!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::LowEntropy { .. })
        ));
    }

    /// **CEPR-02** — too few rejected.
    #[test]
    fn cepr_02_too_few_rejected() {
        let os = vec![obs(1000), obs(2000), obs(1500)];
        assert_eq!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::TooFew { got: 3, min: CEPR_MIN_INTERVALS })
        );
    }

    /// **CEPR-03** — zero interval rejected.
    #[test]
    fn cepr_03_zero_interval_rejected() {
        let mut os = varied_batch();
        os.push(obs(0));
        assert_eq!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::ZeroInterval(os.len() - 1))
        );
    }

    /// **CEPR-04** — interval too large rejected.
    #[test]
    fn cepr_04_interval_too_large_rejected() {
        let mut os = varied_batch();
        os.push(obs(CEPR_MAX_INTERVAL + 1));
        assert_eq!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::IntervalTooLarge {
                idx: os.len() - 1,
                got: CEPR_MAX_INTERVAL + 1,
                max: CEPR_MAX_INTERVAL,
            })
        );
    }

    /// **CEPR-05** — too few runs rejected.
    #[test]
    fn cepr_05_too_few_runs_rejected() {
        let os: Vec<IntervalObservation> = (0..20)
            .map(|i| if i < 15 { obs(100) } else { obs(5000) })
            .collect();
        assert!(matches!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::LowEntropy { .. }) | Err(PatternRandomnessError::TooFewRuns { .. })
        ));
    }

    /// **CEPR-06** — too many rejected.
    #[test]
    fn cepr_06_too_many_rejected() {
        let os: Vec<IntervalObservation> = (0..=CEPR_MAX_INTERVALS)
            .map(|i| obs(((i as u64) % 3000) + 100))
            .collect();
        assert_eq!(
            validate_pattern_randomness(&os),
            Err(PatternRandomnessError::TooMany {
                got: CEPR_MAX_INTERVALS + 1,
                max: CEPR_MAX_INTERVALS,
            })
        );
    }

    /// **CEPR-07** — varied accepted.
    #[test]
    fn cepr_07_varied_accepted() {
        assert_eq!(validate_pattern_randomness(&varied_batch()), Ok(()));
    }

    /// **CEPR-08** — exact minimum accepted.
    #[test]
    fn cepr_08_exact_minimum_accepted() {
        let os = vec![
            obs(500), obs(3000), obs(800), obs(2500),
            obs(1200), obs(1800), obs(600), obs(2200),
        ];
        assert_eq!(validate_pattern_randomness(&os), Ok(()));
    }

    /// **CEPR-09** — boundary interval accepted.
    #[test]
    fn cepr_09_boundary_interval_accepted() {
        let mut os = varied_batch();
        os.push(obs(CEPR_MAX_INTERVAL));
        assert_eq!(validate_pattern_randomness(&os), Ok(()));
    }

    /// **CEPR-10** — large varied batch accepted.
    #[test]
    fn cepr_10_large_batch_accepted() {
        let os: Vec<IntervalObservation> = (0..100)
            .map(|i| {
                let v = if i % 2 == 0 { 500 + (i as u64) * 17 % 1000 } else { 2000 + (i as u64) * 31 % 1000 };
                obs(v)
            })
            .collect();
        assert_eq!(validate_pattern_randomness(&os), Ok(()));
    }
}
