//! # CR-CHAT-03 — TreeKEM leaf node version monotonicity guard (Wave-141 Lane A)
//!
//! RATCHET TREE — leaf node versions must increase monotonically
//! across updates; a version decrease indicates replay.
//!
//! Each TreeKEM leaf node carries a version counter that increments
//! with every key update. If versions can decrease:
//!
//! * **Replay attack** — an old leaf node with a lower version can
//!   be re-injected, reverting the tree to a prior state.
//! * **State rollback** — version non-monotonicity breaks the
//!   epoch advancement guarantee.
//! * **Audit trail break** — decreasing versions make it impossible
//!   to reconstruct the tree history from leaf node versions alone.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Version must be strictly increasing per leaf.
//! 2. Leaf ID must not be zero.
//! 3. No duplicate leaf IDs with same version.
//! 4. Version must be > 0.
//! 5. Group ID must not be zero.
//! 6. Batch size <= `TLNV_MAX_RECORDS`.
//!
//! Tests **TLNV-01..10**. Error enum [`VersionMonotonicityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * VERSION-MONOTONE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum records per batch.
pub const TLNV_MAX_RECORDS: usize = 1024;

/// Leaf ID length.
pub const TLNV_LEAF_ID_LEN: usize = 16;

/// Group ID length.
pub const TLNV_GROUP_ID_LEN: usize = 32;

/// A leaf node version record.
#[derive(Debug, Clone)]
pub struct LeafVersionRecord {
    /// Group identifier.
    pub group_id: [u8; TLNV_GROUP_ID_LEN],
    /// Leaf identifier.
    pub leaf_id: [u8; TLNV_LEAF_ID_LEN],
    /// Version counter.
    pub version: u64,
}

/// All ways version monotonicity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VersionMonotonicityError {
    /// Version decreased.
    VersionDecreased {
        /// Index.
        idx: usize,
        /// Declared version.
        got: u64,
        /// Previous version.
        prev: u64,
    },
    /// Zero leaf ID.
    ZeroLeafId(usize),
    /// Duplicate leaf+version.
    DuplicateLeafVersion {
        /// Index.
        idx: usize,
    },
    /// Zero version.
    ZeroVersion(usize),
    /// Zero group ID.
    ZeroGroupId(usize),
    /// Too many records.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate TreeKEM leaf node version monotonicity.
pub fn validate_leaf_version_monotonicity(
    records: &[LeafVersionRecord],
) -> Result<(), VersionMonotonicityError> {
    if records.len() > TLNV_MAX_RECORDS {
        return Err(VersionMonotonicityError::TooMany {
            got: records.len(),
            max: TLNV_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<([u8; TLNV_LEAF_ID_LEN], u64)> = BTreeSet::new();
    let mut last_version: BTreeSet<([u8; TLNV_LEAF_ID_LEN], u64)> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.group_id == [0u8; TLNV_GROUP_ID_LEN] {
            return Err(VersionMonotonicityError::ZeroGroupId(i));
        }
        if r.leaf_id == [0u8; TLNV_LEAF_ID_LEN] {
            return Err(VersionMonotonicityError::ZeroLeafId(i));
        }
        if r.version == 0 {
            return Err(VersionMonotonicityError::ZeroVersion(i));
        }
        if !seen.insert((r.leaf_id, r.version)) {
            return Err(VersionMonotonicityError::DuplicateLeafVersion { idx: i });
        }
        if let Some(&(_, prev_v)) = last_version.iter().find(|(lid, _)| *lid == r.leaf_id) {
            if r.version <= prev_v {
                return Err(VersionMonotonicityError::VersionDecreased {
                    idx: i,
                    got: r.version,
                    prev: prev_v,
                });
            }
        }
        last_version.replace((r.leaf_id, r.version));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid(byte: u8) -> [u8; TLNV_GROUP_ID_LEN] {
        [byte; TLNV_GROUP_ID_LEN]
    }

    fn lid(byte: u8) -> [u8; TLNV_LEAF_ID_LEN] {
        [byte; TLNV_LEAF_ID_LEN]
    }

    fn rec(gid_byte: u8, lid_byte: u8, version: u64) -> LeafVersionRecord {
        LeafVersionRecord { group_id: gid(gid_byte), leaf_id: lid(lid_byte), version }
    }

    fn valid_records() -> Vec<LeafVersionRecord> {
        vec![
            rec(0x01, 0x01, 1),
            rec(0x01, 0x01, 2),
            rec(0x01, 0x02, 1),
            rec(0x01, 0x02, 3),
        ]
    }

    /// **TLNV-01** — version decreased rejected.
    #[test]
    fn tlnv_01_version_decreased_rejected() {
        let rs = vec![
            rec(0x01, 0x01, 5),
            rec(0x01, 0x01, 3),
        ];
        assert_eq!(
            validate_leaf_version_monotonicity(&rs),
            Err(VersionMonotonicityError::VersionDecreased {
                idx: 1,
                got: 3,
                prev: 5,
            })
        );
    }

    /// **TLNV-02** — zero leaf ID rejected.
    #[test]
    fn tlnv_02_zero_leaf_rejected() {
        let r = LeafVersionRecord { group_id: gid(0x01), leaf_id: [0u8; TLNV_LEAF_ID_LEN], version: 1 };
        assert_eq!(
            validate_leaf_version_monotonicity(&[r]),
            Err(VersionMonotonicityError::ZeroLeafId(0))
        );
    }

    /// **TLNV-03** — duplicate leaf+version rejected.
    #[test]
    fn tlnv_03_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 0x01, 1),
            rec(0x01, 0x01, 1),
        ];
        assert_eq!(
            validate_leaf_version_monotonicity(&rs),
            Err(VersionMonotonicityError::DuplicateLeafVersion { idx: 1 })
        );
    }

    /// **TLNV-04** — zero version rejected.
    #[test]
    fn tlnv_04_zero_version_rejected() {
        let r = LeafVersionRecord { group_id: gid(0x01), leaf_id: lid(0x01), version: 0 };
        assert_eq!(
            validate_leaf_version_monotonicity(&[r]),
            Err(VersionMonotonicityError::ZeroVersion(0))
        );
    }

    /// **TLNV-05** — zero group ID rejected.
    #[test]
    fn tlnv_05_zero_group_rejected() {
        let r = LeafVersionRecord { group_id: [0u8; TLNV_GROUP_ID_LEN], leaf_id: lid(0x01), version: 1 };
        assert_eq!(
            validate_leaf_version_monotonicity(&[r]),
            Err(VersionMonotonicityError::ZeroGroupId(0))
        );
    }

    /// **TLNV-06** — too many records rejected.
    #[test]
    fn tlnv_06_too_many_rejected() {
        let rs: Vec<LeafVersionRecord> = (0..=TLNV_MAX_RECORDS)
            .map(|i| {
                let mut lid = [0u8; TLNV_LEAF_ID_LEN];
                let val = (i as u64) + 1;
                lid[0..8].copy_from_slice(&val.to_be_bytes());
                LeafVersionRecord { group_id: gid(0x01), leaf_id: lid, version: 1 }
            })
            .collect();
        assert_eq!(
            validate_leaf_version_monotonicity(&rs),
            Err(VersionMonotonicityError::TooMany {
                got: TLNV_MAX_RECORDS + 1,
                max: TLNV_MAX_RECORDS,
            })
        );
    }

    /// **TLNV-07** — valid accepted.
    #[test]
    fn tlnv_07_valid_accepted() {
        assert_eq!(validate_leaf_version_monotonicity(&valid_records()), Ok(()));
    }

    /// **TLNV-08** — empty accepted.
    #[test]
    fn tlnv_08_empty_accepted() {
        assert_eq!(validate_leaf_version_monotonicity(&[]), Ok(()));
    }

    /// **TLNV-09** — same version different leaf accepted.
    #[test]
    fn tlnv_09_same_version_diff_leaf() {
        let rs = vec![
            rec(0x01, 0x01, 1),
            rec(0x01, 0x02, 1),
        ];
        assert_eq!(validate_leaf_version_monotonicity(&rs), Ok(()));
    }

    /// **TLNV-10** — long increasing sequence accepted.
    #[test]
    fn tlnv_10_long_increasing_accepted() {
        let rs: Vec<LeafVersionRecord> = (0..50)
            .map(|i| rec(0x01, 0x01, (i as u64) + 1))
            .collect();
        assert_eq!(validate_leaf_version_monotonicity(&rs), Ok(()));
    }
}
