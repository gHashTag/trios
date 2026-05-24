//! # CR-CHAT-07 — Cover traffic emission jitter guard (Wave-89 Lane A)
//!
//! ANTI-CORRELATION — cover traffic must have non-zero jitter, R-CHAT-10.
//!
//! Cover traffic emissions follow a cadence, but without jitter:
//!
//! * **Clock fingerprint** — perfectly periodic emissions are
//!   indistinguishable from a clock signal, allowing an observer to
//!   identify the cover scheduler's cadence and filter cover from real.
//! * **Deterministic elimination** — zero jitter means the observer
//!   can predict exactly when the next emission occurs; any deviation
//!   from the schedule must be a real message.
//! * **Scheduler fingerprinting** — different implementations have
//!   different cadences; zero jitter makes the scheduler identifiable.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Jitter must be >= `CTEJ_MIN_JITTER_MS`.
//! 2. Jitter must be <= `CTEJ_MAX_JITTER_MS`.
//! 3. All jitter values must be non-zero.
//! 4. Emission count <= `CTEJ_MAX_EMISSIONS`.
//! 5. Jitter must not be constant across all emissions.
//! 6. Timestamps must be increasing.
//!
//! Tests **CTEJ-01..10**. Error enum [`JitterError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COVER-JITTER`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum jitter (ms).
pub const CTEJ_MIN_JITTER_MS: u64 = 10;

/// Maximum jitter (ms).
pub const CTEJ_MAX_JITTER_MS: u64 = 5000;

/// Maximum emissions to validate.
pub const CTEJ_MAX_EMISSIONS: usize = 4096;

/// A cover traffic emission record.
#[derive(Debug, Clone)]
pub struct CoverEmission {
    /// Scheduled emission time (ms).
    pub scheduled_ms: u64,
    /// Actual emission time (ms).
    pub actual_ms: u64,
}

impl CoverEmission {
    /// Jitter in ms (absolute difference).
    pub fn jitter_ms(&self) -> u64 {
        if self.actual_ms >= self.scheduled_ms {
            self.actual_ms - self.scheduled_ms
        } else {
            self.scheduled_ms - self.actual_ms
        }
    }
}

/// All ways jitter validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JitterError {
    /// Jitter below minimum.
    JitterTooLow { got: u64, min: u64 },
    /// Jitter above maximum.
    JitterTooHigh { got: u64, max: u64 },
    /// Zero jitter.
    ZeroJitter(usize),
    /// Too many emissions.
    TooManyEmissions,
    /// Constant jitter (all identical).
    ConstantJitter(u64),
    /// Timestamps not increasing.
    TimestampsNotIncreasing,
}

/// `[VERIFIED]` Validate cover traffic emission jitter.
pub fn validate_cover_jitter(
    emissions: &[CoverEmission],
) -> Result<(), JitterError> {
    if emissions.len() > CTEJ_MAX_EMISSIONS {
        return Err(JitterError::TooManyEmissions);
    }
    let mut jitter_values = BTreeSet::new();
    for (i, e) in emissions.iter().enumerate() {
        if i > 0 && e.actual_ms <= emissions[i - 1].actual_ms {
            return Err(JitterError::TimestampsNotIncreasing);
        }
        let j = e.jitter_ms();
        if j == 0 {
            return Err(JitterError::ZeroJitter(i));
        }
        if j < CTEJ_MIN_JITTER_MS {
            return Err(JitterError::JitterTooLow { got: j, min: CTEJ_MIN_JITTER_MS });
        }
        if j > CTEJ_MAX_JITTER_MS {
            return Err(JitterError::JitterTooHigh { got: j, max: CTEJ_MAX_JITTER_MS });
        }
        jitter_values.insert(j);
    }
    if emissions.len() >= 2 && jitter_values.len() == 1 {
        let only = *jitter_values.iter().next().unwrap();
        return Err(JitterError::ConstantJitter(only));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emission(scheduled: u64, actual: u64) -> CoverEmission {
        CoverEmission { scheduled_ms: scheduled, actual_ms: actual }
    }

    fn valid_emissions() -> Vec<CoverEmission> {
        vec![
            emission(1000, 1015),
            emission(2000, 2030),
            emission(3000, 3012),
            emission(4000, 4050),
        ]
    }

    /// **CTEJ-01** — jitter too low rejected.
    #[test]
    fn ctej_01_jitter_too_low_rejected() {
        let es = vec![emission(1000, 1005)];
        assert_eq!(
            validate_cover_jitter(&es),
            Err(JitterError::JitterTooLow { got: 5, min: 10 })
        );
    }

    /// **CTEJ-02** — jitter too high rejected.
    #[test]
    fn ctej_02_jitter_too_high_rejected() {
        let es = vec![emission(1000, 10000)];
        assert_eq!(
            validate_cover_jitter(&es),
            Err(JitterError::JitterTooHigh { got: 9000, max: 5000 })
        );
    }

    /// **CTEJ-03** — zero jitter rejected.
    #[test]
    fn ctej_03_zero_jitter_rejected() {
        let es = vec![emission(1000, 1000)];
        assert_eq!(
            validate_cover_jitter(&es),
            Err(JitterError::ZeroJitter(0))
        );
    }

    /// **CTEJ-04** — too many emissions rejected.
    #[test]
    fn ctej_04_too_many_rejected() {
        let es: Vec<CoverEmission> = (0..=CTEJ_MAX_EMISSIONS as u64)
            .map(|i| emission(i * 1000, i * 1000 + 100 + (i % 50)))
            .collect();
        assert_eq!(validate_cover_jitter(&es), Err(JitterError::TooManyEmissions));
    }

    /// **CTEJ-05** — constant jitter rejected.
    #[test]
    fn ctej_05_constant_jitter_rejected() {
        let es = vec![
            emission(1000, 1020),
            emission(2000, 2020),
            emission(3000, 3020),
        ];
        assert_eq!(
            validate_cover_jitter(&es),
            Err(JitterError::ConstantJitter(20))
        );
    }

    /// **CTEJ-06** — timestamps not increasing rejected.
    #[test]
    fn ctej_06_timestamps_not_increasing_rejected() {
        let es = vec![emission(1000, 1050), emission(2000, 1040)];
        assert_eq!(
            validate_cover_jitter(&es),
            Err(JitterError::TimestampsNotIncreasing)
        );
    }

    /// **CTEJ-07** — valid emissions accepted.
    #[test]
    fn ctej_07_valid_accepted() {
        assert_eq!(validate_cover_jitter(&valid_emissions()), Ok(()));
    }

    /// **CTEJ-08** — empty accepted.
    #[test]
    fn ctej_08_empty_accepted() {
        assert_eq!(validate_cover_jitter(&[]), Ok(()));
    }

    /// **CTEJ-09** — single emission accepted (constant check needs 2+).
    #[test]
    fn ctej_09_single_accepted() {
        assert_eq!(validate_cover_jitter(&[emission(1000, 1020)]), Ok(()));
    }

    /// **CTEJ-10** — negative jitter (actual < scheduled) accepted.
    #[test]
    fn ctej_10_negative_jitter_accepted() {
        let es = vec![
            emission(1100, 1000),
            emission(2100, 2030),
        ];
        assert_eq!(validate_cover_jitter(&es), Ok(()));
    }
}
