//! # CR-CHAT-03 — TreeKEM resolution path node uniqueness guard (Wave-148 Lane A)
//!
//! RATCHET TREE — resolution path nodes must be unique; duplicates
//! indicate tree corruption or manipulation.
//!
//! In TreeKEM, the resolution path from a blank leaf to the root
//! traverses intermediate nodes. If the same node appears twice:
//!
//! * **Tree corruption** — a malformed tree can have cycles or
//!   shared subtrees that produce duplicate resolution nodes.
//! * **Path manipulation** — an attacker injecting duplicate nodes
//!   can extend the resolution path, weakening the tree structure.
//! * **Computation waste** — resolving through duplicates causes
//!   redundant cryptographic operations.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All node IDs in resolution path must be unique.
//! 2. Path length <= `TRNU_MAX_PATH_LEN`.
//! 3. Tree ID must not be zero.
//! 4. No duplicate tree IDs.
//! 5. Node ID must not be zero.
//! 6. Batch size <= `TRNU_MAX_PATHS`.
//!
//! Tests **TRNU-01..10**. Error enum [`PathNodeUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * NODE-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum path length.
pub const TRNU_MAX_PATH_LEN: usize = 64;

/// Maximum paths per batch.
pub const TRNU_MAX_PATHS: usize = 256;

/// Tree ID length.
pub const TRNU_TREE_ID_LEN: usize = 32;

/// Node ID length.
pub const TRNU_NODE_ID_LEN: usize = 16;

/// A resolution path record.
#[derive(Debug, Clone)]
pub struct ResolutionPathRecord {
    /// Tree identifier.
    pub tree_id: [u8; TRNU_TREE_ID_LEN],
    /// Node IDs in the resolution path (leaf to root).
    pub node_ids: Vec<[u8; TRNU_NODE_ID_LEN]>,
}

/// All ways path node uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathNodeUniquenessError {
    /// Duplicate node in path.
    DuplicateNode {
        /// Path index.
        path_idx: usize,
        /// Node position in path.
        node_pos: usize,
    },
    /// Path too long.
    TooLong {
        /// Path index.
        path_idx: usize,
        /// Actual length.
        got: usize,
        /// Maximum length.
        max: usize,
    },
    /// Zero tree ID.
    ZeroTreeId(usize),
    /// Duplicate tree ID.
    DuplicateTreeId {
        /// Index.
        idx: usize,
    },
    /// Zero node ID.
    ZeroNodeId {
        /// Path index.
        path_idx: usize,
        /// Node position.
        node_pos: usize,
    },
    /// Too many paths.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate TreeKEM resolution path node uniqueness.
pub fn validate_resolution_path_uniqueness(
    paths: &[ResolutionPathRecord],
) -> Result<(), PathNodeUniquenessError> {
    if paths.len() > TRNU_MAX_PATHS {
        return Err(PathNodeUniquenessError::TooMany {
            got: paths.len(),
            max: TRNU_MAX_PATHS,
        });
    }
    let mut seen_trees: BTreeSet<[u8; TRNU_TREE_ID_LEN]> = BTreeSet::new();
    for (pi, p) in paths.iter().enumerate() {
        if p.tree_id == [0u8; TRNU_TREE_ID_LEN] {
            return Err(PathNodeUniquenessError::ZeroTreeId(pi));
        }
        if !seen_trees.insert(p.tree_id) {
            return Err(PathNodeUniquenessError::DuplicateTreeId { idx: pi });
        }
        if p.node_ids.len() > TRNU_MAX_PATH_LEN {
            return Err(PathNodeUniquenessError::TooLong {
                path_idx: pi,
                got: p.node_ids.len(),
                max: TRNU_MAX_PATH_LEN,
            });
        }
        let mut seen_nodes: BTreeSet<[u8; TRNU_NODE_ID_LEN]> = BTreeSet::new();
        for (ni, &node) in p.node_ids.iter().enumerate() {
            if node == [0u8; TRNU_NODE_ID_LEN] {
                return Err(PathNodeUniquenessError::ZeroNodeId {
                    path_idx: pi,
                    node_pos: ni,
                });
            }
            if !seen_nodes.insert(node) {
                return Err(PathNodeUniquenessError::DuplicateNode {
                    path_idx: pi,
                    node_pos: ni,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; TRNU_TREE_ID_LEN] {
        [byte; TRNU_TREE_ID_LEN]
    }

    fn nid(byte: u8) -> [u8; TRNU_NODE_ID_LEN] {
        [byte; TRNU_NODE_ID_LEN]
    }

    fn path(tree: u8, nodes: &[u8]) -> ResolutionPathRecord {
        ResolutionPathRecord {
            tree_id: tid(tree),
            node_ids: nodes.iter().map(|&b| nid(b)).collect(),
        }
    }

    fn valid_paths() -> Vec<ResolutionPathRecord> {
        vec![
            path(0x01, &[0x01, 0x02, 0x03, 0x04]),
            path(0x02, &[0x05, 0x06, 0x07]),
        ]
    }

    /// **TRNU-01** — duplicate node rejected.
    #[test]
    fn trnu_01_duplicate_node_rejected() {
        let p = path(0x01, &[0x01, 0x02, 0x01]);
        assert_eq!(
            validate_resolution_path_uniqueness(&[p]),
            Err(PathNodeUniquenessError::DuplicateNode {
                path_idx: 0,
                node_pos: 2,
            })
        );
    }

    /// **TRNU-02** — path too long rejected.
    #[test]
    fn trnu_02_too_long_rejected() {
        let nodes: Vec<u8> = (0..=TRNU_MAX_PATH_LEN as u8).collect();
        let p = path(0x01, &nodes);
        assert_eq!(
            validate_resolution_path_uniqueness(&[p]),
            Err(PathNodeUniquenessError::TooLong {
                path_idx: 0,
                got: TRNU_MAX_PATH_LEN + 1,
                max: TRNU_MAX_PATH_LEN,
            })
        );
    }

    /// **TRNU-03** — zero tree ID rejected.
    #[test]
    fn trnu_03_zero_tree_rejected() {
        let p = ResolutionPathRecord {
            tree_id: [0u8; TRNU_TREE_ID_LEN],
            node_ids: vec![nid(0x01)],
        };
        assert_eq!(
            validate_resolution_path_uniqueness(&[p]),
            Err(PathNodeUniquenessError::ZeroTreeId(0))
        );
    }

    /// **TRNU-04** — duplicate tree ID rejected.
    #[test]
    fn trnu_04_duplicate_tree_rejected() {
        let ps = vec![
            path(0x01, &[0x01, 0x02]),
            path(0x01, &[0x03, 0x04]),
        ];
        assert_eq!(
            validate_resolution_path_uniqueness(&ps),
            Err(PathNodeUniquenessError::DuplicateTreeId { idx: 1 })
        );
    }

    /// **TRNU-05** — zero node ID rejected.
    #[test]
    fn trnu_05_zero_node_rejected() {
        let p = ResolutionPathRecord {
            tree_id: tid(0x01),
            node_ids: vec![nid(0x01), [0u8; TRNU_NODE_ID_LEN], nid(0x03)],
        };
        assert_eq!(
            validate_resolution_path_uniqueness(&[p]),
            Err(PathNodeUniquenessError::ZeroNodeId {
                path_idx: 0,
                node_pos: 1,
            })
        );
    }

    /// **TRNU-06** — too many paths rejected.
    #[test]
    fn trnu_06_too_many_rejected() {
        let ps: Vec<ResolutionPathRecord> = (0..=TRNU_MAX_PATHS)
            .map(|i| {
                let mut id = [0u8; TRNU_TREE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut n = [0u8; TRNU_NODE_ID_LEN];
                n[0..8].copy_from_slice(&val.to_be_bytes());
                ResolutionPathRecord { tree_id: id, node_ids: vec![n] }
            })
            .collect();
        assert_eq!(
            validate_resolution_path_uniqueness(&ps),
            Err(PathNodeUniquenessError::TooMany {
                got: TRNU_MAX_PATHS + 1,
                max: TRNU_MAX_PATHS,
            })
        );
    }

    /// **TRNU-07** — valid accepted.
    #[test]
    fn trnu_07_valid_accepted() {
        assert_eq!(validate_resolution_path_uniqueness(&valid_paths()), Ok(()));
    }

    /// **TRNU-08** — empty accepted.
    #[test]
    fn trnu_08_empty_accepted() {
        assert_eq!(validate_resolution_path_uniqueness(&[]), Ok(()));
    }

    /// **TRNU-09** — empty path accepted.
    #[test]
    fn trnu_09_empty_path_accepted() {
        let p = ResolutionPathRecord { tree_id: tid(0x01), node_ids: vec![] };
        assert_eq!(validate_resolution_path_uniqueness(&[p]), Ok(()));
    }

    /// **TRNU-10** — long unique path accepted.
    #[test]
    fn trnu_10_long_unique_accepted() {
        let nodes: Vec<u8> = (1..33u8).collect();
        let p = path(0x01, &nodes);
        assert_eq!(validate_resolution_path_uniqueness(&[p]), Ok(()));
    }
}
