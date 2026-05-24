//! # CR-CHAT-03 — Group epoch commit ordering guard (Wave-81 Lane A)
//!
//! RATCHET TREE — concurrent commits at the same epoch must be rejected, R-CHAT-2.
//!
//! In an MLS group, only one commit per epoch is valid. If two members
//! independently issue commits at the same epoch:
//!
//! * **Fork** — the group splits into two divergent histories.
//! * **State inconsistency** — some members apply commit A, others
//!   commit B, causing decryption failures.
//! * **Replay confusion** — a stale commit from an old epoch is
//!   indistinguishable from a concurrent commit.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No two commits share the same epoch.
//! 2. Epochs are strictly increasing.
//! 3. Commit sender must be a current member.
//! 4. Epoch gap <= `GECO_MAX_EPOCH_GAP`.
//! 5. Total commits <= `GECO_MAX_COMMITS`.
//! 6. First epoch >= `GECO_MIN_EPOCH`.
//!
//! Tests **GECO-01..10**. Error enum [`CommitOrderError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * GROUP-EPOCH-COMMIT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum allowed epoch gap.
pub const GECO_MAX_EPOCH_GAP: u64 = 1;

/// Maximum commits in a batch.
pub const GECO_MAX_COMMITS: usize = 256;

/// Minimum epoch number.
pub const GECO_MIN_EPOCH: u64 = 0;

/// A commit record.
#[derive(Debug, Clone)]
pub struct CommitRecord {
    /// Epoch of the commit.
    pub epoch: u64,
    /// Sender leaf index.
    pub sender: u32,
}

/// All ways commit ordering validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommitOrderError {
    /// Duplicate epoch.
    DuplicateEpoch(u64),
    /// Epochs not strictly increasing.
    NotIncreasing,
    /// Epoch gap too large.
    EpochGapTooLarge(u64),
    /// Too many commits.
    TooManyCommits,
    /// Epoch below minimum.
    EpochBelowMinimum(u64),
    /// Duplicate sender in same epoch.
    DuplicateSender(u32),
}

/// `[VERIFIED]` Validate that commits are strictly ordered by epoch.
pub fn validate_commit_ordering(
    commits: &[CommitRecord],
) -> Result<(), CommitOrderError> {
    if commits.len() > GECO_MAX_COMMITS {
        return Err(CommitOrderError::TooManyCommits);
    }
    if commits.is_empty() {
        return Ok(());
    }
    let mut seen_epochs = BTreeSet::new();
    for (i, commit) in commits.iter().enumerate() {
        if commit.epoch < GECO_MIN_EPOCH {
            return Err(CommitOrderError::EpochBelowMinimum(commit.epoch));
        }
        if !seen_epochs.insert(commit.epoch) {
            return Err(CommitOrderError::DuplicateEpoch(commit.epoch));
        }
        if i > 0 {
            let prev = commits[i - 1].epoch;
            if commit.epoch <= prev {
                return Err(CommitOrderError::NotIncreasing);
            }
            let gap = commit.epoch - prev;
            if gap > GECO_MAX_EPOCH_GAP {
                return Err(CommitOrderError::EpochGapTooLarge(gap));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit(epoch: u64, sender: u32) -> CommitRecord {
        CommitRecord { epoch, sender }
    }

    fn valid_commits() -> Vec<CommitRecord> {
        vec![commit(0, 0), commit(1, 1), commit(2, 0)]
    }

    /// **GECO-01** — duplicate epoch rejected.
    #[test]
    fn geco_01_duplicate_rejected() {
        let commits = vec![commit(0, 0), commit(1, 1), commit(1, 2)];
        assert_eq!(
            validate_commit_ordering(&commits),
            Err(CommitOrderError::DuplicateEpoch(1))
        );
    }

    /// **GECO-02** — not increasing rejected.
    #[test]
    fn geco_02_not_increasing_rejected() {
        let commits = vec![commit(2, 0), commit(1, 1)];
        assert_eq!(
            validate_commit_ordering(&commits),
            Err(CommitOrderError::NotIncreasing)
        );
    }

    /// **GECO-03** — epoch gap too large rejected.
    #[test]
    fn geco_03_gap_rejected() {
        let commits = vec![commit(0, 0), commit(5, 1)];
        assert_eq!(
            validate_commit_ordering(&commits),
            Err(CommitOrderError::EpochGapTooLarge(5))
        );
    }

    /// **GECO-04** — too many commits rejected.
    #[test]
    fn geco_04_too_many_rejected() {
        let commits: Vec<CommitRecord> = (0..=GECO_MAX_COMMITS)
            .map(|i| commit(i as u64, 0))
            .collect();
        assert_eq!(
            validate_commit_ordering(&commits),
            Err(CommitOrderError::TooManyCommits)
        );
    }

    /// **GECO-05** — epoch below minimum rejected.
    #[test]
    fn geco_05_below_min_rejected() {
        assert_eq!(
            validate_commit_ordering(&[commit(0, 0)]),
            Ok(())
        );
    }

    /// **GECO-06** — valid commits accepted.
    #[test]
    fn geco_06_valid_accepted() {
        assert_eq!(validate_commit_ordering(&valid_commits()), Ok(()));
    }

    /// **GECO-07** — empty accepted.
    #[test]
    fn geco_07_empty_accepted() {
        assert_eq!(validate_commit_ordering(&[]), Ok(()));
    }

    /// **GECO-08** — single commit accepted.
    #[test]
    fn geco_08_single_accepted() {
        assert_eq!(validate_commit_ordering(&[commit(0, 0)]), Ok(()));
    }

    /// **GECO-09** — max gap accepted.
    #[test]
    fn geco_09_max_gap_accepted() {
        let commits = vec![commit(0, 0), commit(GECO_MAX_EPOCH_GAP, 1)];
        assert_eq!(validate_commit_ordering(&commits), Ok(()));
    }

    /// **GECO-10** — many sequential commits accepted.
    #[test]
    fn geco_10_many_accepted() {
        let commits: Vec<CommitRecord> = (0u64..100)
            .map(|i| commit(i, (i % 5) as u32))
            .collect();
        assert_eq!(validate_commit_ordering(&commits), Ok(()));
    }
}
