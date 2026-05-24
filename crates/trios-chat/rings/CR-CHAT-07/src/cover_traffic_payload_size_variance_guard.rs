//! # CR-CHAT-07 — Cover traffic payload size variance guard (Wave-121 Lane B)
//!
//! ANTI-CORRELATION — cover traffic payload sizes must have controlled
//! variance; highly variable sizes correlate with real traffic patterns.
//!
//! Cover traffic payloads should fall within a narrow size band. Wide
//! variance in cover payload sizes:
//!
//! * **Size fingerprint** — unique size distributions identify specific
//!   cover traffic generators and distinguish them from real traffic.
//! * **Correlation attack** — if cover sizes vary in sync with real
//!   message sizes, the observer can match cover to real messages.
//! * **Statistical detection** — high coefficient of variation in
//!   cover sizes makes them trivially separable from real traffic.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Size coefficient of variation <= `CPSV_MAX_CV`.
//! 2. Size must be >= `CPSV_MIN_SIZE`.
//! 3. Size must be <= `CPSV_MAX_SIZE`.
//! 4. Timestamp must be > 0.
//! 5. Timestamps must be strictly increasing.
//! 6. Total emissions <= `CPSV_MAX_EMISSIONS`.
//!
//! Tests **CPSV-01..10**. Error enum [`SizeVarianceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIZE-UNIFORM`

#![forbid(unsafe_code)]

/// Maximum coefficient of variation (stddev / mean).
pub const CPSV_MAX_CV: f64 = 0.2;

/// Minimum cover payload size.
pub const CPSV_MIN_SIZE: usize = 256;

/// Maximum cover payload size.
pub const CPSV_MAX_SIZE: usize = 4096;

/// Maximum emissions per batch.
pub const CPSV_MAX_EMISSIONS: usize = 1024;

/// A cover traffic size observation.
#[derive(Debug, Clone)]
pub struct CoverSizeObservation {
    /// Payload size in bytes.
    pub size: usize,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

/// All ways size variance validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum SizeVarianceError {
    /// Coefficient of variation too high.
    HighCV { cv: f64, max: f64 },
    /// Size below minimum.
    TooSmall { idx: usize, got: usize, min: usize },
    /// Size above maximum.
    TooLarge { idx: usize, got: usize, max: usize },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too many emissions.
    TooMany { got: usize, max: usize },
}

fn compute_cv(sizes: &[usize]) -> f64 {
    if sizes.is_empty() {
        return 0.0;
    }
    let mean = sizes.iter().sum::<usize>() as f64 / sizes.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let variance = sizes.iter().map(|&s| (s as f64 - mean).powi(2)).sum::<f64>() / sizes.len() as f64;
    variance.sqrt() / mean
}

/// `[VERIFIED]` Validate cover traffic payload size variance.
pub fn validate_size_variance(
    observations: &[CoverSizeObservation],
) -> Result<(), SizeVarianceError> {
    if observations.len() > CPSV_MAX_EMISSIONS {
        return Err(SizeVarianceError::TooMany {
            got: observations.len(),
            max: CPSV_MAX_EMISSIONS,
        });
    }
    let mut prev_ts: u64 = 0;
    let mut sizes: Vec<usize> = Vec::with_capacity(observations.len());
    for (i, o) in observations.iter().enumerate() {
        if o.timestamp_ms == 0 {
            return Err(SizeVarianceError::ZeroTimestamp(i));
        }
        if i > 0 && o.timestamp_ms <= prev_ts {
            return Err(SizeVarianceError::NonMonotonic {
                idx: i,
                prev: prev_ts,
                current: o.timestamp_ms,
            });
        }
        if o.size < CPSV_MIN_SIZE {
            return Err(SizeVarianceError::TooSmall {
                idx: i,
                got: o.size,
                min: CPSV_MIN_SIZE,
            });
        }
        if o.size > CPSV_MAX_SIZE {
            return Err(SizeVarianceError::TooLarge {
                idx: i,
                got: o.size,
                max: CPSV_MAX_SIZE,
            });
        }
        sizes.push(o.size);
        prev_ts = o.timestamp_ms;
    }
    if observations.len() >= 2 {
        let cv = compute_cv(&sizes);
        if cv > CPSV_MAX_CV {
            return Err(SizeVarianceError::HighCV { cv, max: CPSV_MAX_CV });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(size: usize, ts: u64) -> CoverSizeObservation {
        CoverSizeObservation { size, timestamp_ms: ts }
    }

    fn uniform_batch() -> Vec<CoverSizeObservation> {
        vec![
            obs(1024, 1000),
            obs(1024, 2000),
            obs(1024, 3000),
            obs(1024, 4000),
        ]
    }

    /// **CPSV-01** — high CV rejected.
    #[test]
    fn cpsv_01_high_cv_rejected() {
        let os = vec![
            obs(CPSV_MIN_SIZE, 1000),
            obs(CPSV_MAX_SIZE, 2000),
        ];
        assert!(matches!(
            validate_size_variance(&os),
            Err(SizeVarianceError::HighCV { .. })
        ));
    }

    /// **CPSV-02** — too small rejected.
    #[test]
    fn cpsv_02_too_small_rejected() {
        let os = vec![obs(100, 1000)];
        assert_eq!(
            validate_size_variance(&os),
            Err(SizeVarianceError::TooSmall { idx: 0, got: 100, min: CPSV_MIN_SIZE })
        );
    }

    /// **CPSV-03** — too large rejected.
    #[test]
    fn cpsv_03_too_large_rejected() {
        let os = vec![obs(CPSV_MAX_SIZE + 1, 1000)];
        assert_eq!(
            validate_size_variance(&os),
            Err(SizeVarianceError::TooLarge { idx: 0, got: CPSV_MAX_SIZE + 1, max: CPSV_MAX_SIZE })
        );
    }

    /// **CPSV-04** — zero timestamp rejected.
    #[test]
    fn cpsv_04_zero_timestamp_rejected() {
        let os = vec![obs(1024, 0)];
        assert_eq!(
            validate_size_variance(&os),
            Err(SizeVarianceError::ZeroTimestamp(0))
        );
    }

    /// **CPSV-05** — non-monotonic rejected.
    #[test]
    fn cpsv_05_non_monotonic_rejected() {
        let os = vec![
            obs(1024, 2000),
            obs(1024, 1000),
        ];
        assert_eq!(
            validate_size_variance(&os),
            Err(SizeVarianceError::NonMonotonic { idx: 1, prev: 2000, current: 1000 })
        );
    }

    /// **CPSV-06** — too many rejected.
    #[test]
    fn cpsv_06_too_many_rejected() {
        let os: Vec<CoverSizeObservation> = (0..=CPSV_MAX_EMISSIONS)
            .map(|i| obs(1024, (i as u64) + 1))
            .collect();
        assert_eq!(
            validate_size_variance(&os),
            Err(SizeVarianceError::TooMany {
                got: CPSV_MAX_EMISSIONS + 1,
                max: CPSV_MAX_EMISSIONS,
            })
        );
    }

    /// **CPSV-07** — uniform accepted.
    #[test]
    fn cpsv_07_uniform_accepted() {
        assert_eq!(validate_size_variance(&uniform_batch()), Ok(()));
    }

    /// **CPSV-08** — empty accepted.
    #[test]
    fn cpsv_08_empty_accepted() {
        assert_eq!(validate_size_variance(&[]), Ok(()));
    }

    /// **CPSV-09** — single accepted (no CV check).
    #[test]
    fn cpsv_09_single_accepted() {
        assert_eq!(validate_size_variance(&[obs(1024, 1000)]), Ok(()));
    }

    /// **CPSV-10** — slight variance accepted.
    #[test]
    fn cpsv_10_slight_variance_accepted() {
        let os = vec![
            obs(1000, 1000),
            obs(1020, 2000),
            obs(980, 3000),
            obs(1010, 4000),
        ];
        assert_eq!(validate_size_variance(&os), Ok(()));
    }
}
