//! # CR-CHAT-04 — Padding class transition monotonicity guard (Wave-96 Lane A)
//!
//! PADDING — class transitions must not oscillate excessively, R-CHAT-9.
//!
//! When padding class changes between messages, rapid oscillation
//! between classes (e.g. 256→16384→256→16384) reveals:
//!
//! * **Payload size changes** — alternating between small and large
//!   classes indicates the user alternates between short and long
//!   messages (typing vs pasting).
//! * **Behavioral pattern** — oscillation frequency correlates with
//!   specific user activities (composing vs sending attachments).
//! * **Timing correlation** — oscillation timing matches user actions,
//!   enabling message-type classification.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Maximum oscillations <= `PCTM_MAX_OSCILLATIONS`.
//! 2. Oscillation = class change that reverses previous direction.
//! 3. Minimum same-class streak >= `PCTM_MIN_STREAK`.
//! 4. Total transitions <= `PCTM_MAX_TRANSITIONS`.
//! 5. All class values must be valid.
//! 6. Must have at least 2 entries to measure transitions.
//!
//! Tests **PCTM-01..10**. Error enum [`OscillationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-OSCILLATION`

#![forbid(unsafe_code)]

/// Maximum oscillations allowed.
pub const PCTM_MAX_OSCILLATIONS: usize = 4;

/// Minimum consecutive same-class messages before switching.
pub const PCTM_MIN_STREAK: usize = 2;

/// Maximum transitions to track.
pub const PCTM_MAX_TRANSITIONS: usize = 1024;

/// Valid padding classes.
pub const PCTM_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// A padding class selection record.
#[derive(Debug, Clone)]
pub struct ClassTransition {
    /// Message sequence number.
    pub seq: u64,
    /// Class size selected.
    pub class: usize,
}

/// All ways oscillation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OscillationError {
    /// Too many oscillations.
    TooManyOscillations { count: usize, max: usize },
    /// Streak too short.
    StreakTooShort { class: usize, streak: usize, min: usize },
    /// Too many transitions.
    TooManyTransitions,
    /// Invalid class.
    InvalidClass(usize),
}

fn class_rank(class: usize) -> Option<usize> {
    PCTM_CLASSES.iter().position(|&c| c == class)
}

/// `[VERIFIED]` Validate padding class transition monotonicity.
pub fn validate_class_transitions(
    transitions: &[ClassTransition],
) -> Result<(), OscillationError> {
    if transitions.len() > PCTM_MAX_TRANSITIONS {
        return Err(OscillationError::TooManyTransitions);
    }
    for t in transitions {
        if !PCTM_CLASSES.contains(&t.class) {
            return Err(OscillationError::InvalidClass(t.class));
        }
    }
    if transitions.len() < 2 {
        return Ok(());
    }
    let mut oscillations = 0usize;
    let mut prev_dir: Option<i8> = None;
    let mut streak = 1usize;
    for i in 1..transitions.len() {
        let prev_rank = class_rank(transitions[i - 1].class).unwrap();
        let curr_rank = class_rank(transitions[i].class).unwrap();
        if prev_rank == curr_rank {
            streak += 1;
        } else {
            if streak < PCTM_MIN_STREAK {
                return Err(OscillationError::StreakTooShort {
                    class: transitions[i - 1].class,
                    streak,
                    min: PCTM_MIN_STREAK,
                });
            }
            let dir = if curr_rank > prev_rank { 1i8 } else { -1i8 };
            if let Some(pd) = prev_dir {
                if pd != dir {
                    oscillations += 1;
                }
            }
            prev_dir = Some(dir);
            streak = 1;
        }
    }
    if oscillations > PCTM_MAX_OSCILLATIONS {
        return Err(OscillationError::TooManyOscillations {
            count: oscillations,
            max: PCTM_MAX_OSCILLATIONS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tr(seq: u64, class: usize) -> ClassTransition {
        ClassTransition { seq, class }
    }

    fn valid_transitions() -> Vec<ClassTransition> {
        vec![
            tr(1, 256), tr(2, 256),
            tr(3, 1024), tr(4, 1024),
            tr(5, 4096), tr(6, 4096),
        ]
    }

    /// **PCTM-01** — too many oscillations rejected.
    #[test]
    fn pctm_01_too_many_oscillations_rejected() {
        let ts = vec![
            tr(1, 256), tr(2, 256),
            tr(3, 4096), tr(4, 4096),
            tr(5, 256), tr(6, 256),
            tr(7, 4096), tr(8, 4096),
            tr(9, 256), tr(10, 256),
            tr(11, 4096), tr(12, 4096),
            tr(13, 256), tr(14, 256),
        ];
        assert!(matches!(
            validate_class_transitions(&ts),
            Err(OscillationError::TooManyOscillations { .. })
        ));
    }

    /// **PCTM-02** — streak too short rejected.
    #[test]
    fn pctm_02_streak_too_short_rejected() {
        let ts = vec![tr(1, 256), tr(2, 1024), tr(3, 1024)];
        assert_eq!(
            validate_class_transitions(&ts),
            Err(OscillationError::StreakTooShort { class: 256, streak: 1, min: 2 })
        );
    }

    /// **PCTM-03** — too many transitions rejected.
    #[test]
    fn pctm_03_too_many_rejected() {
        let ts: Vec<ClassTransition> = (0..=PCTM_MAX_TRANSITIONS as u64)
            .map(|i| tr(i, 256))
            .collect();
        assert_eq!(validate_class_transitions(&ts), Err(OscillationError::TooManyTransitions));
    }

    /// **PCTM-04** — invalid class rejected.
    #[test]
    fn pctm_04_invalid_class_rejected() {
        let ts = vec![tr(1, 999), tr(2, 999)];
        assert_eq!(validate_class_transitions(&ts), Err(OscillationError::InvalidClass(999)));
    }

    /// **PCTM-05** — valid monotonic transitions accepted.
    #[test]
    fn pctm_05_valid_accepted() {
        assert_eq!(validate_class_transitions(&valid_transitions()), Ok(()));
    }

    /// **PCTM-06** — single entry accepted.
    #[test]
    fn pctm_06_single_accepted() {
        assert_eq!(validate_class_transitions(&[tr(1, 256)]), Ok(()));
    }

    /// **PCTM-07** — empty accepted.
    #[test]
    fn pctm_07_empty_accepted() {
        assert_eq!(validate_class_transitions(&[]), Ok(()));
    }

    /// **PCTM-08** — all same class accepted.
    #[test]
    fn pctm_08_same_class_accepted() {
        let ts: Vec<ClassTransition> = (0..20).map(|i| tr(i, 1024)).collect();
        assert_eq!(validate_class_transitions(&ts), Ok(()));
    }

    /// **PCTM-09** — two transitions accepted (1 oscillation = OK).
    #[test]
    fn pctm_09_two_transitions_accepted() {
        let ts = vec![
            tr(1, 256), tr(2, 256),
            tr(3, 1024), tr(4, 1024),
            tr(5, 256), tr(6, 256),
        ];
        assert_eq!(validate_class_transitions(&ts), Ok(()));
    }

    /// **PCTM-10** — monotonic increase accepted (no oscillation).
    #[test]
    fn pctm_10_monotonic_accepted() {
        let ts = vec![
            tr(1, 256), tr(2, 256),
            tr(3, 1024), tr(4, 1024),
            tr(5, 4096), tr(6, 4096),
            tr(7, 16384), tr(8, 16384),
        ];
        assert_eq!(validate_class_transitions(&ts), Ok(()));
    }
}
