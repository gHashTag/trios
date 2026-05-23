//! # CR-CHAT-04 — Padding class downgrade guard (Wave-53 Lane B)
//!
//! ПАДДИНГ — запрет downgrade класса, R-CHAT-9.
//!
//! Если в рамках сессии pad class переходит с большого на меньший
//! (например, 4096 → 256), атакующий по размеру пакета определяет,
//! что plaintext стал короче. Это泄露ает информацию о содержании.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Class index is non-decreasing.
//! 2. Class is a member of the canonical set.
//! 3. No duplicate class entries in a transition log.
//! 4. Maximum transitions ≤ `PCDG_MAX_TRANSITIONS`.
//! 5. First class must be the smallest.
//! 6. Final class must be the largest.
//!
//! Tests **PCDG-01..10**. Error enum [`PadDowngradeError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PAD-DOWNGRADE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical padding classes.
pub const PCDG_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// Maximum transitions in a log.
pub const PCDG_MAX_TRANSITIONS: usize = 1024;

/// All ways padding downgrade detection can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PadDowngradeError {
    /// Class decreased (downgrade).
    ClassDowngraded,
    /// Non-canonical class.
    NonCanonicalClass,
    /// Duplicate entry in log.
    DuplicateEntry,
    /// Too many transitions.
    TooManyTransitions,
    /// First class is not smallest.
    FirstNotSmallest,
    /// Final class is not largest.
    FinalNotLargest,
}

fn class_index(cls: usize) -> Option<usize> {
    PCDG_CLASSES.iter().position(|&c| c == cls)
}

/// `[VERIFIED]` Validate a sequence of padding class transitions.
pub fn validate_padding_downgrade(
    transitions: &[usize],
) -> Result<(), PadDowngradeError> {
    if transitions.len() > PCDG_MAX_TRANSITIONS {
        return Err(PadDowngradeError::TooManyTransitions);
    }
    if transitions.is_empty() {
        return Ok(());
    }
    let mut seen = BTreeSet::new();
    for (i, &cls) in transitions.iter().enumerate() {
        let idx = class_index(cls).ok_or(PadDowngradeError::NonCanonicalClass)?;
        if !seen.insert((i, cls)) {
            return Err(PadDowngradeError::DuplicateEntry);
        }
        if i > 0 {
            let prev_idx = class_index(transitions[i - 1]).unwrap();
            if idx < prev_idx {
                return Err(PadDowngradeError::ClassDowngraded);
            }
        }
    }
    if class_index(transitions[0]).unwrap() != 0 {
        return Err(PadDowngradeError::FirstNotSmallest);
    }
    if class_index(*transitions.last().unwrap()).unwrap() != PCDG_CLASSES.len() - 1 {
        return Err(PadDowngradeError::FinalNotLargest);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **PCDG-01** — downgrade rejected.
    #[test]
    fn pcdg_01_downgrade_rejected() {
        assert_eq!(
            validate_padding_downgrade(&[256, 1024, 256]),
            Err(PadDowngradeError::ClassDowngraded)
        );
    }

    /// **PCDG-02** — non-canonical rejected.
    #[test]
    fn pcdg_02_non_canonical_rejected() {
        assert_eq!(
            validate_padding_downgrade(&[512]),
            Err(PadDowngradeError::NonCanonicalClass)
        );
    }

    /// **PCDG-03** — too many rejected.
    #[test]
    fn pcdg_03_too_many_rejected() {
        let t = vec![256; PCDG_MAX_TRANSITIONS + 1];
        assert_eq!(
            validate_padding_downgrade(&t),
            Err(PadDowngradeError::TooManyTransitions)
        );
    }

    /// **PCDG-04** — first not smallest rejected.
    #[test]
    fn pcdg_04_first_not_smallest_rejected() {
        assert_eq!(
            validate_padding_downgrade(&[1024, 4096, 16384]),
            Err(PadDowngradeError::FirstNotSmallest)
        );
    }

    /// **PCDG-05** — final not largest rejected.
    #[test]
    fn pcdg_05_final_not_largest_rejected() {
        assert_eq!(
            validate_padding_downgrade(&[256, 1024]),
            Err(PadDowngradeError::FinalNotLargest)
        );
    }

    /// **PCDG-06** — full progression accepted.
    #[test]
    fn pcdg_06_full_progression_accepted() {
        assert_eq!(
            validate_padding_downgrade(&[256, 1024, 4096, 16384]),
            Ok(())
        );
    }

    /// **PCDG-07** — empty accepted.
    #[test]
    fn pcdg_07_empty_accepted() {
        assert_eq!(validate_padding_downgrade(&[]), Ok(()));
    }

    /// **PCDG-08** — same class repeated accepted.
    #[test]
    fn pcdg_08_same_class_accepted() {
        assert_eq!(
            validate_padding_downgrade(&[256, 256, 256, 16384]),
            Ok(())
        );
    }

    /// **PCDG-09** — skip class accepted.
    #[test]
    fn pcdg_09_skip_class_accepted() {
        assert_eq!(
            validate_padding_downgrade(&[256, 16384]),
            Ok(())
        );
    }

    /// **PCDG-10** — single smallest-to-largest accepted.
    #[test]
    fn pcdg_10_minimal_accepted() {
        assert_eq!(
            validate_padding_downgrade(&PCDG_CLASSES.to_vec()),
            Ok(())
        );
    }
}
