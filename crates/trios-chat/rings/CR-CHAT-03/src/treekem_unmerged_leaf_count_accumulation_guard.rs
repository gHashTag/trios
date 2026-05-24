//! # CR-CHAT-03 — TreeKEM unmerged leaf count accumulation guard (Wave-134 Lane A)
//!
//! RATCHET TREE — the number of unmerged leaves (leaves that have not
//! been updated after a group operation) must be bounded; excessive
//! unmerged leaves indicate deferred updates piling up, weakening
//! forward secrecy.
//!
//! In TreeKEM, after a Commit, some leaves may remain "unmerged" —
//! their path secrets have not been refreshed. If too many leaves stay
//! unmerged across multiple Commits:
//!
//! * **Forward secrecy gap** — unmerged leaves retain old path secrets
//!   that could be compromised.
//! * **Resolution degradation** — the tree resolution path may fall
//!   back to stale nodes.
//! * **Compound risk** — each unmerged leaf is a potential entry point
//!   for an attacker who has compromised that leaf's key material.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Unmerged leaf count <= `TULB_MAX_UNMERGED`.
//! 2. Total leaf count >= `TULB_MIN_LEAVES`.
//! 3. Epoch must be > 0 (tree must have been initialised).
//! 4. Group ID must not be zero.
//! 5. No duplicate group IDs.
//! 6. Total entries <= `TULB_MAX_ENTRIES`.
//!
//! Tests **TULB-01..10**. Error enum [`UnmergedLeafError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TREE-HEALTHY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum unmerged leaves.
pub const TULB_MAX_UNMERGED: usize = 64;

/// Minimum total leaves.
pub const TULB_MIN_LEAVES: usize = 2;

/// Maximum entries per batch.
pub const TULB_MAX_ENTRIES: usize = 512;

/// Group ID length.
pub const TULB_GROUP_ID_LEN: usize = 32;

/// An unmerged leaf count record.
#[derive(Debug, Clone)]
pub struct UnmergedLeafRecord {
    /// Group identifier.
    pub group_id: [u8; TULB_GROUP_ID_LEN],
    /// Current epoch.
    pub epoch: u64,
    /// Total number of leaves.
    pub total_leaves: usize,
    /// Number of unmerged leaves.
    pub unmerged_count: usize,
}

/// All ways unmerged leaf count validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnmergedLeafError {
    /// Too many unmerged leaves.
    TooManyUnmerged { idx: usize, got: usize, max: usize },
    /// Too few total leaves.
    TooFewLeaves { idx: usize, got: usize, min: usize },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Zero group ID.
    ZeroGroupId(usize),
    /// Duplicate group ID.
    DuplicateGroupId { idx: usize },
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM unmerged leaf count accumulation.
pub fn validate_unmerged_leaf_count(
    records: &[UnmergedLeafRecord],
) -> Result<(), UnmergedLeafError> {
    if records.len() > TULB_MAX_ENTRIES {
        return Err(UnmergedLeafError::TooMany {
            got: records.len(),
            max: TULB_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; TULB_GROUP_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.group_id == [0u8; TULB_GROUP_ID_LEN] {
            return Err(UnmergedLeafError::ZeroGroupId(i));
        }
        if !seen.insert(r.group_id) {
            return Err(UnmergedLeafError::DuplicateGroupId { idx: i });
        }
        if r.epoch == 0 {
            return Err(UnmergedLeafError::ZeroEpoch(i));
        }
        if r.total_leaves < TULB_MIN_LEAVES {
            return Err(UnmergedLeafError::TooFewLeaves {
                idx: i,
                got: r.total_leaves,
                min: TULB_MIN_LEAVES,
            });
        }
        if r.unmerged_count > TULB_MAX_UNMERGED {
            return Err(UnmergedLeafError::TooManyUnmerged {
                idx: i,
                got: r.unmerged_count,
                max: TULB_MAX_UNMERGED,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid(byte: u8) -> [u8; TULB_GROUP_ID_LEN] {
        [byte; TULB_GROUP_ID_LEN]
    }

    fn rec(id: u8, epoch: u64, total: usize, unmerged: usize) -> UnmergedLeafRecord {
        UnmergedLeafRecord { group_id: gid(id), epoch, total_leaves: total, unmerged_count: unmerged }
    }

    fn valid_records() -> Vec<UnmergedLeafRecord> {
        vec![
            rec(0x01, 3, 16, 5),
            rec(0x02, 7, 32, 10),
        ]
    }

    /// **TULB-01** — too many unmerged rejected.
    #[test]
    fn tulb_01_too_many_unmerged_rejected() {
        let r = rec(0x01, 1, 100, TULB_MAX_UNMERGED + 1);
        assert_eq!(
            validate_unmerged_leaf_count(&[r]),
            Err(UnmergedLeafError::TooManyUnmerged {
                idx: 0,
                got: TULB_MAX_UNMERGED + 1,
                max: TULB_MAX_UNMERGED,
            })
        );
    }

    /// **TULB-02** — too few leaves rejected.
    #[test]
    fn tulb_02_too_few_leaves_rejected() {
        let r = rec(0x01, 1, TULB_MIN_LEAVES - 1, 0);
        assert_eq!(
            validate_unmerged_leaf_count(&[r]),
            Err(UnmergedLeafError::TooFewLeaves {
                idx: 0,
                got: TULB_MIN_LEAVES - 1,
                min: TULB_MIN_LEAVES,
            })
        );
    }

    /// **TULB-03** — zero epoch rejected.
    #[test]
    fn tulb_03_zero_epoch_rejected() {
        let r = UnmergedLeafRecord { group_id: gid(0x01), epoch: 0, total_leaves: 16, unmerged_count: 5 };
        assert_eq!(
            validate_unmerged_leaf_count(&[r]),
            Err(UnmergedLeafError::ZeroEpoch(0))
        );
    }

    /// **TULB-04** — zero group ID rejected.
    #[test]
    fn tulb_04_zero_group_id_rejected() {
        let r = UnmergedLeafRecord {
            group_id: [0u8; TULB_GROUP_ID_LEN],
            epoch: 1,
            total_leaves: 16,
            unmerged_count: 5,
        };
        assert_eq!(
            validate_unmerged_leaf_count(&[r]),
            Err(UnmergedLeafError::ZeroGroupId(0))
        );
    }

    /// **TULB-05** — duplicate group ID rejected.
    #[test]
    fn tulb_05_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 1, 16, 5),
            rec(0x01, 2, 32, 10),
        ];
        assert_eq!(
            validate_unmerged_leaf_count(&rs),
            Err(UnmergedLeafError::DuplicateGroupId { idx: 1 })
        );
    }

    /// **TULB-06** — too many entries rejected.
    #[test]
    fn tulb_06_too_many_rejected() {
        let rs: Vec<UnmergedLeafRecord> = (0..=TULB_MAX_ENTRIES)
            .map(|i| {
                let mut id = [0u8; TULB_GROUP_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                UnmergedLeafRecord { group_id: id, epoch: 1, total_leaves: 16, unmerged_count: 5 }
            })
            .collect();
        assert_eq!(
            validate_unmerged_leaf_count(&rs),
            Err(UnmergedLeafError::TooMany {
                got: TULB_MAX_ENTRIES + 1,
                max: TULB_MAX_ENTRIES,
            })
        );
    }

    /// **TULB-07** — valid accepted.
    #[test]
    fn tulb_07_valid_accepted() {
        assert_eq!(validate_unmerged_leaf_count(&valid_records()), Ok(()));
    }

    /// **TULB-08** — empty accepted.
    #[test]
    fn tulb_08_empty_accepted() {
        assert_eq!(validate_unmerged_leaf_count(&[]), Ok(()));
    }

    /// **TULB-09** — zero unmerged accepted.
    #[test]
    fn tulb_09_zero_unmerged_accepted() {
        let r = rec(0x01, 1, 16, 0);
        assert_eq!(validate_unmerged_leaf_count(&[r]), Ok(()));
    }

    /// **TULB-10** — boundary unmerged accepted.
    #[test]
    fn tulb_10_boundary_unmerged_accepted() {
        let r = rec(0x01, 1, 100, TULB_MAX_UNMERGED);
        assert_eq!(validate_unmerged_leaf_count(&[r]), Ok(()));
    }
}
