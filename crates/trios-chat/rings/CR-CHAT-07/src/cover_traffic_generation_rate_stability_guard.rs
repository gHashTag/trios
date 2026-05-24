//! # CR-CHAT-07 — Cover traffic generation rate stability guard (Wave-125 Lane B)
//!
//! ANTI-CORRELATION — cover traffic generation rate must remain stable
//! across epochs; rate fluctuations correlate with user activity.
//!
//! Cover traffic must be generated at a steady rate independent of real
//! user activity. Rate instability:
//!
//! * **Activity correlation** — if cover rate drops when the user is
//!   idle and rises when active, the observer infers presence.
//! * **Epoch fingerprint** — unique rate profiles per epoch identify
//!   the user across epoch boundaries.
//! * **Statistical test** — coefficient of variation in per-epoch
//!   rates reveals instability detectable by a network observer.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Per-epoch rate CV <= `CGRS_MAX_CV`.
//! 2. Rate must be >= `CGRS_MIN_RATE`.
//! 3. Rate must be <= `CGRS_MAX_RATE`.
//! 4. Epoch number must be strictly increasing.
//! 5. Minimum epochs >= `CGRS_MIN_EPOCHS` for CV check.
//! 6. Total epochs <= `CGRS_MAX_EPOCHS`.
//!
//! Tests **CGRS-01..10**. Error enum [`RateStabilityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATE-STABLE`

#![forbid(unsafe_code)]

/// Maximum coefficient of variation in generation rate.
pub const CGRS_MAX_CV: f64 = 0.3;

/// Minimum cover traffic rate (emissions per epoch).
pub const CGRS_MIN_RATE: u64 = 1;

/// Maximum cover traffic rate.
pub const CGRS_MAX_RATE: u64 = 1000;

/// Minimum epochs for CV check.
pub const CGRS_MIN_EPOCHS: usize = 3;

/// Maximum epochs per batch.
pub const CGRS_MAX_EPOCHS: usize = 512;

/// A cover traffic generation rate observation per epoch.
#[derive(Debug, Clone)]
pub struct EpochRateObservation {
    /// Epoch number.
    pub epoch: u64,
    /// Number of cover emissions in this epoch.
    pub rate: u64,
}

/// All ways rate stability validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum RateStabilityError {
    /// Coefficient of variation too high.
    HighCV { cv: f64, max: f64 },
    /// Rate below minimum.
    TooLow { idx: usize, got: u64, min: u64 },
    /// Rate above maximum.
    TooHigh { idx: usize, got: u64, max: u64 },
    /// Non-monotonic epoch.
    NonMonotonicEpoch { idx: usize, prev: u64, current: u64 },
    /// Too few epochs for CV check.
    TooFewEpochs { got: usize, min: usize },
    /// Too many epochs.
    TooMany { got: usize, max: usize },
}

fn compute_cv(values: &[u64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = values.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt() / mean
}

/// `[VERIFIED]` Validate cover traffic generation rate stability.
pub fn validate_rate_stability(
    epochs: &[EpochRateObservation],
) -> Result<(), RateStabilityError> {
    if epochs.len() > CGRS_MAX_EPOCHS {
        return Err(RateStabilityError::TooMany {
            got: epochs.len(),
            max: CGRS_MAX_EPOCHS,
        });
    }
    if epochs.len() < CGRS_MIN_EPOCHS {
        return Err(RateStabilityError::TooFewEpochs {
            got: epochs.len(),
            min: CGRS_MIN_EPOCHS,
        });
    }
    let mut prev_epoch: u64 = 0;
    let mut rates: Vec<u64> = Vec::with_capacity(epochs.len());
    for (i, e) in epochs.iter().enumerate() {
        if e.rate < CGRS_MIN_RATE {
            return Err(RateStabilityError::TooLow {
                idx: i,
                got: e.rate,
                min: CGRS_MIN_RATE,
            });
        }
        if e.rate > CGRS_MAX_RATE {
            return Err(RateStabilityError::TooHigh {
                idx: i,
                got: e.rate,
                max: CGRS_MAX_RATE,
            });
        }
        if i > 0 && e.epoch <= prev_epoch {
            return Err(RateStabilityError::NonMonotonicEpoch {
                idx: i,
                prev: prev_epoch,
                current: e.epoch,
            });
        }
        rates.push(e.rate);
        prev_epoch = e.epoch;
    }
    let cv = compute_cv(&rates);
    if cv > CGRS_MAX_CV {
        return Err(RateStabilityError::HighCV { cv, max: CGRS_MAX_CV });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(epoch: u64, rate: u64) -> EpochRateObservation {
        EpochRateObservation { epoch, rate }
    }

    fn stable_batch() -> Vec<EpochRateObservation> {
        vec![
            obs(1, 10),
            obs(2, 10),
            obs(3, 10),
            obs(4, 10),
            obs(5, 10),
        ]
    }

    /// **CGRS-01** — high CV rejected.
    #[test]
    fn cgrs_01_high_cv_rejected() {
        let es = vec![
            obs(1, 1),
            obs(2, 1000),
            obs(3, 1),
        ];
        assert!(matches!(
            validate_rate_stability(&es),
            Err(RateStabilityError::HighCV { .. })
        ));
    }

    /// **CGRS-02** — rate too low rejected.
    #[test]
    fn cgrs_02_too_low_rejected() {
        let es = vec![
            obs(1, 10),
            obs(2, 0),
            obs(3, 10),
        ];
        assert_eq!(
            validate_rate_stability(&es),
            Err(RateStabilityError::TooLow { idx: 1, got: 0, min: CGRS_MIN_RATE })
        );
    }

    /// **CGRS-03** — rate too high rejected.
    #[test]
    fn cgrs_03_too_high_rejected() {
        let es = vec![
            obs(1, 10),
            obs(2, CGRS_MAX_RATE + 1),
            obs(3, 10),
        ];
        assert_eq!(
            validate_rate_stability(&es),
            Err(RateStabilityError::TooHigh { idx: 1, got: CGRS_MAX_RATE + 1, max: CGRS_MAX_RATE })
        );
    }

    /// **CGRS-04** — non-monotonic epoch rejected.
    #[test]
    fn cgrs_04_non_monotonic_rejected() {
        let es = vec![
            obs(5, 10),
            obs(3, 10),
            obs(4, 10),
        ];
        assert_eq!(
            validate_rate_stability(&es),
            Err(RateStabilityError::NonMonotonicEpoch { idx: 1, prev: 5, current: 3 })
        );
    }

    /// **CGRS-05** — too few epochs rejected.
    #[test]
    fn cgrs_05_too_few_rejected() {
        let es = vec![obs(1, 10), obs(2, 10)];
        assert_eq!(
            validate_rate_stability(&es),
            Err(RateStabilityError::TooFewEpochs { got: 2, min: CGRS_MIN_EPOCHS })
        );
    }

    /// **CGRS-06** — too many rejected.
    #[test]
    fn cgrs_06_too_many_rejected() {
        let es: Vec<EpochRateObservation> = (0..=CGRS_MAX_EPOCHS)
            .map(|i| obs((i as u64) + 1, 10))
            .collect();
        assert_eq!(
            validate_rate_stability(&es),
            Err(RateStabilityError::TooMany {
                got: CGRS_MAX_EPOCHS + 1,
                max: CGRS_MAX_EPOCHS,
            })
        );
    }

    /// **CGRS-07** — stable accepted.
    #[test]
    fn cgrs_07_stable_accepted() {
        assert_eq!(validate_rate_stability(&stable_batch()), Ok(()));
    }

    /// **CGRS-08** — exact minimum epochs accepted.
    #[test]
    fn cgrs_08_exact_minimum_accepted() {
        let es = vec![obs(1, 10), obs(2, 10), obs(3, 10)];
        assert_eq!(validate_rate_stability(&es), Ok(()));
    }

    /// **CGRS-09** — slight variation accepted.
    #[test]
    fn cgrs_09_slight_variation_accepted() {
        let es = vec![
            obs(1, 10),
            obs(2, 11),
            obs(3, 9),
            obs(4, 10),
            obs(5, 10),
        ];
        assert_eq!(validate_rate_stability(&es), Ok(()));
    }

    /// **CGRS-10** — boundary rate accepted.
    #[test]
    fn cgrs_10_boundary_rate_accepted() {
        let es = vec![
            obs(1, CGRS_MIN_RATE),
            obs(2, CGRS_MIN_RATE),
            obs(3, CGRS_MIN_RATE),
        ];
        assert_eq!(validate_rate_stability(&es), Ok(()));
    }
}
