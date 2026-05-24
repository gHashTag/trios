//! # CR-CHAT-04 — Padding timing uniformity guard (Wave-151 Lane B)
//!
//! PADDING — padded message timing must be statistically uniform;
//! patterns leak message sizes and content.
//!
//! When cover traffic padding is applied, the timing of padded
//! messages must not reveal which are real vs decoy. If timing
//! patterns emerge:
//!
//! * **Size correlation** — non-uniform timing between padded
//!   blocks correlates with real message injection points.
//! * **Traffic analysis** — an observer can distinguish real
//!   messages from padding based on inter-arrival variance.
//! * **Fingerprinting** — consistent timing patterns create
//!   a fingerprint for the user's communication patterns.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Coefficient of variation of inter-arrival times <= `PTU_MAX_CV`.
//! 2. Mean inter-arrival >= `PTU_MIN_MEAN_US`.
//! 3. No duplicate observation IDs.
//! 4. Observation ID must not be zero.
//! 5. At least `PTU_MIN_OBS` observations.
//! 6. Batch size <= `PTU_MAX_OBS`.
//!
//! Tests **PTU-01..10**. Error enum [`TimingUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TIMING-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum observations per batch.
pub const PTU_MAX_OBS: usize = 4096;

/// Minimum observations required.
pub const PTU_MIN_OBS: usize = 8;

/// Maximum coefficient of variation (scaled by 10000).
pub const PTU_MAX_CV: u64 = 1500;

/// Minimum mean inter-arrival time in microseconds.
pub const PTU_MIN_MEAN_US: u64 = 100;

/// Observation ID length.
pub const PTU_OBS_ID_LEN: usize = 16;

/// A timing observation.
#[derive(Debug, Clone)]
pub struct TimingObservation {
    /// Observation identifier.
    pub obs_id: [u8; PTU_OBS_ID_LEN],
    /// Inter-arrival time in microseconds.
    pub interval_us: u64,
}

/// All ways timing uniformity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimingUniformityError {
    /// CV exceeds maximum.
    HighCv {
        /// Computed CV (scaled by 10000).
        cv: u64,
        /// Maximum allowed.
        max: u64,
    },
    /// Mean too low.
    LowMean {
        /// Computed mean.
        mean: u64,
        /// Minimum required.
        min: u64,
    },
    /// Duplicate observation ID.
    DuplicateId {
        /// Index.
        idx: usize,
    },
    /// Zero observation ID.
    ZeroId(usize),
    /// Too few observations.
    TooFew {
        got: usize,
        min: usize,
    },
    /// Too many observations.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate padding timing uniformity.
pub fn validate_timing_uniformity(
    obs: &[TimingObservation],
) -> Result<(), TimingUniformityError> {
    if obs.len() > PTU_MAX_OBS {
        return Err(TimingUniformityError::TooMany {
            got: obs.len(),
            max: PTU_MAX_OBS,
        });
    }
    if obs.len() < PTU_MIN_OBS {
        return Err(TimingUniformityError::TooFew {
            got: obs.len(),
            min: PTU_MIN_OBS,
        });
    }
    let mut seen: BTreeSet<[u8; PTU_OBS_ID_LEN]> = BTreeSet::new();
    let mut sum: u64 = 0;
    for (i, o) in obs.iter().enumerate() {
        if o.obs_id == [0u8; PTU_OBS_ID_LEN] {
            return Err(TimingUniformityError::ZeroId(i));
        }
        if !seen.insert(o.obs_id) {
            return Err(TimingUniformityError::DuplicateId { idx: i });
        }
        sum += o.interval_us;
    }
    let n = obs.len() as u64;
    let mean = sum / n;
    if mean < PTU_MIN_MEAN_US {
        return Err(TimingUniformityError::LowMean { mean, min: PTU_MIN_MEAN_US });
    }
    let variance_sum: u128 = obs.iter().map(|o| {
        let diff = if o.interval_us > mean { (o.interval_us - mean) as u128 } else { (mean - o.interval_us) as u128 };
        diff * diff
    }).sum();
    let variance = (variance_sum / n as u128) as u64;
    let std_dev = approx_sqrt(variance);
    let cv = if mean > 0 { (std_dev as u128 * 10000 / mean as u128) as u64 } else { PTU_MAX_CV + 1 };
    if cv > PTU_MAX_CV {
        return Err(TimingUniformityError::HighCv { cv, max: PTU_MAX_CV });
    }
    Ok(())
}

fn approx_sqrt(n: u64) -> u64 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; PTU_OBS_ID_LEN] {
        [byte; PTU_OBS_ID_LEN]
    }

    fn obs(id: u8, interval_us: u64) -> TimingObservation {
        TimingObservation { obs_id: oid(id), interval_us }
    }

    fn uniform_obs() -> Vec<TimingObservation> {
        (0..10u8)
            .map(|i| obs(i + 1, 1000 + i as u64 * 10))
            .collect()
    }

    /// **PTU-01** — high CV rejected.
    #[test]
    fn ptu_01_high_cv_rejected() {
        let os: Vec<TimingObservation> = (0..10u8)
            .map(|i| {
                let interval = if i < 5 { 100 } else { 10000000 };
                obs(i + 1, interval)
            })
            .collect();
        let r = validate_timing_uniformity(&os);
        assert!(matches!(r, Err(TimingUniformityError::HighCv { .. })));
    }

    /// **PTU-02** — low mean rejected.
    #[test]
    fn ptu_02_low_mean_rejected() {
        let os: Vec<TimingObservation> = (0..10u8)
            .map(|i| obs(i + 1, 10))
            .collect();
        assert_eq!(
            validate_timing_uniformity(&os),
            Err(TimingUniformityError::LowMean { mean: 10, min: PTU_MIN_MEAN_US })
        );
    }

    /// **PTU-03** — duplicate ID rejected.
    #[test]
    fn ptu_03_duplicate_rejected() {
        let mut os = uniform_obs();
        os.push(obs(1, 1000));
        assert_eq!(
            validate_timing_uniformity(&os),
            Err(TimingUniformityError::DuplicateId { idx: 10 })
        );
    }

    /// **PTU-04** — zero ID rejected.
    #[test]
    fn ptu_04_zero_id_rejected() {
        let mut os = uniform_obs();
        os[0].obs_id = [0u8; PTU_OBS_ID_LEN];
        assert_eq!(
            validate_timing_uniformity(&os),
            Err(TimingUniformityError::ZeroId(0))
        );
    }

    /// **PTU-05** — too few rejected.
    #[test]
    fn ptu_05_too_few_rejected() {
        let os: Vec<TimingObservation> = (0..3u8)
            .map(|i| obs(i + 1, 1000))
            .collect();
        assert_eq!(
            validate_timing_uniformity(&os),
            Err(TimingUniformityError::TooFew { got: 3, min: PTU_MIN_OBS })
        );
    }

    /// **PTU-06** — too many rejected.
    #[test]
    fn ptu_06_too_many_rejected() {
        let os: Vec<TimingObservation> = (0..=PTU_MAX_OBS)
            .map(|i| {
                let mut id = [0u8; PTU_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                TimingObservation { obs_id: id, interval_us: 1000 }
            })
            .collect();
        assert_eq!(
            validate_timing_uniformity(&os),
            Err(TimingUniformityError::TooMany {
                got: PTU_MAX_OBS + 1,
                max: PTU_MAX_OBS,
            })
        );
    }

    /// **PTU-07** — valid accepted.
    #[test]
    fn ptu_07_valid_accepted() {
        assert_eq!(validate_timing_uniformity(&uniform_obs()), Ok(()));
    }

    /// **PTU-08** — empty rejected (too few).
    #[test]
    fn ptu_08_empty_rejected() {
        assert_eq!(
            validate_timing_uniformity(&[]),
            Err(TimingUniformityError::TooFew { got: 0, min: PTU_MIN_OBS })
        );
    }

    /// **PTU-09** — exact minimum count accepted.
    #[test]
    fn ptu_09_exact_min_accepted() {
        let os: Vec<TimingObservation> = (0..PTU_MIN_OBS as u8)
            .map(|i| obs(i + 1, 1000))
            .collect();
        assert_eq!(validate_timing_uniformity(&os), Ok(()));
    }

    /// **PTU-10** — many uniform accepted.
    #[test]
    fn ptu_10_many_uniform_accepted() {
        let os: Vec<TimingObservation> = (0..100u8)
            .map(|i| {
                let mut id = [0u8; PTU_OBS_ID_LEN];
                id[0] = i + 1;
                TimingObservation { obs_id: id, interval_us: 5000 + i as u64 }
            })
            .collect();
        assert_eq!(validate_timing_uniformity(&os), Ok(()));
    }
}
