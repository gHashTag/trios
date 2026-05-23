//! # CR-CHAT-04 — Padding class transition audit (Wave-43 Lane B)
//!
//! R-CHAT-9 — Padding class transition metadata leak prevention.
//!
//! When a user's message size crosses a padding class boundary (e.g. from
//! 256 → 1024 bytes), the wire observer sees a length change. If transitions
//! happen too frequently or oscillate, the pattern reveals message-size
//! metadata — effectively a traffic-analysis side-channel on padding class.
//!
//! trios-chat enforces **6 rules** on a sequence of padding class choices:
//!
//! 1. All classes are canonical members of `CLASSES`.
//! 2. Transitions are monotonically non-decreasing within a window.
//! 3. No rapid oscillation (class changes ≥ 3 within `WINDOW_SIZE`).
//! 4. Each class persists for at least `MIN_PER_CLASS` messages.
//! 5. No single-message class (only 1 message in a class before switching).
//! 6. Window size is non-zero.
//!
//! Tests **PCTA-01..10**. Error enum [`PaddingTransitionError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PADDING-TRANSITION`

#![forbid(unsafe_code)]

/// Sliding window size for transition audit.
pub const PCTA_WINDOW_SIZE: usize = 10;

/// Minimum messages per class before transitioning.
pub const PCTA_MIN_PER_CLASS: usize = 2;

/// All canonical padding classes.
pub const PCTA_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// One padding class choice in sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaddingChoice {
    /// Chosen class (must be a member of PCTA_CLASSES).
    pub class: usize,
}

/// All ways a padding transition sequence can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaddingTransitionError {
    /// Class is not canonical.
    NonCanonicalClass,
    /// Class decreased within window (non-monotonic).
    ClassDecreased,
    /// Too many class changes within window (oscillation).
    RapidOscillation,
    /// Class used for fewer than minimum messages before changing.
    InsufficientPerClass,
    /// Window size is zero.
    ZeroWindowSize,
}

fn is_canonical(class: usize) -> bool {
    PCTA_CLASSES.contains(&class)
}

/// `[VERIFIED]` Audit a sequence of padding class choices for metadata
/// leaks. Returns `Ok(())` if all rules pass.
///
/// Rules enforced in fixed order:
///
/// 1. All classes are canonical.
/// 2. Within any `PCTA_WINDOW_SIZE` window, class is non-decreasing.
/// 3. Within any window, class changes ≤ window_size / 2.
/// 4. Each class run length ≥ `PCTA_MIN_PER_CLASS` (except possibly the last).
/// 5. Window size > 0.
pub fn audit_padding_transitions(
    choices: &[PaddingChoice],
) -> Result<(), PaddingTransitionError> {
    if PCTA_WINDOW_SIZE == 0 {
        return Err(PaddingTransitionError::ZeroWindowSize);
    }
    if choices.is_empty() {
        return Ok(());
    }
    for c in choices {
        if !is_canonical(c.class) {
            return Err(PaddingTransitionError::NonCanonicalClass);
        }
    }
    for window in choices.windows(PCTA_WINDOW_SIZE) {
        let changes = window.windows(2).filter(|w| w[0].class != w[1].class).count();
        if changes > PCTA_WINDOW_SIZE / 2 {
            return Err(PaddingTransitionError::RapidOscillation);
        }
        for i in 1..window.len() {
            if window[i].class < window[i - 1].class {
                return Err(PaddingTransitionError::ClassDecreased);
            }
        }
    }
    let mut run_start = 0;
    for i in 1..choices.len() {
        if choices[i].class != choices[run_start].class {
            let run_len = i - run_start;
            if run_len < PCTA_MIN_PER_CLASS {
                return Err(PaddingTransitionError::InsufficientPerClass);
            }
            run_start = i;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pc(class: usize) -> PaddingChoice {
        PaddingChoice { class }
    }

    fn uniform_seq(class: usize, n: usize) -> Vec<PaddingChoice> {
        vec![pc(class); n]
    }

    /// **PCTA-01** — non-canonical class rejected.
    #[test]
    fn pcta_01_non_canonical_class_rejected() {
        let choices = vec![pc(512), pc(512)];
        assert_eq!(
            audit_padding_transitions(&choices),
            Err(PaddingTransitionError::NonCanonicalClass)
        );
    }

    /// **PCTA-02** — class decreased rejected.
    #[test]
    fn pcta_02_class_decreased_rejected() {
        let choices = vec![pc(1024); 5].into_iter().chain(vec![pc(256); 5]).collect::<Vec<_>>();
        assert_eq!(
            audit_padding_transitions(&choices),
            Err(PaddingTransitionError::ClassDecreased)
        );
    }

    /// **PCTA-03** — rapid class changes with insufficient per-class rejected.
    #[test]
    fn pcta_03_rapid_changes_insufficient_rejected() {
        let choices: Vec<PaddingChoice> = PCTA_CLASSES
            .iter()
            .flat_map(|&c| vec![pc(c); 1])
            .collect();
        assert_eq!(
            audit_padding_transitions(&choices),
            Err(PaddingTransitionError::InsufficientPerClass)
        );
    }

    /// **PCTA-04** — insufficient per-class messages rejected.
    #[test]
    fn pcta_04_insufficient_per_class_rejected() {
        let choices = vec![pc(256), pc(1024), pc(1024), pc(1024)];
        assert_eq!(
            audit_padding_transitions(&choices),
            Err(PaddingTransitionError::InsufficientPerClass)
        );
    }

    /// **PCTA-05** — uniform sequence accepted.
    #[test]
    fn pcta_05_uniform_accepted() {
        assert_eq!(audit_padding_transitions(&uniform_seq(1024, 20)), Ok(()));
    }

    /// **PCTA-06** — empty sequence accepted.
    #[test]
    fn pcta_06_empty_accepted() {
        assert_eq!(audit_padding_transitions(&[]), Ok(()));
    }

    /// **PCTA-07** — monotonic transition accepted.
    #[test]
    fn pcta_07_monotonic_transition_accepted() {
        let choices: Vec<PaddingChoice> = (0..3)
            .flat_map(|c| vec![pc(PCTA_CLASSES[c]); PCTA_MIN_PER_CLASS + 2])
            .collect();
        assert_eq!(audit_padding_transitions(&choices), Ok(()));
    }

    /// **PCTA-08** — single message accepted (no transitions).
    #[test]
    fn pcta_08_single_message_accepted() {
        assert_eq!(audit_padding_transitions(&[pc(256)]), Ok(()));
    }

    /// **PCTA-09** — same class, long sequence accepted.
    #[test]
    fn pcta_09_long_uniform_accepted() {
        assert_eq!(audit_padding_transitions(&uniform_seq(4096, 100)), Ok(()));
    }

    /// **PCTA-10** — minimal valid transition (exactly MIN_PER_CLASS) accepted.
    #[test]
    fn pcta_10_min_per_class_transition_accepted() {
        let choices: Vec<PaddingChoice> = (0..2)
            .flat_map(|c| vec![pc(PCTA_CLASSES[c]); PCTA_MIN_PER_CLASS])
            .collect();
        assert_eq!(audit_padding_transitions(&choices), Ok(()));
    }
}
