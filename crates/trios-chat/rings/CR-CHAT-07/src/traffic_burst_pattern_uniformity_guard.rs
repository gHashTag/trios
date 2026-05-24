//! # CR-CHAT-07 — Traffic burst pattern uniformity guard (Wave-85 Lane B)
//!
//! ANTI-CORRELATION — consecutive message bursts must have uniform
//! inter-burst timing, R-CHAT-7.
//!
//! Users send messages in bursts (typing, then sending several messages).
//! If inter-burst timing is not uniform:
//!
//! * **Behavioral fingerprinting** — an observer correlates burst
//!   patterns with a specific user's typing rhythm across sessions.
//! * **Activity inference** — irregular inter-burst gaps reveal when
//!   the user is online vs idle, enabling targeted surveillance.
//! * **De-anonymization** — unique burst timing acts as a fingerprint
//!   across different anonymization layers.
//!
//! This is distinct from BURST (burst size uniformity) and ECAD
//! (emission cadence). TBPU enforces uniform *timing between bursts*.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Inter-burst interval >= `TBPU_MIN_INTERVAL_MS`.
//! 2. Inter-burst interval <= `TBPU_MAX_INTERVAL_MS`.
//! 3. Interval variance <= `TBPU_MAX_VARIANCE_MS`.
//! 4. Burst count <= `TBPU_MAX_BURSTS`.
//! 5. Messages per burst >= 1.
//! 6. Timestamps must be increasing.
//!
//! Tests **TBPU-01..10**. Error enum [`BurstPatternError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BURST-PATTERN-UNIFORM`

#![forbid(unsafe_code)]

/// Minimum inter-burst interval (ms).
pub const TBPU_MIN_INTERVAL_MS: u64 = 500;

/// Maximum inter-burst interval (ms).
pub const TBPU_MAX_INTERVAL_MS: u64 = 300_000;

/// Maximum allowed variance from mean interval (ms).
pub const TBPU_MAX_VARIANCE_MS: u64 = 60_000;

/// Maximum bursts to track.
pub const TBPU_MAX_BURSTS: usize = 1024;

/// A burst of messages at a specific time.
#[derive(Debug, Clone)]
pub struct MessageBurst {
    /// Timestamp of the burst start (ms).
    pub timestamp_ms: u64,
    /// Number of messages in this burst.
    pub message_count: usize,
}

/// All ways burst pattern validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BurstPatternError {
    /// Interval too short.
    IntervalTooShort { got: u64, min: u64 },
    /// Interval too long.
    IntervalTooLong { got: u64, max: u64 },
    /// Variance exceeded.
    VarianceExceeded { variance: u64, max: u64 },
    /// Too many bursts.
    TooManyBursts,
    /// Empty burst (zero messages).
    EmptyBurst(u64),
    /// Timestamps not increasing.
    TimestampNotIncreasing,
}

/// `[VERIFIED]` Validate traffic burst pattern uniformity.
pub fn validate_burst_patterns(bursts: &[MessageBurst]) -> Result<(), BurstPatternError> {
    if bursts.len() > TBPU_MAX_BURSTS {
        return Err(BurstPatternError::TooManyBursts);
    }
    for b in bursts {
        if b.message_count == 0 {
            return Err(BurstPatternError::EmptyBurst(b.timestamp_ms));
        }
    }
    if bursts.len() < 2 {
        return Ok(());
    }
    let mut intervals: Vec<u64> = Vec::new();
    for i in 1..bursts.len() {
        if bursts[i].timestamp_ms <= bursts[i - 1].timestamp_ms {
            return Err(BurstPatternError::TimestampNotIncreasing);
        }
        let interval = bursts[i].timestamp_ms - bursts[i - 1].timestamp_ms;
        if interval < TBPU_MIN_INTERVAL_MS {
            return Err(BurstPatternError::IntervalTooShort {
                got: interval,
                min: TBPU_MIN_INTERVAL_MS,
            });
        }
        if interval > TBPU_MAX_INTERVAL_MS {
            return Err(BurstPatternError::IntervalTooLong {
                got: interval,
                max: TBPU_MAX_INTERVAL_MS,
            });
        }
        intervals.push(interval);
    }
    if !intervals.is_empty() {
        let mean: u64 = intervals.iter().sum::<u64>() / intervals.len() as u64;
        let max_var = intervals
            .iter()
            .map(|&v| if v > mean { v - mean } else { mean - v })
            .max()
            .unwrap_or(0);
        if max_var > TBPU_MAX_VARIANCE_MS {
            return Err(BurstPatternError::VarianceExceeded {
                variance: max_var,
                max: TBPU_MAX_VARIANCE_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst(ts: u64, count: usize) -> MessageBurst {
        MessageBurst { timestamp_ms: ts, message_count: count }
    }

    fn valid_bursts() -> Vec<MessageBurst> {
        vec![burst(10000, 3), burst(30000, 2), burst(50000, 4)]
    }

    /// **TBPU-01** — interval too short rejected.
    #[test]
    fn tbpu_01_interval_too_short_rejected() {
        let bursts = vec![burst(1000, 2), burst(1200, 3)];
        assert_eq!(
            validate_burst_patterns(&bursts),
            Err(BurstPatternError::IntervalTooShort { got: 200, min: 500 })
        );
    }

    /// **TBPU-02** — interval too long rejected.
    #[test]
    fn tbpu_02_interval_too_long_rejected() {
        let bursts = vec![burst(1000, 2), burst(500_000, 3)];
        assert_eq!(
            validate_burst_patterns(&bursts),
            Err(BurstPatternError::IntervalTooLong { got: 499_000, max: 300_000 })
        );
    }

    /// **TBPU-03** — variance exceeded rejected.
    #[test]
    fn tbpu_03_variance_exceeded_rejected() {
        let bursts = vec![
            burst(10000, 2),
            burst(20000, 2),
            burst(200_000, 2),
        ];
        assert_eq!(
            validate_burst_patterns(&bursts),
            Err(BurstPatternError::VarianceExceeded {
                variance: 85000,
                max: 60000,
            })
        );
    }

    /// **TBPU-04** — too many bursts rejected.
    #[test]
    fn tbpu_04_too_many_rejected() {
        let bursts: Vec<MessageBurst> = (0..=TBPU_MAX_BURSTS as u64)
            .map(|i| burst(i * 1000, 1))
            .collect();
        assert_eq!(validate_burst_patterns(&bursts), Err(BurstPatternError::TooManyBursts));
    }

    /// **TBPU-05** — empty burst rejected.
    #[test]
    fn tbpu_05_empty_burst_rejected() {
        let bursts = vec![burst(1000, 0)];
        assert_eq!(
            validate_burst_patterns(&bursts),
            Err(BurstPatternError::EmptyBurst(1000))
        );
    }

    /// **TBPU-06** — timestamps not increasing rejected.
    #[test]
    fn tbpu_06_timestamp_not_increasing_rejected() {
        let bursts = vec![burst(20000, 2), burst(10000, 2)];
        assert_eq!(
            validate_burst_patterns(&bursts),
            Err(BurstPatternError::TimestampNotIncreasing)
        );
    }

    /// **TBPU-07** — valid bursts accepted.
    #[test]
    fn tbpu_07_valid_accepted() {
        assert_eq!(validate_burst_patterns(&valid_bursts()), Ok(()));
    }

    /// **TBPU-08** — empty accepted.
    #[test]
    fn tbpu_08_empty_accepted() {
        assert_eq!(validate_burst_patterns(&[]), Ok(()));
    }

    /// **TBPU-09** — single burst accepted.
    #[test]
    fn tbpu_09_single_accepted() {
        assert_eq!(validate_burst_patterns(&[burst(1000, 5)]), Ok(()));
    }

    /// **TBPU-10** — uniform intervals accepted.
    #[test]
    fn tbpu_10_uniform_accepted() {
        let bursts = vec![
            burst(10000, 2),
            burst(30000, 2),
            burst(50000, 2),
            burst(70000, 2),
        ];
        assert_eq!(validate_burst_patterns(&bursts), Ok(()));
    }
}
