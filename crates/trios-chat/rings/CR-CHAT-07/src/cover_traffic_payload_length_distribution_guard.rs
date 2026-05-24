//! # CR-CHAT-07 — Cover traffic payload length distribution guard (Wave-140 Lane B)
//!
//! ANTI-CORRELATION — cover traffic payload lengths must follow the
//! same distribution as real traffic; divergent distributions enable
//! statistical separation.
//!
//! Cover traffic must be indistinguishable from real traffic on the
//! wire. If cover payloads consistently have different length
//! distributions:
//!
//! * **Statistical separation** — an observer can use a chi-squared
//!   test to distinguish cover from real traffic by length.
//! * **Fingerprinting** — specific length patterns identify the
//!   cover traffic generator.
//! * **Cover defeat** — once cover is identified, all remaining
//!   traffic is known to be real.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cover/real length chi-squared <= `CTPL_MAX_CHI_SQUARED`.
//! 2. Minimum observations >= `CTPL_MIN_OBSERVATIONS`.
//! 3. Number of length classes <= `CTPL_MAX_CLASSES`.
//! 4. Number of length classes >= `CTPL_MIN_CLASSES`.
//! 5. No duplicate class IDs.
//! 6. Batch size <= `CTPL_MAX_BATCHES`.
//!
//! Tests **CTPL-01..10**. Error enum [`LengthDistError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LENGTH-INDISTINGUISHABLE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chi-squared statistic.
pub const CTPL_MAX_CHI_SQUARED: f64 = 50.0;

/// Minimum observations per batch.
pub const CTPL_MIN_OBSERVATIONS: usize = 10;

/// Minimum length classes.
pub const CTPL_MIN_CLASSES: usize = 2;

/// Maximum length classes.
pub const CTPL_MAX_CLASSES: usize = 16;

/// Maximum batches.
pub const CTPL_MAX_BATCHES: usize = 128;

/// Class ID length.
pub const CTPL_CLASS_ID_LEN: usize = 8;

/// A length distribution comparison record.
#[derive(Debug, Clone)]
pub struct LengthDistRecord {
    /// Batch identifier.
    pub batch_id: [u8; CTPL_CLASS_ID_LEN],
    /// Number of observations.
    pub observations: usize,
    /// Number of length classes.
    pub num_classes: usize,
    /// Chi-squared statistic comparing cover vs real distribution.
    pub chi_squared: f64,
}

/// All ways length distribution validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum LengthDistError {
    /// Chi-squared too high.
    HighChi { idx: usize, got: f64, max: f64 },
    /// Too few observations.
    TooFewObs { idx: usize, got: usize, min: usize },
    /// Too many classes.
    TooManyClasses { idx: usize, got: usize, max: usize },
    /// Too few classes.
    TooFewClasses { idx: usize, got: usize, min: usize },
    /// Duplicate batch ID.
    DuplicateBatch { idx: usize },
    /// Too many batches.
    TooManyBatches { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic payload length distribution.
pub fn validate_length_distribution(
    batches: &[LengthDistRecord],
) -> Result<(), LengthDistError> {
    if batches.len() > CTPL_MAX_BATCHES {
        return Err(LengthDistError::TooManyBatches {
            got: batches.len(),
            max: CTPL_MAX_BATCHES,
        });
    }
    let mut seen: BTreeSet<[u8; CTPL_CLASS_ID_LEN]> = BTreeSet::new();
    for (i, b) in batches.iter().enumerate() {
        if !seen.insert(b.batch_id) {
            return Err(LengthDistError::DuplicateBatch { idx: i });
        }
        if b.num_classes < CTPL_MIN_CLASSES {
            return Err(LengthDistError::TooFewClasses {
                idx: i,
                got: b.num_classes,
                min: CTPL_MIN_CLASSES,
            });
        }
        if b.num_classes > CTPL_MAX_CLASSES {
            return Err(LengthDistError::TooManyClasses {
                idx: i,
                got: b.num_classes,
                max: CTPL_MAX_CLASSES,
            });
        }
        if b.observations < CTPL_MIN_OBSERVATIONS {
            return Err(LengthDistError::TooFewObs {
                idx: i,
                got: b.observations,
                min: CTPL_MIN_OBSERVATIONS,
            });
        }
        if b.chi_squared > CTPL_MAX_CHI_SQUARED {
            return Err(LengthDistError::HighChi {
                idx: i,
                got: b.chi_squared,
                max: CTPL_MAX_CHI_SQUARED,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; CTPL_CLASS_ID_LEN] {
        [byte; CTPL_CLASS_ID_LEN]
    }

    fn batch(id: u8, obs: usize, classes: usize, chi: f64) -> LengthDistRecord {
        LengthDistRecord { batch_id: bid(id), observations: obs, num_classes: classes, chi_squared: chi }
    }

    fn valid_batches() -> Vec<LengthDistRecord> {
        vec![
            batch(0x01, 100, 4, 15.0),
            batch(0x02, 200, 8, 25.0),
        ]
    }

    /// **CTPL-01** — high chi rejected.
    #[test]
    fn ctpl_01_high_chi_rejected() {
        let b = batch(0x01, 100, 4, CTPL_MAX_CHI_SQUARED + 10.0);
        assert_eq!(
            validate_length_distribution(&[b]),
            Err(LengthDistError::HighChi {
                idx: 0,
                got: CTPL_MAX_CHI_SQUARED + 10.0,
                max: CTPL_MAX_CHI_SQUARED,
            })
        );
    }

    /// **CTPL-02** — too few observations rejected.
    #[test]
    fn ctpl_02_too_few_obs_rejected() {
        let b = batch(0x01, CTPL_MIN_OBSERVATIONS - 1, 4, 15.0);
        assert_eq!(
            validate_length_distribution(&[b]),
            Err(LengthDistError::TooFewObs {
                idx: 0,
                got: CTPL_MIN_OBSERVATIONS - 1,
                min: CTPL_MIN_OBSERVATIONS,
            })
        );
    }

    /// **CTPL-03** — too many classes rejected.
    #[test]
    fn ctpl_03_too_many_classes_rejected() {
        let b = batch(0x01, 100, CTPL_MAX_CLASSES + 1, 15.0);
        assert_eq!(
            validate_length_distribution(&[b]),
            Err(LengthDistError::TooManyClasses {
                idx: 0,
                got: CTPL_MAX_CLASSES + 1,
                max: CTPL_MAX_CLASSES,
            })
        );
    }

    /// **CTPL-04** — too few classes rejected.
    #[test]
    fn ctpl_04_too_few_classes_rejected() {
        let b = batch(0x01, 100, CTPL_MIN_CLASSES - 1, 15.0);
        assert_eq!(
            validate_length_distribution(&[b]),
            Err(LengthDistError::TooFewClasses {
                idx: 0,
                got: CTPL_MIN_CLASSES - 1,
                min: CTPL_MIN_CLASSES,
            })
        );
    }

    /// **CTPL-05** — duplicate batch rejected.
    #[test]
    fn ctpl_05_duplicate_rejected() {
        let bs = vec![
            batch(0x01, 100, 4, 15.0),
            batch(0x01, 200, 8, 25.0),
        ];
        assert_eq!(
            validate_length_distribution(&bs),
            Err(LengthDistError::DuplicateBatch { idx: 1 })
        );
    }

    /// **CTPL-06** — too many batches rejected.
    #[test]
    fn ctpl_06_too_many_rejected() {
        let bs: Vec<LengthDistRecord> = (0..=CTPL_MAX_BATCHES)
            .map(|i| {
                let mut id = [0u8; CTPL_CLASS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                LengthDistRecord { batch_id: id, observations: 100, num_classes: 4, chi_squared: 15.0 }
            })
            .collect();
        assert_eq!(
            validate_length_distribution(&bs),
            Err(LengthDistError::TooManyBatches {
                got: CTPL_MAX_BATCHES + 1,
                max: CTPL_MAX_BATCHES,
            })
        );
    }

    /// **CTPL-07** — valid accepted.
    #[test]
    fn ctpl_07_valid_accepted() {
        assert_eq!(validate_length_distribution(&valid_batches()), Ok(()));
    }

    /// **CTPL-08** — empty accepted.
    #[test]
    fn ctpl_08_empty_accepted() {
        assert_eq!(validate_length_distribution(&[]), Ok(()));
    }

    /// **CTPL-09** — boundary chi accepted.
    #[test]
    fn ctpl_09_boundary_chi_accepted() {
        let b = batch(0x01, 100, 4, CTPL_MAX_CHI_SQUARED);
        assert_eq!(validate_length_distribution(&[b]), Ok(()));
    }

    /// **CTPL-10** — many valid batches accepted.
    #[test]
    fn ctpl_10_many_valid_accepted() {
        let bs: Vec<LengthDistRecord> = (0..20u8)
            .map(|i| batch(i + 1, 50 + (i as usize) * 10, 4, 10.0 + (i as f64)))
            .collect();
        assert_eq!(validate_length_distribution(&bs), Ok(()));
    }
}
