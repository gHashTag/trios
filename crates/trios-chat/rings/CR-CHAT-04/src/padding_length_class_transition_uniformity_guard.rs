//! # CR-CHAT-04 — Padding length class transition uniformity guard (Wave-128 Lane A)
//!
//! PADDING — transitions between padding length classes must be uniform;
//! biased transitions leak which class the real message falls into.
//!
//! When messages are padded to size classes, the sequence of class
//! transitions must be statistically uniform:
//!
//! * **Transition fingerprint** — if transitions to a specific class
//!   are more frequent, the observer learns that class contains real
//!   messages more often.
//! * **Markov analysis** — an observer building a transition matrix
//!   can predict the next class, enabling real-time classification.
//! * **Class bias** — non-uniform transitions reveal which classes
//!   are "preferred" by the padding algorithm.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chi-squared on transition counts <= `PLCT_MAX_CHI_SQUARED`.
//! 2. Minimum transitions >= `PLCT_MIN_TRANSITIONS`.
//! 3. Class index must be < `PLCT_NUM_CLASSES`.
//! 4. No consecutive duplicate classes (real traffic mixes classes).
//! 5. Timestamps must be strictly increasing.
//! 6. Total transitions <= `PLCT_MAX_TRANSITIONS`.
//!
//! Tests **PLCT-01..10**. Error enum [`TransitionUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TRANSITION-UNIFORM`

#![forbid(unsafe_code)]

/// Number of padding length classes.
pub const PLCT_NUM_CLASSES: usize = 4;

/// Maximum chi-squared statistic.
pub const PLCT_MAX_CHI_SQUARED: f64 = 15.0;

/// Minimum transitions for chi-squared validity.
pub const PLCT_MIN_TRANSITIONS: usize = 8;

/// Maximum transitions per batch.
pub const PLCT_MAX_TRANSITIONS: usize = 4096;

/// A padding length class transition.
#[derive(Debug, Clone)]
pub struct TransitionObservation {
    /// Source class index.
    pub from_class: usize,
    /// Destination class index.
    pub to_class: usize,
    /// Timestamp of the transition.
    pub timestamp_ms: u64,
}

/// All ways transition uniformity validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum TransitionUniformityError {
    /// Chi-squared too high (non-uniform transitions).
    NonUniform { chi_squared: f64, max: f64 },
    /// Too few transitions.
    TooFew { got: usize, min: usize },
    /// Invalid class index.
    InvalidClass { idx: usize, class: usize, max: usize },
    /// Consecutive duplicate classes.
    DuplicateConsecutive { idx: usize, class: usize },
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too many transitions.
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

/// `[VERIFIED]` Validate padding length class transition uniformity.
pub fn validate_transition_uniformity(
    transitions: &[TransitionObservation],
) -> Result<(), TransitionUniformityError> {
    if transitions.len() > PLCT_MAX_TRANSITIONS {
        return Err(TransitionUniformityError::TooMany {
            got: transitions.len(),
            max: PLCT_MAX_TRANSITIONS,
        });
    }
    if transitions.len() < PLCT_MIN_TRANSITIONS {
        return Err(TransitionUniformityError::TooFew {
            got: transitions.len(),
            min: PLCT_MIN_TRANSITIONS,
        });
    }
    let mut prev_ts: u64 = 0;
    let mut transition_counts: Vec<usize> = vec![0; PLCT_NUM_CLASSES];
    for (i, t) in transitions.iter().enumerate() {
        if t.from_class >= PLCT_NUM_CLASSES {
            return Err(TransitionUniformityError::InvalidClass {
                idx: i,
                class: t.from_class,
                max: PLCT_NUM_CLASSES - 1,
            });
        }
        if t.to_class >= PLCT_NUM_CLASSES {
            return Err(TransitionUniformityError::InvalidClass {
                idx: i,
                class: t.to_class,
                max: PLCT_NUM_CLASSES - 1,
            });
        }
        if i > 0 && t.timestamp_ms <= prev_ts {
            return Err(TransitionUniformityError::NonMonotonic {
                idx: i,
                prev: prev_ts,
                current: t.timestamp_ms,
            });
        }
        if t.from_class == t.to_class {
            return Err(TransitionUniformityError::DuplicateConsecutive {
                idx: i,
                class: t.from_class,
            });
        }
        transition_counts[t.to_class] += 1;
        prev_ts = t.timestamp_ms;
    }
    let chi = chi_squared_flat(&transition_counts);
    if chi > PLCT_MAX_CHI_SQUARED {
        return Err(TransitionUniformityError::NonUniform {
            chi_squared: chi,
            max: PLCT_MAX_CHI_SQUARED,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trans(from: usize, to: usize, ts: u64) -> TransitionObservation {
        TransitionObservation { from_class: from, to_class: to, timestamp_ms: ts }
    }

    fn uniform_batch() -> Vec<TransitionObservation> {
        let mut ts = Vec::new();
        let mut t = 1u64;
        for round in 0..4 {
            for class in 0..PLCT_NUM_CLASSES {
                let next = (class + 1) % PLCT_NUM_CLASSES;
                ts.push(trans(class, next, t));
                t += 1;
            }
        }
        ts
    }

    /// **PLCT-01** — non-uniform rejected.
    #[test]
    fn plct_01_non_uniform_rejected() {
        let mut ts = Vec::new();
        for i in 0..PLCT_MIN_TRANSITIONS {
            ts.push(trans(1, 0, (i as u64) + 1));
        }
        assert!(matches!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::NonUniform { .. })
        ));
    }

    /// **PLCT-02** — too few rejected.
    #[test]
    fn plct_02_too_few_rejected() {
        let ts: Vec<TransitionObservation> = (0..PLCT_MIN_TRANSITIONS - 1)
            .map(|i| trans(0, 1, (i as u64) + 1))
            .collect();
        assert_eq!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::TooFew {
                got: PLCT_MIN_TRANSITIONS - 1,
                min: PLCT_MIN_TRANSITIONS,
            })
        );
    }

    /// **PLCT-03** — invalid class rejected.
    #[test]
    fn plct_03_invalid_class_rejected() {
        let mut ts = uniform_batch();
        ts.push(trans(0, PLCT_NUM_CLASSES, 9999));
        assert_eq!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::InvalidClass {
                idx: ts.len() - 1,
                class: PLCT_NUM_CLASSES,
                max: PLCT_NUM_CLASSES - 1,
            })
        );
    }

    /// **PLCT-04** — duplicate consecutive rejected.
    #[test]
    fn plct_04_duplicate_consecutive_rejected() {
        let mut ts = uniform_batch();
        ts.push(trans(0, 0, 9999));
        assert_eq!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::DuplicateConsecutive { idx: ts.len() - 1, class: 0 })
        );
    }

    /// **PLCT-05** — non-monotonic rejected.
    #[test]
    fn plct_05_non_monotonic_rejected() {
        let mut ts = uniform_batch();
        ts.push(trans(0, 1, 1));
        assert!(matches!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::NonMonotonic { .. })
        ));
    }

    /// **PLCT-06** — too many rejected.
    #[test]
    fn plct_06_too_many_rejected() {
        let ts: Vec<TransitionObservation> = (0..=PLCT_MAX_TRANSITIONS)
            .map(|i| {
                let from = i % PLCT_NUM_CLASSES;
                let to = (from + 1) % PLCT_NUM_CLASSES;
                trans(from, to, (i as u64) + 1)
            })
            .collect();
        assert_eq!(
            validate_transition_uniformity(&ts),
            Err(TransitionUniformityError::TooMany {
                got: PLCT_MAX_TRANSITIONS + 1,
                max: PLCT_MAX_TRANSITIONS,
            })
        );
    }

    /// **PLCT-07** — uniform accepted.
    #[test]
    fn plct_07_uniform_accepted() {
        assert_eq!(validate_transition_uniformity(&uniform_batch()), Ok(()));
    }

    /// **PLCT-08** — exact minimum accepted.
    #[test]
    fn plct_08_exact_minimum_accepted() {
        let ts: Vec<TransitionObservation> = (0..PLCT_MIN_TRANSITIONS)
            .map(|i| {
                let from = i % PLCT_NUM_CLASSES;
                let to = (from + 1) % PLCT_NUM_CLASSES;
                trans(from, to, (i as u64) + 1)
            })
            .collect();
        assert_eq!(validate_transition_uniformity(&ts), Ok(()));
    }

    /// **PLCT-09** — mixed transitions accepted.
    #[test]
    fn plct_09_mixed_accepted() {
        let ts = vec![
            trans(0, 1, 1),
            trans(1, 2, 2),
            trans(2, 3, 3),
            trans(3, 0, 4),
            trans(0, 2, 5),
            trans(2, 1, 6),
            trans(1, 3, 7),
            trans(3, 0, 8),
        ];
        assert_eq!(validate_transition_uniformity(&ts), Ok(()));
    }

    /// **PLCT-10** — large uniform batch accepted.
    #[test]
    fn plct_10_large_batch_accepted() {
        let ts: Vec<TransitionObservation> = (0..100)
            .map(|i| {
                let from = i % PLCT_NUM_CLASSES;
                let to = (from + 1) % PLCT_NUM_CLASSES;
                trans(from, to, (i as u64) + 1)
            })
            .collect();
        assert_eq!(validate_transition_uniformity(&ts), Ok(()));
    }
}
