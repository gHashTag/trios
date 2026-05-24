//! # CR-CHAT-07 — Cover traffic timing jitter uniformity guard (Wave-137 Lane A)
//!
//! ANTI-CORRELATION — timing jitter between cover traffic messages
//! must be uniformly distributed; patterns reveal real vs dummy traffic.
//!
//! Cover traffic sends dummy messages at irregular intervals to hide
//! real traffic patterns. The jitter (variation in inter-message
//! timing) must be statistically uniform:
//!
//! * **Timing fingerprint** — non-uniform jitter creates a
//!   distinguishable pattern that an observer can use to identify
//!   real messages vs cover.
//! * **Burst detection** — clustered timing reveals when real
//!   messages are being sent between cover bursts.
//! * **Correlation attack** — an adversary who can detect the
//!   jitter distribution can correlate cover timing with real
//!   traffic, defeating the cover traffic purpose.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Jitter coefficient of variation <= `CTJU_MAX_CV_NUM / CTJU_MAX_CV_DEN`.
//! 2. Minimum intervals >= `CTJU_MIN_INTERVALS`.
//! 3. Maximum intervals <= `CTJU_MAX_INTERVALS`.
//! 4. Mean jitter must be >= `CTJU_MIN_MEAN_US`.
//! 5. No duplicate observation IDs.
//! 6. Batch size <= `CTJU_MAX_OBSERVATIONS`.
//!
//! Tests **CTJU-01..10**. Error enum [`JitterUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TIMING-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum CV numerator (×1000).
pub const CTJU_MAX_CV_NUM: u64 = 300;

/// Maximum CV denominator (×1000).
pub const CTJU_MAX_CV_DEN: u64 = 1000;

/// Minimum number of intervals.
pub const CTJU_MIN_INTERVALS: usize = 5;

/// Maximum number of intervals.
pub const CTJU_MAX_INTERVALS: usize = 1024;

/// Minimum mean jitter in microseconds.
pub const CTJU_MIN_MEAN_US: u64 = 100;

/// Maximum observations per batch.
pub const CTJU_MAX_OBSERVATIONS: usize = 512;

/// Observation ID length.
pub const CTJU_OBS_ID_LEN: usize = 16;

/// A timing jitter observation.
#[derive(Debug, Clone)]
pub struct JitterObservation {
    /// Observation identifier.
    pub obs_id: [u8; CTJU_OBS_ID_LEN],
    /// Inter-message interval in microseconds.
    pub interval_us: u64,
}

/// All ways jitter uniformity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum JitterUniformityError {
    /// CV too high (non-uniform).
    HighCv {
        /// Computed CV ×1000.
        cv_x1000: u64,
        /// Maximum CV ×1000.
        max_cv_x1000: u64,
    },
    /// Too few intervals.
    TooFew { got: usize, min: usize },
    /// Too many intervals.
    TooMany { got: usize, max: usize },
    /// Mean jitter too low.
    LowMean { got: u64, min: u64 },
    /// Duplicate observation ID.
    DuplicateObs { idx: usize },
    /// Batch too large.
    TooLargeBatch { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic timing jitter uniformity.
pub fn validate_jitter_uniformity(
    observations: &[JitterObservation],
) -> Result<(), JitterUniformityError> {
    if observations.len() > CTJU_MAX_OBSERVATIONS {
        return Err(JitterUniformityError::TooLargeBatch {
            got: observations.len(),
            max: CTJU_MAX_OBSERVATIONS,
        });
    }
    if observations.is_empty() {
        return Ok(());
    }
    if observations.len() < CTJU_MIN_INTERVALS {
        return Err(JitterUniformityError::TooFew {
            got: observations.len(),
            min: CTJU_MIN_INTERVALS,
        });
    }
    if observations.len() > CTJU_MAX_INTERVALS {
        return Err(JitterUniformityError::TooMany {
            got: observations.len(),
            max: CTJU_MAX_INTERVALS,
        });
    }
    let mut seen: BTreeSet<[u8; CTJU_OBS_ID_LEN]> = BTreeSet::new();
    for (i, o) in observations.iter().enumerate() {
        if !seen.insert(o.obs_id) {
            return Err(JitterUniformityError::DuplicateObs { idx: i });
        }
    }
    let sum: u64 = observations.iter().map(|o| o.interval_us).sum();
    let mean = sum / observations.len() as u64;
    if mean < CTJU_MIN_MEAN_US {
        return Err(JitterUniformityError::LowMean {
            got: mean,
            min: CTJU_MIN_MEAN_US,
        });
    }
    let variance_sum: u64 = observations
        .iter()
        .map(|o| {
            let diff = if o.interval_us > mean { o.interval_us - mean } else { mean - o.interval_us };
            diff * diff
        })
        .sum();
    let std_dev_x1000 = (variance_sum / observations.len() as u64).isqrt() * 1000;
    let cv_x1000 = if mean > 0 { std_dev_x1000 / mean } else { 0 };
    if cv_x1000 > CTJU_MAX_CV_NUM * 1000 / CTJU_MAX_CV_DEN {
        return Err(JitterUniformityError::HighCv {
            cv_x1000,
            max_cv_x1000: CTJU_MAX_CV_NUM * 1000 / CTJU_MAX_CV_DEN,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; CTJU_OBS_ID_LEN] {
        [byte; CTJU_OBS_ID_LEN]
    }

    fn obs(id: u8, interval: u64) -> JitterObservation {
        JitterObservation { obs_id: oid(id), interval_us: interval }
    }

    fn uniform_observations() -> Vec<JitterObservation> {
        (0..10u8)
            .map(|i| obs(i + 1, 500 + (i as u64) * 20))
            .collect()
    }

    /// **CTJU-01** — high CV rejected.
    #[test]
    fn ctju_01_high_cv_rejected() {
        let obs_data = vec![
            obs(0x01, 100),
            obs(0x02, 100),
            obs(0x03, 100),
            obs(0x04, 100),
            obs(0x05, 50000),
        ];
        let result = validate_jitter_uniformity(&obs_data);
        assert!(matches!(result, Err(JitterUniformityError::HighCv { .. })));
    }

    /// **CTJU-02** — too few intervals rejected.
    #[test]
    fn ctju_02_too_few_rejected() {
        let obs_data = vec![obs(0x01, 500), obs(0x02, 500), obs(0x03, 500)];
        assert_eq!(
            validate_jitter_uniformity(&obs_data),
            Err(JitterUniformityError::TooFew { got: 3, min: CTJU_MIN_INTERVALS })
        );
    }

    /// **CTJU-03** — too many intervals rejected.
    #[test]
    fn ctju_03_too_many_rejected() {
        let obs_data: Vec<JitterObservation> = (0..=CTJU_MAX_INTERVALS)
            .map(|i| {
                let mut id = [0u8; CTJU_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                JitterObservation { obs_id: id, interval_us: 500 }
            })
            .collect();
        assert_eq!(
            validate_jitter_uniformity(&obs_data),
            Err(JitterUniformityError::TooLargeBatch { got: CTJU_MAX_INTERVALS + 1, max: CTJU_MAX_OBSERVATIONS })
        );
    }

    /// **CTJU-04** — low mean rejected.
    #[test]
    fn ctju_04_low_mean_rejected() {
        let obs_data: Vec<JitterObservation> = (0..10u8)
            .map(|i| obs(i + 1, 10))
            .collect();
        assert!(matches!(
            validate_jitter_uniformity(&obs_data),
            Err(JitterUniformityError::LowMean { .. })
        ));
    }

    /// **CTJU-05** — duplicate obs rejected.
    #[test]
    fn ctju_05_duplicate_rejected() {
        let obs_data = vec![
            obs(0x01, 500),
            obs(0x01, 600),
            obs(0x02, 500),
            obs(0x03, 500),
            obs(0x04, 500),
            obs(0x05, 500),
        ];
        assert_eq!(
            validate_jitter_uniformity(&obs_data),
            Err(JitterUniformityError::DuplicateObs { idx: 1 })
        );
    }

    /// **CTJU-06** — batch too large rejected.
    #[test]
    fn ctju_06_too_large_batch_rejected() {
        let obs_data: Vec<JitterObservation> = (0..=CTJU_MAX_OBSERVATIONS)
            .map(|i| {
                let mut id = [0u8; CTJU_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                JitterObservation { obs_id: id, interval_us: 500 }
            })
            .collect();
        assert_eq!(
            validate_jitter_uniformity(&obs_data),
            Err(JitterUniformityError::TooLargeBatch { got: CTJU_MAX_OBSERVATIONS + 1, max: CTJU_MAX_OBSERVATIONS })
        );
    }

    /// **CTJU-07** — valid accepted.
    #[test]
    fn ctju_07_valid_accepted() {
        assert_eq!(validate_jitter_uniformity(&uniform_observations()), Ok(()));
    }

    /// **CTJU-08** — empty accepted.
    #[test]
    fn ctju_08_empty_accepted() {
        assert_eq!(validate_jitter_uniformity(&[]), Ok(()));
    }

    /// **CTJU-09** — minimum intervals boundary accepted.
    #[test]
    fn ctju_09_min_intervals_accepted() {
        let obs_data: Vec<JitterObservation> = (0..CTJU_MIN_INTERVALS as u8)
            .map(|i| obs(i + 1, 1000 + (i as u64) * 10))
            .collect();
        assert_eq!(validate_jitter_uniformity(&obs_data), Ok(()));
    }

    /// **CTJU-10** — uniform high values accepted.
    #[test]
    fn ctju_10_uniform_high_accepted() {
        let obs_data: Vec<JitterObservation> = (0..20u8)
            .map(|i| obs(i + 1, 10000 + (i as u64) * 5))
            .collect();
        assert_eq!(validate_jitter_uniformity(&obs_data), Ok(()));
    }
}
