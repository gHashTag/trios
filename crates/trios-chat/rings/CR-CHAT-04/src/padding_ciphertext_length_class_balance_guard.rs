//! # CR-CHAT-04 — Padding ciphertext length class balance guard (Wave-131 Lane B)
//!
//! PADDING — ciphertext length classes must be balanced; imbalanced
//! classes reveal which class real messages most often occupy.
//!
//! When messages are padded to discrete length classes, the distribution
//! of ciphertext across classes must be balanced:
//!
//! * **Class frequency analysis** — the most populated class likely
//!   contains real messages, while sparse classes are mostly cover.
//! * **Imbalance fingerprint** — a unique class distribution profile
//!   identifies the user or conversation.
//! * **Statistical test** — chi-squared test on class counts detects
//!   significant imbalance.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chi-squared <= `PCLB_MAX_CHI_SQUARED`.
//! 2. Minimum records >= `PCLB_MIN_RECORDS`.
//! 3. Class index must be < `PCLB_NUM_CLASSES`.
//! 4. No duplicate record IDs.
//! 5. Record size must be > 0.
//! 6. Total records <= `PCLB_MAX_RECORDS`.
//!
//! Tests **PCLB-01..10**. Error enum [`ClassBalanceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CLASS-BALANCED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Number of length classes.
pub const PCLB_NUM_CLASSES: usize = 4;

/// Maximum chi-squared statistic.
pub const PCLB_MAX_CHI_SQUARED: f64 = 15.0;

/// Minimum records for chi-squared validity.
pub const PCLB_MIN_RECORDS: usize = 16;

/// Maximum records per batch.
pub const PCLB_MAX_RECORDS: usize = 4096;

/// Record ID length.
pub const PCLB_RECORD_ID_LEN: usize = 32;

/// A ciphertext length class record.
#[derive(Debug, Clone)]
pub struct ClassRecord {
    /// Record identifier.
    pub record_id: [u8; PCLB_RECORD_ID_LEN],
    /// Length class index.
    pub class_index: usize,
    /// Record size in bytes.
    pub size: usize,
}

/// All ways class balance validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ClassBalanceError {
    /// Chi-squared too high (imbalanced).
    Imbalanced { chi_squared: f64, max: f64 },
    /// Too few records.
    TooFew { got: usize, min: usize },
    /// Invalid class index.
    InvalidClass { idx: usize, got: usize, max: usize },
    /// Duplicate record ID.
    DuplicateRecordId { idx: usize },
    /// Zero size.
    ZeroSize(usize),
    /// Too many records.
    TooMany { got: usize, max: usize },
}

fn chi_squared_flat(observed: &[usize]) -> f64 {
    let total: usize = observed.iter().sum();
    if total == 0 || observed.is_empty() {
        return 0.0;
    }
    let expected = total as f64 / observed.len() as f64;
    if expected == 0.0 {
        return 0.0;
    }
    observed.iter().map(|&o| {
        let diff = o as f64 - expected;
        diff * diff / expected
    }).sum()
}

/// `[VERIFIED]` Validate padding ciphertext length class balance.
pub fn validate_class_balance(
    records: &[ClassRecord],
) -> Result<(), ClassBalanceError> {
    if records.len() > PCLB_MAX_RECORDS {
        return Err(ClassBalanceError::TooMany {
            got: records.len(),
            max: PCLB_MAX_RECORDS,
        });
    }
    if records.len() < PCLB_MIN_RECORDS {
        return Err(ClassBalanceError::TooFew {
            got: records.len(),
            min: PCLB_MIN_RECORDS,
        });
    }
    let mut seen: BTreeSet<[u8; PCLB_RECORD_ID_LEN]> = BTreeSet::new();
    let mut counts = vec![0usize; PCLB_NUM_CLASSES];
    for (i, r) in records.iter().enumerate() {
        if r.size == 0 {
            return Err(ClassBalanceError::ZeroSize(i));
        }
        if r.class_index >= PCLB_NUM_CLASSES {
            return Err(ClassBalanceError::InvalidClass {
                idx: i,
                got: r.class_index,
                max: PCLB_NUM_CLASSES - 1,
            });
        }
        if !seen.insert(r.record_id) {
            return Err(ClassBalanceError::DuplicateRecordId { idx: i });
        }
        counts[r.class_index] += 1;
    }
    let chi = chi_squared_flat(&counts);
    if chi > PCLB_MAX_CHI_SQUARED {
        return Err(ClassBalanceError::Imbalanced {
            chi_squared: chi,
            max: PCLB_MAX_CHI_SQUARED,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; PCLB_RECORD_ID_LEN] {
        [byte; PCLB_RECORD_ID_LEN]
    }

    fn rec(id: u8, class: usize, size: usize) -> ClassRecord {
        ClassRecord { record_id: rid(id), class_index: class, size }
    }

    fn balanced_batch() -> Vec<ClassRecord> {
        let mut rs = Vec::new();
        let mut id = 1u8;
        for class in 0..PCLB_NUM_CLASSES {
            for _ in 0..4 {
                rs.push(ClassRecord {
                    record_id: rid(id),
                    class_index: class,
                    size: 512 + class * 256,
                });
                id = id.wrapping_add(1);
            }
        }
        rs
    }

    /// **PCLB-01** — imbalanced rejected.
    #[test]
    fn pclb_01_imbalanced_rejected() {
        let mut rs = Vec::new();
        let mut id = 1u8;
        for _ in 0..20 {
            rs.push(ClassRecord { record_id: rid(id), class_index: 0, size: 512 });
            id = id.wrapping_add(1);
        }
        assert!(matches!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::Imbalanced { .. })
        ));
    }

    /// **PCLB-02** — too few rejected.
    #[test]
    fn pclb_02_too_few_rejected() {
        let rs: Vec<ClassRecord> = (0..PCLB_MIN_RECORDS - 1)
            .map(|i| ClassRecord {
                record_id: rid((i as u8).wrapping_add(1)),
                class_index: i % PCLB_NUM_CLASSES,
                size: 512,
            })
            .collect();
        assert_eq!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::TooFew {
                got: PCLB_MIN_RECORDS - 1,
                min: PCLB_MIN_RECORDS,
            })
        );
    }

    /// **PCLB-03** — invalid class rejected.
    #[test]
    fn pclb_03_invalid_class_rejected() {
        let mut rs = balanced_batch();
        rs.push(ClassRecord { record_id: rid(0xFF), class_index: PCLB_NUM_CLASSES, size: 512 });
        assert_eq!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::InvalidClass {
                idx: rs.len() - 1,
                got: PCLB_NUM_CLASSES,
                max: PCLB_NUM_CLASSES - 1,
            })
        );
    }

    /// **PCLB-04** — duplicate record ID rejected.
    #[test]
    fn pclb_04_duplicate_rejected() {
        let mut rs = balanced_batch();
        rs.push(ClassRecord { record_id: rid(0x01), class_index: 0, size: 512 });
        assert_eq!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::DuplicateRecordId { idx: rs.len() - 1 })
        );
    }

    /// **PCLB-05** — zero size rejected.
    #[test]
    fn pclb_05_zero_size_rejected() {
        let mut rs = balanced_batch();
        rs.push(ClassRecord { record_id: rid(0xFF), class_index: 0, size: 0 });
        assert_eq!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::ZeroSize(rs.len() - 1))
        );
    }

    /// **PCLB-06** — too many rejected.
    #[test]
    fn pclb_06_too_many_rejected() {
        let rs: Vec<ClassRecord> = (0..=PCLB_MAX_RECORDS)
            .map(|i| {
                let mut id = [0u8; PCLB_RECORD_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                ClassRecord { record_id: id, class_index: i % PCLB_NUM_CLASSES, size: 512 }
            })
            .collect();
        assert_eq!(
            validate_class_balance(&rs),
            Err(ClassBalanceError::TooMany {
                got: PCLB_MAX_RECORDS + 1,
                max: PCLB_MAX_RECORDS,
            })
        );
    }

    /// **PCLB-07** — balanced accepted.
    #[test]
    fn pclb_07_balanced_accepted() {
        assert_eq!(validate_class_balance(&balanced_batch()), Ok(()));
    }

    /// **PCLB-08** — exact minimum accepted.
    #[test]
    fn pclb_08_exact_minimum_accepted() {
        let rs: Vec<ClassRecord> = (0..PCLB_MIN_RECORDS)
            .map(|i| ClassRecord {
                record_id: rid((i as u8).wrapping_add(1)),
                class_index: i % PCLB_NUM_CLASSES,
                size: 512,
            })
            .collect();
        assert_eq!(validate_class_balance(&rs), Ok(()));
    }

    /// **PCLB-09** — slight imbalance accepted.
    #[test]
    fn pclb_09_slight_imbalance_accepted() {
        let mut rs = Vec::new();
        let mut id = 1u8;
        for class in 0..PCLB_NUM_CLASSES {
            let count = if class == 0 { 6 } else { 4 };
            for _ in 0..count {
                rs.push(ClassRecord { record_id: rid(id), class_index: class, size: 512 });
                id = id.wrapping_add(1);
            }
        }
        assert_eq!(validate_class_balance(&rs), Ok(()));
    }

    /// **PCLB-10** — large balanced batch accepted.
    #[test]
    fn pclb_10_large_batch_accepted() {
        let rs: Vec<ClassRecord> = (0..200)
            .map(|i| ClassRecord {
                record_id: rid((i as u8).wrapping_add(1)),
                class_index: i % PCLB_NUM_CLASSES,
                size: 512,
            })
            .collect();
        assert_eq!(validate_class_balance(&rs), Ok(()));
    }
}
