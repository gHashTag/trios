//! # CR-CHAT-05 — Store revision monotonicity guard (Wave-78 Lane A)
//!
//! PERSISTENCE — store revision numbers must be strictly monotone, R-CHAT-5.
//!
//! The persistence store assigns a revision number to each write. If
//! revisions are not strictly increasing:
//!
//! * **Revision regression** — a write with an older revision overwrites
//!   newer data, effectively rolling back state.
//! * **Revision gap** — a skipped revision number means a write was
//!   lost (crash between assignment and commit).
//! * **Duplicate revision** — two writes share the same revision,
//!   causing non-deterministic last-write-wins.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Revisions are strictly increasing (no regression).
//! 2. No duplicate revisions.
//! 3. No gaps in revision sequence.
//! 4. First revision >= `SRVM_MIN_REVISION`.
//! 5. Last revision <= `SRVM_MAX_REVISION`.
//! 6. Revision count <= `SRVM_MAX_REVISIONS`.
//!
//! Tests **SRVM-01..10**. Error enum [`RevisionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * STORE-REVISION-MONOTONE`

#![forbid(unsafe_code)]

/// Minimum starting revision.
pub const SRVM_MIN_REVISION: u64 = 1;

/// Maximum revision.
pub const SRVM_MAX_REVISION: u64 = 1_000_000;

/// Maximum revisions in a batch.
pub const SRVM_MAX_REVISIONS: usize = 1024;

/// All ways revision monotonicity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RevisionError {
    /// Revision regression.
    Regression(u64),
    /// Duplicate revision.
    Duplicate(u64),
    /// Gap in sequence.
    Gap {
        /// Expected revision.
        expected: u64,
        /// Found revision.
        found: u64,
    },
    /// Below minimum revision.
    BelowMinimum(u64),
    /// Above maximum revision.
    AboveMaximum(u64),
    /// Too many revisions.
    TooManyRevisions,
}

/// `[VERIFIED]` Validate that store revisions are strictly monotone with no gaps.
pub fn validate_revision_monotonicity(
    revisions: &[u64],
) -> Result<(), RevisionError> {
    if revisions.len() > SRVM_MAX_REVISIONS {
        return Err(RevisionError::TooManyRevisions);
    }
    if revisions.is_empty() {
        return Ok(());
    }
    let first = revisions[0];
    if first < SRVM_MIN_REVISION {
        return Err(RevisionError::BelowMinimum(first));
    }
    let mut prev = first;
    for i in 1..revisions.len() {
        let cur = revisions[i];
        if cur > SRVM_MAX_REVISION {
            return Err(RevisionError::AboveMaximum(cur));
        }
        if cur == prev {
            return Err(RevisionError::Duplicate(cur));
        }
        if cur < prev {
            return Err(RevisionError::Regression(cur));
        }
        if cur != prev + 1 {
            return Err(RevisionError::Gap { expected: prev + 1, found: cur });
        }
        prev = cur;
    }
    if first > SRVM_MAX_REVISION {
        return Err(RevisionError::AboveMaximum(first));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_revisions() -> Vec<u64> {
        vec![1, 2, 3, 4, 5]
    }

    /// **SRVM-01** — regression rejected.
    #[test]
    fn srvm_01_regression_rejected() {
        assert_eq!(
            validate_revision_monotonicity(&[1, 2, 3, 2]),
            Err(RevisionError::Regression(2))
        );
    }

    /// **SRVM-02** — duplicate rejected.
    #[test]
    fn srvm_02_duplicate_rejected() {
        assert_eq!(
            validate_revision_monotonicity(&[1, 2, 2, 3]),
            Err(RevisionError::Duplicate(2))
        );
    }

    /// **SRVM-03** — gap rejected.
    #[test]
    fn srvm_03_gap_rejected() {
        assert_eq!(
            validate_revision_monotonicity(&[1, 2, 5]),
            Err(RevisionError::Gap { expected: 3, found: 5 })
        );
    }

    /// **SRVM-04** — below minimum rejected.
    #[test]
    fn srvm_04_below_min_rejected() {
        assert_eq!(
            validate_revision_monotonicity(&[0, 1, 2]),
            Err(RevisionError::BelowMinimum(0))
        );
    }

    /// **SRVM-05** — above maximum rejected.
    #[test]
    fn srvm_05_above_max_rejected() {
        assert_eq!(
            validate_revision_monotonicity(&[SRVM_MAX_REVISION, SRVM_MAX_REVISION + 1]),
            Err(RevisionError::AboveMaximum(SRVM_MAX_REVISION + 1))
        );
    }

    /// **SRVM-06** — too many revisions rejected.
    #[test]
    fn srvm_06_too_many_rejected() {
        let revs: Vec<u64> = (1..=SRVM_MAX_REVISIONS as u64 + 1).collect();
        assert_eq!(
            validate_revision_monotonicity(&revs),
            Err(RevisionError::TooManyRevisions)
        );
    }

    /// **SRVM-07** — valid revisions accepted.
    #[test]
    fn srvm_07_valid_accepted() {
        assert_eq!(validate_revision_monotonicity(&valid_revisions()), Ok(()));
    }

    /// **SRVM-08** — empty accepted.
    #[test]
    fn srvm_08_empty_accepted() {
        assert_eq!(validate_revision_monotonicity(&[]), Ok(()));
    }

    /// **SRVM-09** — single revision accepted.
    #[test]
    fn srvm_09_single_accepted() {
        assert_eq!(validate_revision_monotonicity(&[1]), Ok(()));
    }

    /// **SRVM-10** — max revisions accepted.
    #[test]
    fn srvm_10_max_accepted() {
        let revs: Vec<u64> = (1..=SRVM_MAX_REVISIONS as u64).collect();
        assert_eq!(validate_revision_monotonicity(&revs), Ok(()));
    }
}
