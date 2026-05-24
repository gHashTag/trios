//! # CR-CHAT-04 — Padding class selection entropy guard (Wave-88 Lane B)
//!
//! PADDING — padding class selection distribution must have sufficient
//! entropy, R-CHAT-9.
//!
//! Padding classes are chosen to hide payload sizes. If the selection
//! distribution is biased:
//!
//! * **Payload size inference** — if class A is selected 90% of the
//!   time, an observer can infer that most payloads are in class A's
//!   range, narrowing down possible message types.
//! * **User fingerprinting** — a user's typical payload sizes create
//!   a unique class selection fingerprint across sessions.
//! * **Classification attack** — ML classifiers trained on biased
//!   class distributions can distinguish message types.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each class count >= `PCSE_MIN_PER_CLASS`.
//! 2. No class exceeds `PCSE_MAX_RATIO` of total.
//! 3. Total selections >= `PCSE_MIN_SAMPLES`.
//! 4. Number of classes must match `PCSE_NUM_CLASSES`.
//! 5. Class values must be valid (in the known set).
//! 6. Total selections <= `PCSE_MAX_SAMPLES`.
//!
//! Tests **PCSE-01..10**. Error enum [`ClassEntropyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-CLASS-ENTROPY`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Minimum samples per class.
pub const PCSE_MIN_PER_CLASS: usize = 10;

/// Maximum ratio any single class may occupy.
pub const PCSE_MAX_RATIO_NUM: usize = 3;
pub const PCSE_MAX_RATIO_DEN: usize = 4;

/// Minimum total samples.
pub const PCSE_MIN_SAMPLES: usize = 40;

/// Maximum total samples.
pub const PCSE_MAX_SAMPLES: usize = 1_000_000;

/// Number of padding classes.
pub const PCSE_NUM_CLASSES: usize = 4;

/// Valid class sizes.
pub const PCSE_CLASSES: [usize; PCSE_NUM_CLASSES] = [256, 1024, 4096, 16384];

/// A class selection record.
#[derive(Debug, Clone)]
pub struct ClassSelection {
    /// Class size chosen.
    pub class_size: usize,
}

/// All ways class entropy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClassEntropyError {
    /// Class under minimum count.
    UnderMinimum { class: usize, count: usize, min: usize },
    /// Single class dominates.
    DominantClass { class: usize, ratio_num: usize, ratio_den: usize },
    /// Too few samples.
    TooFewSamples { got: usize, min: usize },
    /// Invalid class size.
    InvalidClass(usize),
    /// Too many samples.
    TooManySamples,
    /// Wrong number of classes observed.
    WrongClassCount { got: usize, expected: usize },
}

/// `[VERIFIED]` Validate padding class selection entropy.
pub fn validate_class_entropy(
    selections: &[ClassSelection],
) -> Result<(), ClassEntropyError> {
    if selections.len() > PCSE_MAX_SAMPLES {
        return Err(ClassEntropyError::TooManySamples);
    }
    if selections.len() < PCSE_MIN_SAMPLES {
        return Err(ClassEntropyError::TooFewSamples {
            got: selections.len(),
            min: PCSE_MIN_SAMPLES,
        });
    }
    let valid: std::collections::HashSet<usize> = PCSE_CLASSES.into_iter().collect();
    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for s in selections {
        if !valid.contains(&s.class_size) {
            return Err(ClassEntropyError::InvalidClass(s.class_size));
        }
        *counts.entry(s.class_size).or_insert(0) += 1;
    }
    if counts.len() != PCSE_NUM_CLASSES {
        return Err(ClassEntropyError::WrongClassCount {
            got: counts.len(),
            expected: PCSE_NUM_CLASSES,
        });
    }
    let total = selections.len();
    for (&class, &count) in &counts {
        if count < PCSE_MIN_PER_CLASS {
            return Err(ClassEntropyError::UnderMinimum {
                class,
                count,
                min: PCSE_MIN_PER_CLASS,
            });
        }
        let ratio = (count * PCSE_MAX_RATIO_DEN + total - 1) / total;
        if ratio > PCSE_MAX_RATIO_NUM {
            return Err(ClassEntropyError::DominantClass {
                class,
                ratio_num: count,
                ratio_den: total,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced_selections() -> Vec<ClassSelection> {
        let mut sel = Vec::new();
        for &c in &PCSE_CLASSES {
            for _ in 0..15 {
                sel.push(ClassSelection { class_size: c });
            }
        }
        sel
    }

    /// **PCSE-01** — class under minimum rejected.
    #[test]
    fn pcse_01_under_minimum_rejected() {
        let mut sel = Vec::new();
        for &c in &PCSE_CLASSES {
            let count = if c == 256 { 3 } else { 15 };
            for _ in 0..count {
                sel.push(ClassSelection { class_size: c });
            }
        }
        assert_eq!(
            validate_class_entropy(&sel),
            Err(ClassEntropyError::UnderMinimum { class: 256, count: 3, min: 10 })
        );
    }

    /// **PCSE-02** — dominant class rejected.
    #[test]
    fn pcse_02_dominant_rejected() {
        let mut sel = Vec::new();
        for _ in 0..100 {
            sel.push(ClassSelection { class_size: 256 });
        }
        for &c in &[1024, 4096, 16384] {
            for _ in 0..10 {
                sel.push(ClassSelection { class_size: c });
            }
        }
        let result = validate_class_entropy(&sel);
        assert!(matches!(result, Err(ClassEntropyError::DominantClass { .. })));
    }

    /// **PCSE-03** — too few samples rejected.
    #[test]
    fn pcse_03_too_few_rejected() {
        let sel: Vec<ClassSelection> = PCSE_CLASSES.iter()
            .map(|&c| ClassSelection { class_size: c })
            .collect();
        assert_eq!(
            validate_class_entropy(&sel),
            Err(ClassEntropyError::TooFewSamples { got: 4, min: 40 })
        );
    }

    /// **PCSE-04** — invalid class rejected.
    #[test]
    fn pcse_04_invalid_class_rejected() {
        let mut sel = balanced_selections();
        sel.push(ClassSelection { class_size: 999 });
        assert_eq!(
            validate_class_entropy(&sel),
            Err(ClassEntropyError::InvalidClass(999))
        );
    }

    /// **PCSE-05** — too many samples rejected.
    #[test]
    fn pcse_05_too_many_rejected() {
        let sel: Vec<ClassSelection> = (0..=PCSE_MAX_SAMPLES)
            .map(|i| ClassSelection { class_size: PCSE_CLASSES[i % PCSE_NUM_CLASSES] })
            .collect();
        assert_eq!(validate_class_entropy(&sel), Err(ClassEntropyError::TooManySamples));
    }

    /// **PCSE-06** — wrong class count rejected.
    #[test]
    fn pcse_06_wrong_class_count_rejected() {
        let mut sel = Vec::new();
        for _ in 0..20 {
            sel.push(ClassSelection { class_size: 256 });
        }
        for _ in 0..20 {
            sel.push(ClassSelection { class_size: 1024 });
        }
        assert!(matches!(
            validate_class_entropy(&sel),
            Err(ClassEntropyError::WrongClassCount { .. })
        ));
    }

    /// **PCSE-07** — balanced selections accepted.
    #[test]
    fn pcse_07_balanced_accepted() {
        assert_eq!(validate_class_entropy(&balanced_selections()), Ok(()));
    }

    /// **PCSE-08** — minimum samples boundary accepted.
    #[test]
    fn pcse_08_min_boundary_accepted() {
        let mut sel = Vec::new();
        for &c in &PCSE_CLASSES {
            for _ in 0..10 {
                sel.push(ClassSelection { class_size: c });
            }
        }
        assert_eq!(validate_class_entropy(&sel), Ok(()));
    }

    /// **PCSE-09** — slightly imbalanced accepted.
    #[test]
    fn pcse_09_slightly_imbalanced_accepted() {
        let mut sel = Vec::new();
        for &c in &PCSE_CLASSES {
            let count = if c == 256 { 20 } else { 12 };
            for _ in 0..count {
                sel.push(ClassSelection { class_size: c });
            }
        }
        assert_eq!(validate_class_entropy(&sel), Ok(()));
    }

    /// **PCSE-10** — all classes equal count accepted.
    #[test]
    fn pcse_10_equal_count_accepted() {
        let sel = balanced_selections();
        assert_eq!(validate_class_entropy(&sel), Ok(()));
    }
}
