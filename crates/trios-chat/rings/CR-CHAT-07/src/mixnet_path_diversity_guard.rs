//! # CR-CHAT-07 — Mixnet path diversity guard (Wave-73 Lane B)
//!
//! ANTI-CORRELATION — consecutive messages must not share the same hop-set, R-CHAT-10.
//!
//! In a mixnet, each message follows a path through relay nodes. If
//! consecutive messages use the same or highly overlapping hop-sets:
//!
//! * **Traceability** — an observer correlating traffic at shared hops
//!   can link consecutive messages to the same sender.
//! * **Intersection attack** — repeated use of the same relay reveals
//!   the sender's entry node.
//! * **De-anonymization** — small hop-set overlaps across many messages
//!   narrow down the sender.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No two consecutive paths share > `MPDG_MAX_OVERLAP` relays.
//! 2. Path length >= `MPDG_MIN_PATH_LEN`.
//! 3. Path length <= `MPDG_MAX_PATH_LEN`.
//! 4. No duplicate relays within a single path.
//! 5. Relay ID is non-zero.
//! 6. Consecutive pair count >= 2 to check.
//!
//! Tests **MPDG-01..10**. Error enum [`PathDiversityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MIXNET-PATH-DIVERSITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum shared relays between consecutive paths.
pub const MPDG_MAX_OVERLAP: usize = 1;

/// Minimum path length.
pub const MPDG_MIN_PATH_LEN: usize = 3;

/// Maximum path length.
pub const MPDG_MAX_PATH_LEN: usize = 8;

/// All ways path diversity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathDiversityError {
    /// Too much overlap between consecutive paths.
    TooMuchOverlap(usize),
    /// Path too short.
    PathTooShort,
    /// Path too long.
    PathTooLong,
    /// Duplicate relay in single path.
    DuplicateRelay,
    /// Zero relay ID.
    ZeroRelayId,
    /// Need at least 2 paths to check.
    TooFewPaths,
}

/// `[VERIFIED]` Validate that consecutive mixnet paths have sufficient diversity.
pub fn validate_path_diversity(
    paths: &[&[u64]],
) -> Result<(), PathDiversityError> {
    if paths.len() < 2 {
        return Err(PathDiversityError::TooFewPaths);
    }
    for path in paths {
        if path.len() < MPDG_MIN_PATH_LEN {
            return Err(PathDiversityError::PathTooShort);
        }
        if path.len() > MPDG_MAX_PATH_LEN {
            return Err(PathDiversityError::PathTooLong);
        }
        let mut seen = BTreeSet::new();
        for &relay in *path {
            if relay == 0 {
                return Err(PathDiversityError::ZeroRelayId);
            }
            if !seen.insert(relay) {
                return Err(PathDiversityError::DuplicateRelay);
            }
        }
    }
    for w in paths.windows(2) {
        let set_a: BTreeSet<u64> = w[0].iter().copied().collect();
        let set_b: BTreeSet<u64> = w[1].iter().copied().collect();
        let overlap = set_a.intersection(&set_b).count();
        if overlap > MPDG_MAX_OVERLAP {
            return Err(PathDiversityError::TooMuchOverlap(overlap));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path1() -> Vec<u64> {
        vec![10, 20, 30]
    }

    fn path2() -> Vec<u64> {
        vec![40, 50, 60]
    }

    fn path3() -> Vec<u64> {
        vec![70, 80, 90]
    }

    /// **MPDG-01** — too much overlap rejected.
    #[test]
    fn mpdg_01_overlap_rejected() {
        let p1 = vec![10, 20, 30];
        let p2 = vec![10, 20, 40];
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Err(PathDiversityError::TooMuchOverlap(2))
        );
    }

    /// **MPDG-02** — path too short rejected.
    #[test]
    fn mpdg_02_too_short_rejected() {
        let p1 = vec![10, 20];
        let p2 = vec![30, 40];
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Err(PathDiversityError::PathTooShort)
        );
    }

    /// **MPDG-03** — path too long rejected.
    #[test]
    fn mpdg_03_too_long_rejected() {
        let p1: Vec<u64> = (1..=MPDG_MAX_PATH_LEN as u64 + 1).map(|i| i * 10).collect();
        let p2 = path2();
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Err(PathDiversityError::PathTooLong)
        );
    }

    /// **MPDG-04** — duplicate relay in path rejected.
    #[test]
    fn mpdg_04_duplicate_rejected() {
        let p1 = vec![10, 10, 30];
        let p2 = path2();
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Err(PathDiversityError::DuplicateRelay)
        );
    }

    /// **MPDG-05** — zero relay ID rejected.
    #[test]
    fn mpdg_05_zero_relay_rejected() {
        let p1 = vec![10, 0, 30];
        let p2 = path2();
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Err(PathDiversityError::ZeroRelayId)
        );
    }

    /// **MPDG-06** — too few paths rejected.
    #[test]
    fn mpdg_06_too_few_rejected() {
        assert_eq!(
            validate_path_diversity(&[path1().as_slice()]),
            Err(PathDiversityError::TooFewPaths)
        );
    }

    /// **MPDG-07** — diverse paths accepted.
    #[test]
    fn mpdg_07_diverse_accepted() {
        assert_eq!(
            validate_path_diversity(&[path1().as_slice(), path2().as_slice()]),
            Ok(())
        );
    }

    /// **MPDG-08** — single overlap accepted.
    #[test]
    fn mpdg_08_single_overlap_accepted() {
        let p1 = vec![10, 20, 30];
        let p2 = vec![30, 40, 50];
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Ok(())
        );
    }

    /// **MPDG-09** — three diverse paths accepted.
    #[test]
    fn mpdg_09_three_paths_accepted() {
        assert_eq!(
            validate_path_diversity(&[
                path1().as_slice(),
                path2().as_slice(),
                path3().as_slice(),
            ]),
            Ok(())
        );
    }

    /// **MPDG-10** — max path length accepted.
    #[test]
    fn mpdg_10_max_len_accepted() {
        let p1: Vec<u64> = (1..=MPDG_MAX_PATH_LEN as u64).map(|i| i * 10).collect();
        let p2: Vec<u64> = (100u64..100 + MPDG_MAX_PATH_LEN as u64).map(|i| i * 10).collect();
        assert_eq!(
            validate_path_diversity(&[p1.as_slice(), p2.as_slice()]),
            Ok(())
        );
    }
}
