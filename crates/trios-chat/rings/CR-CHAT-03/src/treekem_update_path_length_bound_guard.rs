//! # CR-CHAT-03 — TreeKEM update path length bound guard (Wave-144 Lane A)
//!
//! RATCHET TREE — update paths must not exceed a maximum length;
//! excessively long paths indicate tree manipulation or corruption.
//!
//! In TreeKEM, each member's update path from leaf to root should
//! have length bounded by `log2(n)` where n is the number of leaves.
//! If an update path exceeds the expected bound:
//!
//! * **Tree manipulation** — an attacker may have injected extra
//!   nodes to create a longer path, weakening the tree structure.
//! * **Corruption** — malformed tree data can produce paths that
//!   traverse more nodes than expected.
//! * **Resource exhaustion** — processing excessively long paths
//!   consumes unbounded CPU and memory.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Path length <= `TUPL_MAX_PATH_LEN`.
//! 2. Path length >= `TUPL_MIN_PATH_LEN`.
//! 3. Tree ID must not be zero.
//! 4. No duplicate tree IDs.
//! 5. Leaf index must be < total leaves.
//! 6. Batch size <= `TUPL_MAX_PATHS`.
//!
//! Tests **TUPL-01..10**. Error enum [`PathLengthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum path length.
pub const TUPL_MAX_PATH_LEN: usize = 64;

/// Minimum path length.
pub const TUPL_MIN_PATH_LEN: usize = 1;

/// Maximum paths per batch.
pub const TUPL_MAX_PATHS: usize = 512;

/// Tree ID length.
pub const TUPL_TREE_ID_LEN: usize = 32;

/// An update path length record.
#[derive(Debug, Clone)]
pub struct UpdatePathRecord {
    /// Tree identifier.
    pub tree_id: [u8; TUPL_TREE_ID_LEN],
    /// Total number of leaves in the tree.
    pub total_leaves: usize,
    /// Leaf index of the updating member.
    pub leaf_index: u32,
    /// Actual path length.
    pub path_length: usize,
}

/// All ways path length validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathLengthError {
    /// Path too long.
    TooLong { idx: usize, got: usize, max: usize },
    /// Path too short.
    TooShort { idx: usize, got: usize, min: usize },
    /// Zero tree ID.
    ZeroTreeId(usize),
    /// Duplicate tree ID.
    DuplicateTreeId { idx: usize },
    /// Leaf index out of range.
    LeafOutOfRange { idx: usize, leaf: u32, total: usize },
    /// Too many paths.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM update path length bound.
pub fn validate_path_length(
    paths: &[UpdatePathRecord],
) -> Result<(), PathLengthError> {
    if paths.len() > TUPL_MAX_PATHS {
        return Err(PathLengthError::TooMany {
            got: paths.len(),
            max: TUPL_MAX_PATHS,
        });
    }
    let mut seen: BTreeSet<[u8; TUPL_TREE_ID_LEN]> = BTreeSet::new();
    for (i, p) in paths.iter().enumerate() {
        if p.tree_id == [0u8; TUPL_TREE_ID_LEN] {
            return Err(PathLengthError::ZeroTreeId(i));
        }
        if !seen.insert(p.tree_id) {
            return Err(PathLengthError::DuplicateTreeId { idx: i });
        }
        if (p.leaf_index as usize) >= p.total_leaves {
            return Err(PathLengthError::LeafOutOfRange {
                idx: i,
                leaf: p.leaf_index,
                total: p.total_leaves,
            });
        }
        if p.path_length < TUPL_MIN_PATH_LEN {
            return Err(PathLengthError::TooShort {
                idx: i,
                got: p.path_length,
                min: TUPL_MIN_PATH_LEN,
            });
        }
        if p.path_length > TUPL_MAX_PATH_LEN {
            return Err(PathLengthError::TooLong {
                idx: i,
                got: p.path_length,
                max: TUPL_MAX_PATH_LEN,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; TUPL_TREE_ID_LEN] {
        [byte; TUPL_TREE_ID_LEN]
    }

    fn path(id: u8, total: usize, leaf: u32, len: usize) -> UpdatePathRecord {
        UpdatePathRecord { tree_id: tid(id), total_leaves: total, leaf_index: leaf, path_length: len }
    }

    fn valid_paths() -> Vec<UpdatePathRecord> {
        vec![
            path(0x01, 16, 3, 4),
            path(0x02, 32, 10, 5),
        ]
    }

    /// **TUPL-01** — too long rejected.
    #[test]
    fn tupl_01_too_long_rejected() {
        let p = path(0x01, 16, 0, TUPL_MAX_PATH_LEN + 1);
        assert_eq!(
            validate_path_length(&[p]),
            Err(PathLengthError::TooLong {
                idx: 0,
                got: TUPL_MAX_PATH_LEN + 1,
                max: TUPL_MAX_PATH_LEN,
            })
        );
    }

    /// **TUPL-02** — too short rejected.
    #[test]
    fn tupl_02_too_short_rejected() {
        let p = path(0x01, 16, 0, TUPL_MIN_PATH_LEN - 1);
        assert_eq!(
            validate_path_length(&[p]),
            Err(PathLengthError::TooShort {
                idx: 0,
                got: TUPL_MIN_PATH_LEN - 1,
                min: TUPL_MIN_PATH_LEN,
            })
        );
    }

    /// **TUPL-03** — zero tree ID rejected.
    #[test]
    fn tupl_03_zero_tree_rejected() {
        let p = UpdatePathRecord {
            tree_id: [0u8; TUPL_TREE_ID_LEN],
            total_leaves: 16,
            leaf_index: 0,
            path_length: 4,
        };
        assert_eq!(
            validate_path_length(&[p]),
            Err(PathLengthError::ZeroTreeId(0))
        );
    }

    /// **TUPL-04** — duplicate tree ID rejected.
    #[test]
    fn tupl_04_duplicate_rejected() {
        let ps = vec![
            path(0x01, 16, 0, 4),
            path(0x01, 32, 1, 5),
        ];
        assert_eq!(
            validate_path_length(&ps),
            Err(PathLengthError::DuplicateTreeId { idx: 1 })
        );
    }

    /// **TUPL-05** — leaf out of range rejected.
    #[test]
    fn tupl_05_leaf_out_of_range_rejected() {
        let p = path(0x01, 8, 10, 3);
        assert_eq!(
            validate_path_length(&[p]),
            Err(PathLengthError::LeafOutOfRange {
                idx: 0,
                leaf: 10,
                total: 8,
            })
        );
    }

    /// **TUPL-06** — too many paths rejected.
    #[test]
    fn tupl_06_too_many_rejected() {
        let ps: Vec<UpdatePathRecord> = (0..=TUPL_MAX_PATHS)
            .map(|i| {
                let mut id = [0u8; TUPL_TREE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                UpdatePathRecord { tree_id: id, total_leaves: 16, leaf_index: 0, path_length: 4 }
            })
            .collect();
        assert_eq!(
            validate_path_length(&ps),
            Err(PathLengthError::TooMany {
                got: TUPL_MAX_PATHS + 1,
                max: TUPL_MAX_PATHS,
            })
        );
    }

    /// **TUPL-07** — valid accepted.
    #[test]
    fn tupl_07_valid_accepted() {
        assert_eq!(validate_path_length(&valid_paths()), Ok(()));
    }

    /// **TUPL-08** — empty accepted.
    #[test]
    fn tupl_08_empty_accepted() {
        assert_eq!(validate_path_length(&[]), Ok(()));
    }

    /// **TUPL-09** — boundary length accepted.
    #[test]
    fn tupl_09_boundary_accepted() {
        let p = path(0x01, 16, 0, TUPL_MAX_PATH_LEN);
        assert_eq!(validate_path_length(&[p]), Ok(()));
    }

    /// **TUPL-10** — many valid paths accepted.
    #[test]
    fn tupl_10_many_valid_accepted() {
        let ps: Vec<UpdatePathRecord> = (0..20u8)
            .map(|i| path(i + 1, 16 + (i as usize), (i as u32) % 8, 3 + (i as usize) % 5))
            .collect();
        assert_eq!(validate_path_length(&ps), Ok(()));
    }
}
