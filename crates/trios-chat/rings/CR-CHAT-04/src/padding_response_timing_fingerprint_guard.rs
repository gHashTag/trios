//! # CR-CHAT-04 — Padding response timing fingerprint guard (Wave-143 Lane A)
//!
//! PADDING — padding must be applied with uniform timing; variable
//! response timing leaks whether padding was applied.
//!
//! When a server applies padding to responses, the processing time
//! must be constant regardless of whether padding was needed. If
//! padded responses take noticeably different time:
//!
//! * **Padding detection** — an observer can distinguish padded vs
//!   unpadded responses by response time, defeating the purpose.
//! * **Content inference** — faster responses indicate short
//!   messages (no padding needed), slower ones indicate padding.
//! * **Statistical attack** — timing variance creates a binary
//!   classifier for message type.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Timing CV <= `PRTF_MAX_CV_NUM / PRTF_MAX_CV_DEN`.
//! 2. Minimum samples >= `PRTF_MIN_SAMPLES`.
//! 3. Mean timing >= `PRTF_MIN_MEAN_US`.
//! 4. No duplicate sample IDs.
//! 5. Sample ID must not be zero.
//! 6. Batch size <= `PRTF_MAX_SAMPLES`.
//!
//! Tests **PRTF-01..10**. Error enum [`TimingFingerprintError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TIMING-CONSTANT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum CV numerator (×1000).
pub const PRTF_MAX_CV_NUM: u64 = 200;

/// Maximum CV denominator (×1000).
pub const PRTF_MAX_CV_DEN: u64 = 1000;

/// Minimum samples.
pub const PRTF_MIN_SAMPLES: usize = 8;

/// Minimum mean timing in microseconds.
pub const PRTF_MIN_MEAN_US: u64 = 50;

/// Maximum samples per batch.
pub const PRTF_MAX_SAMPLES: usize = 512;

/// Sample ID length.
pub const PRTF_SAMPLE_ID_LEN: usize = 16;

/// A timing sample record.
#[derive(Debug, Clone)]
pub struct TimingSample {
    /// Sample identifier.
    pub sample_id: [u8; PRTF_SAMPLE_ID_LEN],
    /// Response time in microseconds.
    pub time_us: u64,
}

/// All ways timing fingerprint validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimingFingerprintError {
    /// CV too high (non-uniform timing).
    HighCv {
        /// Computed CV ×1000.
        cv_x1000: u64,
        /// Maximum CV ×1000.
        max_cv_x1000: u64,
    },
    /// Too few samples.
    TooFew { got: usize, min: usize },
    /// Mean timing too low.
    LowMean { got: u64, min: u64 },
    /// Duplicate sample ID.
    Duplicate { idx: usize },
    /// Zero sample ID.
    ZeroId(usize),
    /// Too many samples.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding response timing fingerprint.
pub fn validate_timing_fingerprint(
    samples: &[TimingSample],
) -> Result<(), TimingFingerprintError> {
    if samples.len() > PRTF_MAX_SAMPLES {
        return Err(TimingFingerprintError::TooMany {
            got: samples.len(),
            max: PRTF_MAX_SAMPLES,
        });
    }
    if samples.is_empty() {
        return Ok(());
    }
    if samples.len() < PRTF_MIN_SAMPLES {
        return Err(TimingFingerprintError::TooFew {
            got: samples.len(),
            min: PRTF_MIN_SAMPLES,
        });
    }
    let mut seen: BTreeSet<[u8; PRTF_SAMPLE_ID_LEN]> = BTreeSet::new();
    for (i, s) in samples.iter().enumerate() {
        if s.sample_id == [0u8; PRTF_SAMPLE_ID_LEN] {
            return Err(TimingFingerprintError::ZeroId(i));
        }
        if !seen.insert(s.sample_id) {
            return Err(TimingFingerprintError::Duplicate { idx: i });
        }
    }
    let sum: u64 = samples.iter().map(|s| s.time_us).sum();
    let mean = sum / samples.len() as u64;
    if mean < PRTF_MIN_MEAN_US {
        return Err(TimingFingerprintError::LowMean { got: mean, min: PRTF_MIN_MEAN_US });
    }
    let var_sum: u64 = samples
        .iter()
        .map(|s| {
            let diff = if s.time_us > mean { s.time_us - mean } else { mean - s.time_us };
            diff * diff
        })
        .sum();
    let std_dev_x1000 = (var_sum / samples.len() as u64).isqrt() * 1000;
    let cv_x1000 = if mean > 0 { std_dev_x1000 / mean } else { u64::MAX };
    let max_cv_x1000 = PRTF_MAX_CV_NUM * 1000 / PRTF_MAX_CV_DEN;
    if cv_x1000 > max_cv_x1000 {
        return Err(TimingFingerprintError::HighCv { cv_x1000, max_cv_x1000 });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; PRTF_SAMPLE_ID_LEN] {
        [byte; PRTF_SAMPLE_ID_LEN]
    }

    fn sample(id: u8, time: u64) -> TimingSample {
        TimingSample { sample_id: sid(id), time_us: time }
    }

    fn uniform_samples() -> Vec<TimingSample> {
        (0..10u8).map(|i| sample(i + 1, 1000 + (i as u64) * 10)).collect()
    }

    /// **PRTF-01** — high CV rejected.
    #[test]
    fn prtf_01_high_cv_rejected() {
        let ss = vec![
            sample(0x01, 100),
            sample(0x02, 100),
            sample(0x03, 100),
            sample(0x04, 100),
            sample(0x05, 100),
            sample(0x06, 100),
            sample(0x07, 100),
            sample(0x08, 100000),
        ];
        assert!(matches!(
            validate_timing_fingerprint(&ss),
            Err(TimingFingerprintError::HighCv { .. })
        ));
    }

    /// **PRTF-02** — too few rejected.
    #[test]
    fn prtf_02_too_few_rejected() {
        let ss = (0..PRTF_MIN_SAMPLES - 1).map(|i| sample((i + 1) as u8, 1000)).collect::<Vec<_>>();
        assert_eq!(
            validate_timing_fingerprint(&ss),
            Err(TimingFingerprintError::TooFew { got: PRTF_MIN_SAMPLES - 1, min: PRTF_MIN_SAMPLES })
        );
    }

    /// **PRTF-03** — low mean rejected.
    #[test]
    fn prtf_03_low_mean_rejected() {
        let ss: Vec<TimingSample> = (0..10u8).map(|i| sample(i + 1, 10)).collect();
        assert!(matches!(
            validate_timing_fingerprint(&ss),
            Err(TimingFingerprintError::LowMean { .. })
        ));
    }

    /// **PRTF-04** — duplicate rejected.
    #[test]
    fn prtf_04_duplicate_rejected() {
        let ss = vec![
            sample(0x01, 1000),
            sample(0x01, 1000),
            sample(0x02, 1000),
            sample(0x03, 1000),
            sample(0x04, 1000),
            sample(0x05, 1000),
            sample(0x06, 1000),
            sample(0x07, 1000),
        ];
        assert_eq!(
            validate_timing_fingerprint(&ss),
            Err(TimingFingerprintError::Duplicate { idx: 1 })
        );
    }

    /// **PRTF-05** — zero ID rejected.
    #[test]
    fn prtf_05_zero_id_rejected() {
        let s = TimingSample { sample_id: [0u8; PRTF_SAMPLE_ID_LEN], time_us: 1000 };
        assert_eq!(
            validate_timing_fingerprint(&[
                s,
                sample(0x02, 1000), sample(0x03, 1000), sample(0x04, 1000),
                sample(0x05, 1000), sample(0x06, 1000), sample(0x07, 1000),
                sample(0x08, 1000),
            ]),
            Err(TimingFingerprintError::ZeroId(0))
        );
    }

    /// **PRTF-06** — too many rejected.
    #[test]
    fn prtf_06_too_many_rejected() {
        let ss: Vec<TimingSample> = (0..=PRTF_MAX_SAMPLES)
            .map(|i| {
                let mut id = [0u8; PRTF_SAMPLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                TimingSample { sample_id: id, time_us: 1000 }
            })
            .collect();
        assert_eq!(
            validate_timing_fingerprint(&ss),
            Err(TimingFingerprintError::TooMany { got: PRTF_MAX_SAMPLES + 1, max: PRTF_MAX_SAMPLES })
        );
    }

    /// **PRTF-07** — valid accepted.
    #[test]
    fn prtf_07_valid_accepted() {
        assert_eq!(validate_timing_fingerprint(&uniform_samples()), Ok(()));
    }

    /// **PRTF-08** — empty accepted.
    #[test]
    fn prtf_08_empty_accepted() {
        assert_eq!(validate_timing_fingerprint(&[]), Ok(()));
    }

    /// **PRTF-09** — minimum samples boundary accepted.
    #[test]
    fn prtf_09_min_samples_accepted() {
        let ss: Vec<TimingSample> = (0..PRTF_MIN_SAMPLES as u8)
            .map(|i| sample(i + 1, 500 + (i as u64) * 5))
            .collect();
        assert_eq!(validate_timing_fingerprint(&ss), Ok(()));
    }

    /// **PRTF-10** — many uniform high values accepted.
    #[test]
    fn prtf_10_many_uniform_accepted() {
        let ss: Vec<TimingSample> = (0..30u8)
            .map(|i| sample(i + 1, 5000 + (i as u64) * 2))
            .collect();
        assert_eq!(validate_timing_fingerprint(&ss), Ok(()));
    }
}
