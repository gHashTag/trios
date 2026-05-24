//! # CR-CHAT-05 — Store record size distribution uniformity guard (Wave-123 Lane B)
//!
//! PERSISTENCE — encrypted record sizes at rest must follow a uniform
//! distribution; skewed distributions leak message frequency patterns.
//!
//! When encrypted records are stored, their sizes should fall into
//! uniform size classes. A skewed size distribution:
//!
//! * **Frequency analysis** — the most common size class reveals
//!   the typical message length, even without decryption.
//! * **Traffic fingerprint** — unique size distributions identify
//!   specific users or conversation patterns.
//! * **Statistical test** — chi-squared test on size class
//!   frequencies reveals non-uniform storage patterns.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Size class count must be <= `SRSD_NUM_CLASSES`.
//! 2. Chi-squared statistic <= `SRSD_MAX_CHI_SQUARED`.
//! 3. Record size must be >= `SRSD_MIN_SIZE`.
//! 4. Record size must be <= `SRSD_MAX_SIZE`.
//! 5. No zero-size records.
//! 6. Total records >= `SRSD_MIN_RECORDS` for chi-squared validity.
//!
//! Tests **SRSD-01..10**. Error enum [`SizeDistUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIZE-UNIFORM`

#![forbid(unsafe_code)]

/// Number of size classes.
pub const SRSD_NUM_CLASSES: usize = 8;

/// Maximum chi-squared statistic for uniformity.
pub const SRSD_MAX_CHI_SQUARED: f64 = 20.0;

/// Minimum record size.
pub const SRSD_MIN_SIZE: usize = 128;

/// Maximum record size.
pub const SRSD_MAX_SIZE: usize = 8192;

/// Minimum records for chi-squared validity.
pub const SRSD_MIN_RECORDS: usize = 16;

/// A stored record size observation.
#[derive(Debug, Clone)]
pub struct RecordSizeObservation {
    /// Record size in bytes.
    pub size: usize,
    /// Size class index (0..SRSD_NUM_CLASSES-1).
    pub size_class: usize,
}

/// All ways size distribution uniformity validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum SizeDistUniformityError {
    /// Chi-squared too high (non-uniform).
    NonUniform {
        /// Observed chi-squared statistic.
        chi_squared: f64,
        /// Maximum allowed chi-squared.
        max: f64,
    },
    /// Invalid size class.
    InvalidClass {
        /// Index of the offending record.
        idx: usize,
        /// Size class found.
        got: usize,
        /// Maximum valid class index.
        max: usize,
    },
    /// Size below minimum.
    TooSmall {
        /// Index of the offending record.
        idx: usize,
        /// Size found.
        got: usize,
        /// Minimum allowed size.
        min: usize,
    },
    /// Size above maximum.
    TooLarge {
        /// Index of the offending record.
        idx: usize,
        /// Size found.
        got: usize,
        /// Maximum allowed size.
        max: usize,
    },
    /// Zero size.
    ZeroSize(usize),
    /// Too few records for chi-squared.
    TooFew {
        /// Record count found.
        got: usize,
        /// Minimum records required.
        min: usize,
    },
}

fn chi_squared(observed: &[usize], num_classes: usize) -> f64 {
    let total: usize = observed.iter().sum();
    if total == 0 || num_classes == 0 {
        return 0.0;
    }
    let expected = total as f64 / num_classes as f64;
    if expected == 0.0 {
        return 0.0;
    }
    observed.iter().map(|&o| {
        let diff = o as f64 - expected;
        diff * diff / expected
    }).sum()
}

/// `[VERIFIED]` Validate store record size distribution uniformity.
pub fn validate_size_distribution(
    records: &[RecordSizeObservation],
) -> Result<(), SizeDistUniformityError> {
    if records.len() < SRSD_MIN_RECORDS {
        return Err(SizeDistUniformityError::TooFew {
            got: records.len(),
            min: SRSD_MIN_RECORDS,
        });
    }
    let mut counts = vec![0usize; SRSD_NUM_CLASSES];
    for (i, r) in records.iter().enumerate() {
        if r.size == 0 {
            return Err(SizeDistUniformityError::ZeroSize(i));
        }
        if r.size < SRSD_MIN_SIZE {
            return Err(SizeDistUniformityError::TooSmall {
                idx: i,
                got: r.size,
                min: SRSD_MIN_SIZE,
            });
        }
        if r.size > SRSD_MAX_SIZE {
            return Err(SizeDistUniformityError::TooLarge {
                idx: i,
                got: r.size,
                max: SRSD_MAX_SIZE,
            });
        }
        if r.size_class >= SRSD_NUM_CLASSES {
            return Err(SizeDistUniformityError::InvalidClass {
                idx: i,
                got: r.size_class,
                max: SRSD_NUM_CLASSES - 1,
            });
        }
        counts[r.size_class] += 1;
    }
    let chi = chi_squared(&counts, SRSD_NUM_CLASSES);
    if chi > SRSD_MAX_CHI_SQUARED {
        return Err(SizeDistUniformityError::NonUniform {
            chi_squared: chi,
            max: SRSD_MAX_CHI_SQUARED,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(size: usize, class: usize) -> RecordSizeObservation {
        RecordSizeObservation { size, size_class: class }
    }

    fn uniform_batch() -> Vec<RecordSizeObservation> {
        let mut rs = Vec::new();
        for class in 0..SRSD_NUM_CLASSES {
            for _ in 0..4 {
                rs.push(rec(512 + class * 100, class));
            }
        }
        rs
    }

    /// **SRSD-01** — non-uniform rejected.
    #[test]
    fn srsd_01_non_uniform_rejected() {
        let mut rs = Vec::new();
        for _ in 0..56 {
            rs.push(rec(512, 0));
        }
        for class in 1..SRSD_NUM_CLASSES {
            rs.push(rec(512 + class * 100, class));
        }
        assert!(matches!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::NonUniform { .. })
        ));
    }

    /// **SRSD-02** — invalid class rejected.
    #[test]
    fn srsd_02_invalid_class_rejected() {
        let mut rs = uniform_batch();
        rs.push(rec(512, SRSD_NUM_CLASSES));
        assert_eq!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::InvalidClass {
                idx: rs.len() - 1,
                got: SRSD_NUM_CLASSES,
                max: SRSD_NUM_CLASSES - 1,
            })
        );
    }

    /// **SRSD-03** — too small rejected.
    #[test]
    fn srsd_03_too_small_rejected() {
        let mut rs = uniform_batch();
        rs.push(rec(SRSD_MIN_SIZE - 1, 0));
        assert_eq!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::TooSmall {
                idx: rs.len() - 1,
                got: SRSD_MIN_SIZE - 1,
                min: SRSD_MIN_SIZE,
            })
        );
    }

    /// **SRSD-04** — too large rejected.
    #[test]
    fn srsd_04_too_large_rejected() {
        let mut rs = uniform_batch();
        rs.push(rec(SRSD_MAX_SIZE + 1, 0));
        assert_eq!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::TooLarge {
                idx: rs.len() - 1,
                got: SRSD_MAX_SIZE + 1,
                max: SRSD_MAX_SIZE,
            })
        );
    }

    /// **SRSD-05** — zero size rejected.
    #[test]
    fn srsd_05_zero_size_rejected() {
        let mut rs = uniform_batch();
        rs.push(rec(0, 0));
        assert_eq!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::ZeroSize(rs.len() - 1))
        );
    }

    /// **SRSD-06** — too few records rejected.
    #[test]
    fn srsd_06_too_few_rejected() {
        let rs: Vec<RecordSizeObservation> = (0..SRSD_MIN_RECORDS - 1)
            .map(|i| rec(512, i % SRSD_NUM_CLASSES))
            .collect();
        assert_eq!(
            validate_size_distribution(&rs),
            Err(SizeDistUniformityError::TooFew {
                got: SRSD_MIN_RECORDS - 1,
                min: SRSD_MIN_RECORDS,
            })
        );
    }

    /// **SRSD-07** — uniform accepted.
    #[test]
    fn srsd_07_uniform_accepted() {
        assert_eq!(validate_size_distribution(&uniform_batch()), Ok(()));
    }

    /// **SRSD-08** — exact minimum records accepted.
    #[test]
    fn srsd_08_exact_minimum_accepted() {
        let rs: Vec<RecordSizeObservation> = (0..SRSD_MIN_RECORDS)
            .map(|i| rec(512, i % SRSD_NUM_CLASSES))
            .collect();
        assert_eq!(validate_size_distribution(&rs), Ok(()));
    }

    /// **SRSD-09** — boundary size accepted.
    #[test]
    fn srsd_09_boundary_size_accepted() {
        let mut rs = Vec::new();
        for class in 0..SRSD_NUM_CLASSES {
            for _ in 0..2 {
                rs.push(rec(SRSD_MIN_SIZE, class));
            }
        }
        assert_eq!(validate_size_distribution(&rs), Ok(()));
    }

    /// **SRSD-10** — large uniform batch accepted.
    #[test]
    fn srsd_10_large_batch_accepted() {
        let mut rs = Vec::new();
        for class in 0..SRSD_NUM_CLASSES {
            for _ in 0..20 {
                rs.push(rec(512 + class * 100, class));
            }
        }
        assert_eq!(validate_size_distribution(&rs), Ok(()));
    }
}
