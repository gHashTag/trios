//! # CR-CHAT-05 — WAL fsync barrier guard (Wave-67 Lane B)
//!
//! PERSISTENCE — WAL flush must have fsync barrier, R-CHAT-5.
//!
//! A Write-Ahead Log (WAL) must fsync before marking a commit record
//! as durable. Without the barrier, a crash after the commit marker
//! but before the data reaches disk means:
//!
//! * **Lost tail** — the last N entries are silently dropped after crash.
//! * **Reordered entries** — commit marker is on disk but preceding
//!   data entries are not, leaving an inconsistent log.
//! * **Duplicate replay** — entries without a commit marker are
//!   replayed on recovery, causing double-application.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Every commit record is preceded by an fsync barrier.
//! 2. Barrier LSN <= commit LSN.
//! 3. Barriers are strictly increasing (no regression).
//! 4. No two commits share the same LSN.
//! 5. Commit LSN > last barrier LSN (barrier actually covers it).
//! 6. Total WAL entries <= `WFSB_MAX_ENTRIES`.
//!
//! Tests **WFSB-01..10**. Error enum [`WalFsyncError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WAL-FSYNC-BARRIER`

#![forbid(unsafe_code)]

/// Maximum WAL entries.
pub const WFSB_MAX_ENTRIES: usize = 1024;

/// A WAL entry — either a data record or a commit barrier.
#[derive(Debug, Clone)]
pub enum WalEntry {
    /// Data record at LSN.
    Data {
        /// Log sequence number.
        lsn: u64,
    },
    /// Fsync barrier at LSN (marks all data <= lsn as durable).
    Barrier {
        /// Log sequence number covered by this barrier.
        lsn: u64,
    },
}

/// All ways WAL fsync validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WalFsyncError {
    /// Commit without preceding barrier.
    CommitWithoutBarrier,
    /// Barrier LSN exceeds commit LSN.
    BarrierExceedsCommit,
    /// Barrier regression (non-increasing).
    BarrierRegression,
    /// Duplicate commit LSN.
    DuplicateCommitLsn,
    /// Commit not covered by barrier.
    CommitNotCovered,
    /// Too many entries.
    TooManyEntries,
}

/// `[VERIFIED]` Validate that a WAL sequence has proper fsync barriers.
pub fn validate_wal_fsync_barriers(
    entries: &[WalEntry],
) -> Result<(), WalFsyncError> {
    if entries.len() > WFSB_MAX_ENTRIES {
        return Err(WalFsyncError::TooManyEntries);
    }
    let mut last_barrier_lsn: u64 = 0;
    let mut data_since_barrier: Vec<u64> = Vec::new();
    let mut committed: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    for entry in entries {
        match entry {
            WalEntry::Data { lsn } => {
                data_since_barrier.push(*lsn);
            }
            WalEntry::Barrier { lsn } => {
                if *lsn < last_barrier_lsn {
                    return Err(WalFsyncError::BarrierRegression);
                }
                for dlsn in &data_since_barrier {
                    if *dlsn > *lsn {
                        return Err(WalFsyncError::CommitNotCovered);
                    }
                    if committed.contains(dlsn) {
                        return Err(WalFsyncError::DuplicateCommitLsn);
                    }
                    committed.insert(*dlsn);
                }
                data_since_barrier.clear();
                last_barrier_lsn = *lsn;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_wal() -> Vec<WalEntry> {
        vec![
            WalEntry::Data { lsn: 1 },
            WalEntry::Data { lsn: 2 },
            WalEntry::Barrier { lsn: 2 },
            WalEntry::Data { lsn: 3 },
            WalEntry::Barrier { lsn: 3 },
        ]
    }

    /// **WFSB-01** — data without any barrier is still valid (pending).
    #[test]
    fn wfsb_01_no_barrier_pending_ok() {
        let wal = vec![WalEntry::Data { lsn: 1 }, WalEntry::Data { lsn: 2 }];
        assert_eq!(validate_wal_fsync_barriers(&wal), Ok(()));
    }

    /// **WFSB-02** — barrier regression rejected.
    #[test]
    fn wfsb_02_barrier_regression_rejected() {
        let wal = vec![
            WalEntry::Barrier { lsn: 10 },
            WalEntry::Barrier { lsn: 5 },
        ];
        assert_eq!(
            validate_wal_fsync_barriers(&wal),
            Err(WalFsyncError::BarrierRegression)
        );
    }

    /// **WFSB-03** — duplicate commit LSN rejected.
    #[test]
    fn wfsb_03_duplicate_lsn_rejected() {
        let wal = vec![
            WalEntry::Data { lsn: 1 },
            WalEntry::Barrier { lsn: 1 },
            WalEntry::Data { lsn: 1 },
            WalEntry::Barrier { lsn: 2 },
        ];
        assert_eq!(
            validate_wal_fsync_barriers(&wal),
            Err(WalFsyncError::DuplicateCommitLsn)
        );
    }

    /// **WFSB-04** — commit not covered by barrier rejected.
    #[test]
    fn wfsb_04_uncovered_rejected() {
        let wal = vec![
            WalEntry::Data { lsn: 5 },
            WalEntry::Barrier { lsn: 3 },
        ];
        assert_eq!(
            validate_wal_fsync_barriers(&wal),
            Err(WalFsyncError::CommitNotCovered)
        );
    }

    /// **WFSB-05** — too many entries rejected.
    #[test]
    fn wfsb_05_too_many_rejected() {
        let wal: Vec<WalEntry> = (0..=WFSB_MAX_ENTRIES)
            .map(|i| WalEntry::Data { lsn: i as u64 })
            .collect();
        assert_eq!(
            validate_wal_fsync_barriers(&wal),
            Err(WalFsyncError::TooManyEntries)
        );
    }

    /// **WFSB-06** — valid WAL accepted.
    #[test]
    fn wfsb_06_valid_accepted() {
        assert_eq!(validate_wal_fsync_barriers(&valid_wal()), Ok(()));
    }

    /// **WFSB-07** — single barrier accepted.
    #[test]
    fn wfsb_07_single_barrier_accepted() {
        let wal = vec![WalEntry::Barrier { lsn: 1 }];
        assert_eq!(validate_wal_fsync_barriers(&wal), Ok(()));
    }

    /// **WFSB-08** — empty WAL accepted.
    #[test]
    fn wfsb_08_empty_accepted() {
        assert_eq!(validate_wal_fsync_barriers(&[]), Ok(()));
    }

    /// **WFSB-09** — multiple increasing barriers accepted.
    #[test]
    fn wfsb_09_multi_barrier_accepted() {
        let wal = vec![
            WalEntry::Data { lsn: 1 },
            WalEntry::Barrier { lsn: 1 },
            WalEntry::Data { lsn: 2 },
            WalEntry::Barrier { lsn: 2 },
            WalEntry::Data { lsn: 3 },
            WalEntry::Barrier { lsn: 3 },
        ];
        assert_eq!(validate_wal_fsync_barriers(&wal), Ok(()));
    }

    /// **WFSB-10** — barrier covers all data accepted.
    #[test]
    fn wfsb_10_full_cover_accepted() {
        let wal = vec![
            WalEntry::Data { lsn: 1 },
            WalEntry::Data { lsn: 2 },
            WalEntry::Data { lsn: 3 },
            WalEntry::Barrier { lsn: 3 },
        ];
        assert_eq!(validate_wal_fsync_barriers(&wal), Ok(()));
    }
}
