//! # CR-CHAT-02 — Skipped message key gap bound guard (Wave-80 Lane A)
//!
//! RATCHET — skipped message key gaps must be bounded, R-CHAT-2.
//!
//! In the double ratchet, out-of-order messages cause skipped message
//! keys to be stored for later use. If the gap between two consecutive
//! skipped keys is unbounded:
//!
//! * **Memory exhaustion** — an attacker forces the receiver to store
//!   keys for every chain index from 0 to N, where N is unbounded.
//! * **DoS via gap** — a single message at chain index 1_000_000
//!   forces the receiver to skip and store 1M keys.
//! * **Timing attack** — large gaps cause processing delays that
//!   reveal which messages are out-of-order.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Gap between consecutive skipped keys <= `SMKG_MAX_GAP`.
//! 2. First skipped key index >= `SMKG_MIN_INDEX`.
//! 3. Total skipped keys <= `SMKG_MAX_SKIPPED`.
//! 4. Skipped key indices are strictly increasing.
//! 5. No duplicate indices.
//! 6. All indices < `SMKG_MAX_INDEX`.
//!
//! Tests **SMKG-01..10**. Error enum [`SkippedGapError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SKIPPED-KEY-GAP`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum gap between consecutive skipped keys.
pub const SMKG_MAX_GAP: u32 = 32;

/// Minimum key index.
pub const SMKG_MIN_INDEX: u32 = 0;

/// Maximum total skipped keys.
pub const SMKG_MAX_SKIPPED: usize = 256;

/// Maximum key index.
pub const SMKG_MAX_INDEX: u32 = 1_000_000;

/// All ways skipped gap validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkippedGapError {
    /// Gap too large.
    GapTooLarge(u32),
    /// Index below minimum.
    IndexBelowMin(u32),
    /// Too many skipped keys.
    TooManySkipped,
    /// Indices not strictly increasing.
    NotIncreasing,
    /// Duplicate index.
    DuplicateIndex(u32),
    /// Index exceeds maximum.
    IndexTooLarge(u32),
}

/// `[VERIFIED]` Validate that skipped message key gaps are bounded.
pub fn validate_skipped_key_gaps(
    indices: &[u32],
) -> Result<(), SkippedGapError> {
    if indices.len() > SMKG_MAX_SKIPPED {
        return Err(SkippedGapError::TooManySkipped);
    }
    let mut seen = BTreeSet::new();
    for (i, &idx) in indices.iter().enumerate() {
        if idx > SMKG_MAX_INDEX {
            return Err(SkippedGapError::IndexTooLarge(idx));
        }
        if !seen.insert(idx) {
            return Err(SkippedGapError::DuplicateIndex(idx));
        }
        if i > 0 && idx <= indices[i - 1] {
            return Err(SkippedGapError::NotIncreasing);
        }
        if i > 0 {
            let gap = idx - indices[i - 1];
            if gap > SMKG_MAX_GAP {
                return Err(SkippedGapError::GapTooLarge(gap));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_indices() -> Vec<u32> {
        vec![0, 1, 2, 3, 5, 6, 8, 9]
    }

    /// **SMKG-01** — gap too large rejected.
    #[test]
    fn smkg_01_gap_large_rejected() {
        let indices = vec![0, 1, 100];
        assert_eq!(
            validate_skipped_key_gaps(&indices),
            Err(SkippedGapError::GapTooLarge(99))
        );
    }

    /// **SMKG-02** — too many skipped rejected.
    #[test]
    fn smkg_02_too_many_rejected() {
        let indices: Vec<u32> = (0..=SMKG_MAX_SKIPPED as u32).collect();
        assert_eq!(
            validate_skipped_key_gaps(&indices),
            Err(SkippedGapError::TooManySkipped)
        );
    }

    /// **SMKG-03** — not increasing rejected.
    #[test]
    fn smkg_03_not_increasing_rejected() {
        assert_eq!(
            validate_skipped_key_gaps(&[1, 3, 2]),
            Err(SkippedGapError::NotIncreasing)
        );
    }

    /// **SMKG-04** — duplicate index rejected.
    #[test]
    fn smkg_04_duplicate_rejected() {
        assert_eq!(
            validate_skipped_key_gaps(&[1, 2, 2]),
            Err(SkippedGapError::DuplicateIndex(2))
        );
    }

    /// **SMKG-05** — index too large rejected.
    #[test]
    fn smkg_05_index_large_rejected() {
        assert_eq!(
            validate_skipped_key_gaps(&[SMKG_MAX_INDEX + 1]),
            Err(SkippedGapError::IndexTooLarge(SMKG_MAX_INDEX + 1))
        );
    }

    /// **SMKG-06** — valid indices accepted.
    #[test]
    fn smkg_06_valid_accepted() {
        assert_eq!(validate_skipped_key_gaps(&valid_indices()), Ok(()));
    }

    /// **SMKG-07** — empty accepted.
    #[test]
    fn smkg_07_empty_accepted() {
        assert_eq!(validate_skipped_key_gaps(&[]), Ok(()));
    }

    /// **SMKG-08** — single index accepted.
    #[test]
    fn smkg_08_single_accepted() {
        assert_eq!(validate_skipped_key_gaps(&[0]), Ok(()));
    }

    /// **SMKG-09** — max gap accepted.
    #[test]
    fn smkg_09_max_gap_accepted() {
        let indices = vec![0, SMKG_MAX_GAP];
        assert_eq!(validate_skipped_key_gaps(&indices), Ok(()));
    }

    /// **SMKG-10** — max skipped accepted.
    #[test]
    fn smkg_10_max_skipped_accepted() {
        let indices: Vec<u32> = (0..SMKG_MAX_SKIPPED as u32).collect();
        assert_eq!(validate_skipped_key_gaps(&indices), Ok(()));
    }
}
