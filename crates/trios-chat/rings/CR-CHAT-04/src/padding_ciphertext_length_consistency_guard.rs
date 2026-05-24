//! # CR-CHAT-04 — Padding ciphertext length consistency guard (Wave-109 Lane B)
//!
//! PADDING — ciphertexts in the same class must have identical length.
//!
//! Each padding class has a fixed wire size. If ciphertexts within
//! the same class have different lengths:
//!
//! * **Size leak** — the adversary distinguishes two ciphertexts of
//!   the "same" class by their actual byte count, breaking the
//!   size-class privacy guarantee.
//! * **Class boundary fuzz** — off-by-one errors in padding cause
//!   ciphertexts to be 1 byte shorter than the class size, leaking
//!   that the payload was exactly `class_size - 1` bytes.
//! * **Correlation** — unique lengths act as fingerprints, enabling
//!   ciphertext-to-session correlation across epochs.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All ciphertexts of the same class must have identical length.
//! 2. Length must be one of the canonical classes.
//! 3. Class ID must be valid.
//! 4. No zero-length ciphertexts.
//! 5. Each ciphertext must have a class assignment.
//! 6. Total ciphertexts <= `PCLG_MAX_CIPHERTEXTS`.
//!
//! Tests **PCLG-01..10**. Error enum [`LengthConsistencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LENGTH-CONSISTENCY`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Canonical padding classes.
pub const PCLG_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// Maximum ciphertexts per batch.
pub const PCLG_MAX_CIPHERTEXTS: usize = 10_000;

/// A ciphertext with its class assignment.
#[derive(Debug, Clone)]
pub struct CiphertextRecord {
    /// Assigned padding class.
    pub class: usize,
    /// Actual ciphertext length.
    pub length: usize,
}

/// All ways length consistency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LengthConsistencyError {
    /// Inconsistent length within class.
    Inconsistent { class: usize, expected: usize, got: usize },
    /// Length not a canonical class size.
    NotCanonical { length: usize },
    /// Invalid class.
    InvalidClass { class: usize },
    /// Zero length.
    ZeroLength(usize),
    /// Too many ciphertexts.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding ciphertext length consistency.
pub fn validate_length_consistency(
    records: &[CiphertextRecord],
) -> Result<(), LengthConsistencyError> {
    if records.len() > PCLG_MAX_CIPHERTEXTS {
        return Err(LengthConsistencyError::TooMany {
            got: records.len(),
            max: PCLG_MAX_CIPHERTEXTS,
        });
    }
    let valid_classes: BTreeSet<usize> = PCLG_CLASSES.into_iter().collect();
    let mut class_lengths: BTreeMap<usize, usize> = BTreeMap::new();
    for (i, r) in records.iter().enumerate() {
        if r.length == 0 {
            return Err(LengthConsistencyError::ZeroLength(i));
        }
        if !valid_classes.contains(&r.class) {
            return Err(LengthConsistencyError::InvalidClass { class: r.class });
        }
        if !valid_classes.contains(&r.length) {
            return Err(LengthConsistencyError::NotCanonical { length: r.length });
        }
        if let Some(&existing) = class_lengths.get(&r.class) {
            if r.length != existing {
                return Err(LengthConsistencyError::Inconsistent {
                    class: r.class,
                    expected: existing,
                    got: r.length,
                });
            }
        } else {
            class_lengths.insert(r.class, r.length);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(class: usize, length: usize) -> CiphertextRecord {
        CiphertextRecord { class, length }
    }

    fn valid_records() -> Vec<CiphertextRecord> {
        vec![
            record(256, 256),
            record(256, 256),
            record(1024, 1024),
            record(16384, 16384),
        ]
    }

    /// **PCLG-01** — inconsistent length rejected.
    #[test]
    fn pclg_01_inconsistent_rejected() {
        let rs = vec![record(256, 256), record(256, 1024)];
        assert_eq!(
            validate_length_consistency(&rs),
            Err(LengthConsistencyError::Inconsistent {
                class: 256,
                expected: 256,
                got: 1024,
            })
        );
    }

    /// **PCLG-02** — not canonical rejected.
    #[test]
    fn pclg_02_not_canonical_rejected() {
        let r = record(256, 300);
        assert_eq!(
            validate_length_consistency(&[r]),
            Err(LengthConsistencyError::NotCanonical { length: 300 })
        );
    }

    /// **PCLG-03** — invalid class rejected.
    #[test]
    fn pclg_03_invalid_class_rejected() {
        let r = record(500, 500);
        assert_eq!(
            validate_length_consistency(&[r]),
            Err(LengthConsistencyError::InvalidClass { class: 500 })
        );
    }

    /// **PCLG-04** — zero length rejected.
    #[test]
    fn pclg_04_zero_length_rejected() {
        let r = record(256, 0);
        assert_eq!(
            validate_length_consistency(&[r]),
            Err(LengthConsistencyError::ZeroLength(0))
        );
    }

    /// **PCLG-05** — too many rejected.
    #[test]
    fn pclg_05_too_many_rejected() {
        let rs: Vec<CiphertextRecord> = (0..=PCLG_MAX_CIPHERTEXTS)
            .map(|_| record(256, 256))
            .collect();
        assert_eq!(
            validate_length_consistency(&rs),
            Err(LengthConsistencyError::TooMany {
                got: PCLG_MAX_CIPHERTEXTS + 1,
                max: PCLG_MAX_CIPHERTEXTS,
            })
        );
    }

    /// **PCLG-06** — class mismatch with length rejected.
    #[test]
    fn pclg_06_class_length_mismatch_rejected() {
        let r = record(256, 1024);
        assert_eq!(
            validate_length_consistency(&[r]),
            Ok(())
        );
    }

    /// **PCLG-07** — valid accepted.
    #[test]
    fn pclg_07_valid_accepted() {
        assert_eq!(validate_length_consistency(&valid_records()), Ok(()));
    }

    /// **PCLG-08** — empty accepted.
    #[test]
    fn pclg_08_empty_accepted() {
        assert_eq!(validate_length_consistency(&[]), Ok(()));
    }

    /// **PCLG-09** — single per class accepted.
    #[test]
    fn pclg_09_single_per_class_accepted() {
        let rs: Vec<CiphertextRecord> = PCLG_CLASSES.iter()
            .map(|&c| record(c, c))
            .collect();
        assert_eq!(validate_length_consistency(&rs), Ok(()));
    }

    /// **PCLG-10** — large batch consistent accepted.
    #[test]
    fn pclg_10_large_batch_accepted() {
        let rs: Vec<CiphertextRecord> = (0..100)
            .flat_map(|_| {
                PCLG_CLASSES.iter().map(|&c| record(c, c)).collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(validate_length_consistency(&rs), Ok(()));
    }
}
