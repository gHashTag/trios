//! # CR-CHAT-07 — Cover traffic volume ratio stability guard (Wave-149 Lane A)
//!
//! ANTI-CORRELATION — the ratio of cover to real traffic must stay
//! stable; sudden ratio changes reveal traffic pattern shifts.
//!
//! Cover traffic is injected to maintain a consistent traffic volume.
//! If the cover-to-real ratio changes suddenly:
//!
//! * **Traffic pattern leak** — a sudden drop in cover traffic
//!   indicates the user stopped sending real messages, and vice
//!   versa.
//! * **Ratio fingerprint** — specific ratio values identify the
//!   cover traffic algorithm parameters.
//! * **Adaptive attack** — an observer who detects ratio changes
//!   can adapt their analysis window to improve detection.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Ratio CV <= `CTVS_MAX_RATIO_CV_NUM / CTVS_MAX_RATIO_CV_DEN`.
//! 2. Minimum observations >= `CTVS_MIN_OBS`.
//! 3. Mean ratio >= `CTVS_MIN_RATIO_NUM / CTVS_MIN_RATIO_DEN`.
//! 4. No duplicate window IDs.
//! 5. Window ID must not be zero.
//! 6. Batch size <= `CTVS_MAX_WINDOWS`.
//!
//! Tests **CTVS-01..10**. Error enum [`RatioStabilityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATIO-STABLE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum ratio CV numerator (×1000).
pub const CTVS_MAX_RATIO_CV_NUM: u64 = 200;

/// Maximum ratio CV denominator (×1000).
pub const CTVS_MAX_RATIO_CV_DEN: u64 = 1000;

/// Minimum observations.
pub const CTVS_MIN_OBS: usize = 5;

/// Minimum mean ratio numerator (×1000).
pub const CTVS_MIN_RATIO_NUM: u64 = 500;

/// Minimum mean ratio denominator (×1000).
pub const CTVS_MIN_RATIO_DEN: u64 = 1000;

/// Maximum windows per batch.
pub const CTVS_MAX_WINDOWS: usize = 256;

/// Window ID length.
pub const CTVS_WINDOW_ID_LEN: usize = 16;

/// A volume ratio observation record.
#[derive(Debug, Clone)]
pub struct RatioObservation {
    /// Window identifier.
    pub window_id: [u8; CTVS_WINDOW_ID_LEN],
    /// Cover-to-real ratio (cover_bytes * 1000 / real_bytes).
    pub ratio_x1000: u64,
}

/// All ways ratio stability validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RatioStabilityError {
    /// CV too high.
    HighCv {
        /// Computed CV ×1000.
        cv_x1000: u64,
        /// Maximum CV ×1000.
        max_cv_x1000: u64,
    },
    /// Too few observations.
    TooFew { got: usize, min: usize },
    /// Mean ratio too low.
    LowMean { got: u64, min: u64 },
    /// Duplicate window ID.
    Duplicate { idx: usize },
    /// Zero window ID.
    ZeroId(usize),
    /// Too many windows.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic volume ratio stability.
pub fn validate_ratio_stability(
    observations: &[RatioObservation],
) -> Result<(), RatioStabilityError> {
    if observations.len() > CTVS_MAX_WINDOWS {
        return Err(RatioStabilityError::TooMany {
            got: observations.len(),
            max: CTVS_MAX_WINDOWS,
        });
    }
    if observations.is_empty() {
        return Ok(());
    }
    if observations.len() < CTVS_MIN_OBS {
        return Err(RatioStabilityError::TooFew {
            got: observations.len(),
            min: CTVS_MIN_OBS,
        });
    }
    let mut seen: BTreeSet<[u8; CTVS_WINDOW_ID_LEN]> = BTreeSet::new();
    for (i, o) in observations.iter().enumerate() {
        if o.window_id == [0u8; CTVS_WINDOW_ID_LEN] {
            return Err(RatioStabilityError::ZeroId(i));
        }
        if !seen.insert(o.window_id) {
            return Err(RatioStabilityError::Duplicate { idx: i });
        }
    }
    let sum: u64 = observations.iter().map(|o| o.ratio_x1000).sum();
    let mean = sum / observations.len() as u64;
    let min_mean = CTVS_MIN_RATIO_NUM * 1000 / CTVS_MIN_RATIO_DEN;
    if mean < min_mean {
        return Err(RatioStabilityError::LowMean { got: mean, min: min_mean });
    }
    let var_sum: u64 = observations
        .iter()
        .map(|o| {
            let diff = if o.ratio_x1000 > mean { o.ratio_x1000 - mean } else { mean - o.ratio_x1000 };
            diff * diff
        })
        .sum();
    let std_dev_x1000 = (var_sum / observations.len() as u64).isqrt() * 1000;
    let cv_x1000 = if mean > 0 { std_dev_x1000 / mean } else { u64::MAX };
    let max_cv_x1000 = CTVS_MAX_RATIO_CV_NUM * 1000 / CTVS_MAX_RATIO_CV_DEN;
    if cv_x1000 > max_cv_x1000 {
        return Err(RatioStabilityError::HighCv { cv_x1000, max_cv_x1000 });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wid(byte: u8) -> [u8; CTVS_WINDOW_ID_LEN] {
        [byte; CTVS_WINDOW_ID_LEN]
    }

    fn obs(id: u8, ratio: u64) -> RatioObservation {
        RatioObservation { window_id: wid(id), ratio_x1000: ratio }
    }

    fn stable_obs() -> Vec<RatioObservation> {
        vec![
            obs(0x01, 800), obs(0x02, 820), obs(0x03, 790),
            obs(0x04, 810), obs(0x05, 800),
        ]
    }

    /// **CTVS-01** — high CV rejected.
    #[test]
    fn ctvs_01_high_cv_rejected() {
        let os = vec![
            obs(0x01, 500), obs(0x02, 500), obs(0x03, 500),
            obs(0x04, 500), obs(0x05, 5000),
        ];
        assert!(matches!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::HighCv { .. })
        ));
    }

    /// **CTVS-02** — too few rejected.
    #[test]
    fn ctvs_02_too_few_rejected() {
        let os = vec![obs(0x01, 800), obs(0x02, 800), obs(0x03, 800)];
        assert_eq!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::TooFew { got: 3, min: CTVS_MIN_OBS })
        );
    }

    /// **CTVS-03** — low mean rejected.
    #[test]
    fn ctvs_03_low_mean_rejected() {
        let os: Vec<RatioObservation> = (0..10u8)
            .map(|i| obs(i + 1, 100))
            .collect();
        assert!(matches!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::LowMean { .. })
        ));
    }

    /// **CTVS-04** — duplicate rejected.
    #[test]
    fn ctvs_04_duplicate_rejected() {
        let os = vec![
            obs(0x01, 800), obs(0x01, 800), obs(0x02, 800),
            obs(0x03, 800), obs(0x04, 800),
        ];
        assert_eq!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::Duplicate { idx: 1 })
        );
    }

    /// **CTVS-05** — zero ID rejected.
    #[test]
    fn ctvs_05_zero_id_rejected() {
        let o = RatioObservation { window_id: [0u8; CTVS_WINDOW_ID_LEN], ratio_x1000: 800 };
        let os: Vec<RatioObservation> = std::iter::once(o)
            .chain((1..CTVS_MIN_OBS).map(|i| obs(i as u8, 800)))
            .collect();
        assert_eq!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::ZeroId(0))
        );
    }

    /// **CTVS-06** — too many rejected.
    #[test]
    fn ctvs_06_too_many_rejected() {
        let os: Vec<RatioObservation> = (0..=CTVS_MAX_WINDOWS)
            .map(|i| {
                let mut id = [0u8; CTVS_WINDOW_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                RatioObservation { window_id: id, ratio_x1000: 800 }
            })
            .collect();
        assert_eq!(
            validate_ratio_stability(&os),
            Err(RatioStabilityError::TooMany { got: CTVS_MAX_WINDOWS + 1, max: CTVS_MAX_WINDOWS })
        );
    }

    /// **CTVS-07** — valid accepted.
    #[test]
    fn ctvs_07_valid_accepted() {
        assert_eq!(validate_ratio_stability(&stable_obs()), Ok(()));
    }

    /// **CTVS-08** — empty accepted.
    #[test]
    fn ctvs_08_empty_accepted() {
        assert_eq!(validate_ratio_stability(&[]), Ok(()));
    }

    /// **CTVS-09** — constant ratio accepted (zero CV).
    #[test]
    fn ctvs_09_constant_accepted() {
        let os: Vec<RatioObservation> = (0..10u8)
            .map(|i| obs(i + 1, 1000))
            .collect();
        assert_eq!(validate_ratio_stability(&os), Ok(()));
    }

    /// **CTVS-10** — many stable windows accepted.
    #[test]
    fn ctvs_10_many_stable_accepted() {
        let os: Vec<RatioObservation> = (0..30u8)
            .map(|i| obs(i + 1, 800 + (i as u64) * 2))
            .collect();
        assert_eq!(validate_ratio_stability(&os), Ok(()));
    }
}
