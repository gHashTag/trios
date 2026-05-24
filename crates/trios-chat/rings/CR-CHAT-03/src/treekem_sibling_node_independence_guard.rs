//! # CR-CHAT-03 — TreeKEM sibling node independence guard (Wave-119 Lane A)
//!
//! RATCHET TREE — sibling nodes must have independent keys; shared
//! secrets between siblings enable tree-level key compromise.
//!
//! In a TreeKEM ratchet tree, each node has a key pair. If two sibling
//! nodes share the same secret key material, compromise of one sibling
//! immediately compromises the other:
//!
//! * **Lateral key compromise** — compromising one leaf gives the
//!   attacker the sibling's secret without any additional work.
//! * **Tree traversal shortcut** — independent sibling keys are
//!   required for the TreeKEM path secret derivation; shared keys
//!   break the security proof.
//! * **Parent key derivation failure** — the parent node's key is
//!   derived from both children; shared child keys produce a weak
//!   parent key.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No two sibling nodes share the same key hash.
//! 2. Key hash must not be zero.
//! 3. Node index must be unique.
//! 4. Tree level must be <= `TSNI_MAX_LEVEL`.
//! 5. No duplicate node indices.
//! 6. Total nodes <= `TSNI_MAX_NODES`.
//!
//! Tests **TSNI-01..10**. Error enum [`SiblingIndependenceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIBLING-INDEPENDENT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Key hash length.
pub const TSNI_KEY_HASH_LEN: usize = 32;

/// Maximum tree level.
pub const TSNI_MAX_LEVEL: u32 = 32;

/// Maximum nodes per batch.
pub const TSNI_MAX_NODES: usize = 1024;

/// A node in the ratchet tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Node index (position in the tree array).
    pub node_index: u32,
    /// Tree level (0 = leaf).
    pub level: u32,
    /// Hash of the node's public key.
    pub key_hash: [u8; TSNI_KEY_HASH_LEN],
}

/// All ways sibling independence validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SiblingIndependenceError {
    /// Sibling nodes share the same key hash.
    SharedKey { node_a: u32, node_b: u32 },
    /// Zero key hash.
    ZeroKey(usize),
    /// Duplicate node index.
    DuplicateIndex { idx: usize, node_index: u32 },
    /// Level exceeds maximum.
    LevelTooHigh { idx: usize, level: u32, max: u32 },
    /// Too many nodes.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM sibling node independence.
pub fn validate_sibling_independence(
    nodes: &[TreeNode],
) -> Result<(), SiblingIndependenceError> {
    if nodes.len() > TSNI_MAX_NODES {
        return Err(SiblingIndependenceError::TooMany {
            got: nodes.len(),
            max: TSNI_MAX_NODES,
        });
    }
    let mut seen_indices: BTreeSet<u32> = BTreeSet::new();
    for (i, n) in nodes.iter().enumerate() {
        if n.key_hash == [0u8; TSNI_KEY_HASH_LEN] {
            return Err(SiblingIndependenceError::ZeroKey(i));
        }
        if n.level > TSNI_MAX_LEVEL {
            return Err(SiblingIndependenceError::LevelTooHigh {
                idx: i,
                level: n.level,
                max: TSNI_MAX_LEVEL,
            });
        }
        if !seen_indices.insert(n.node_index) {
            return Err(SiblingIndependenceError::DuplicateIndex {
                idx: i,
                node_index: n.node_index,
            });
        }
    }
    let mut key_to_node: std::collections::BTreeMap<[u8; TSNI_KEY_HASH_LEN], u32> =
        std::collections::BTreeMap::new();
    for n in nodes {
        if let Some(&other) = key_to_node.get(&n.key_hash) {
            return Err(SiblingIndependenceError::SharedKey {
                node_a: other,
                node_b: n.node_index,
            });
        }
        key_to_node.insert(n.key_hash, n.node_index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; TSNI_KEY_HASH_LEN] {
        [byte; TSNI_KEY_HASH_LEN]
    }

    fn node(idx: u32, level: u32, key: u8) -> TreeNode {
        TreeNode { node_index: idx, level, key_hash: hash(key) }
    }

    fn valid_tree() -> Vec<TreeNode> {
        vec![
            node(0, 0, 0xA1),
            node(1, 0, 0xA2),
            node(2, 0, 0xA3),
            node(3, 0, 0xA4),
            node(4, 1, 0xB1),
            node(5, 1, 0xB2),
        ]
    }

    /// **TSNI-01** — shared key rejected.
    #[test]
    fn tsni_01_shared_key_rejected() {
        let ns = vec![
            node(0, 0, 0xAA),
            node(1, 0, 0xAA),
        ];
        assert_eq!(
            validate_sibling_independence(&ns),
            Err(SiblingIndependenceError::SharedKey {
                node_a: 0,
                node_b: 1,
            })
        );
    }

    /// **TSNI-02** — zero key rejected.
    #[test]
    fn tsni_02_zero_key_rejected() {
        let n = TreeNode { node_index: 0, level: 0, key_hash: [0u8; TSNI_KEY_HASH_LEN] };
        assert_eq!(
            validate_sibling_independence(&[n]),
            Err(SiblingIndependenceError::ZeroKey(0))
        );
    }

    /// **TSNI-03** — duplicate index rejected.
    #[test]
    fn tsni_03_duplicate_index_rejected() {
        let ns = vec![
            node(0, 0, 0xA1),
            node(0, 0, 0xA2),
        ];
        assert_eq!(
            validate_sibling_independence(&ns),
            Err(SiblingIndependenceError::DuplicateIndex { idx: 1, node_index: 0 })
        );
    }

    /// **TSNI-04** — level too high rejected.
    #[test]
    fn tsni_04_level_too_high_rejected() {
        let n = TreeNode { node_index: 0, level: TSNI_MAX_LEVEL + 1, key_hash: hash(0xAA) };
        assert_eq!(
            validate_sibling_independence(&[n]),
            Err(SiblingIndependenceError::LevelTooHigh {
                idx: 0,
                level: TSNI_MAX_LEVEL + 1,
                max: TSNI_MAX_LEVEL,
            })
        );
    }

    /// **TSNI-05** — too many rejected.
    #[test]
    fn tsni_05_too_many_rejected() {
        let ns: Vec<TreeNode> = (0..=TSNI_MAX_NODES)
            .map(|i| {
                let mut h = [0u8; TSNI_KEY_HASH_LEN];
                let val = (i as u64) + 1;
                h[0..8].copy_from_slice(&val.to_be_bytes());
                TreeNode { node_index: i as u32, level: 0, key_hash: h }
            })
            .collect();
        assert_eq!(
            validate_sibling_independence(&ns),
            Err(SiblingIndependenceError::TooMany {
                got: TSNI_MAX_NODES + 1,
                max: TSNI_MAX_NODES,
            })
        );
    }

    /// **TSNI-06** — valid accepted.
    #[test]
    fn tsni_06_valid_accepted() {
        assert_eq!(validate_sibling_independence(&valid_tree()), Ok(()));
    }

    /// **TSNI-07** — empty accepted.
    #[test]
    fn tsni_07_empty_accepted() {
        assert_eq!(validate_sibling_independence(&[]), Ok(()));
    }

    /// **TSNI-08** — single node accepted.
    #[test]
    fn tsni_08_single_accepted() {
        let ns = vec![node(0, 0, 0xAA)];
        assert_eq!(validate_sibling_independence(&ns), Ok(()));
    }

    /// **TSNI-09** — max level boundary accepted.
    #[test]
    fn tsni_09_max_level_accepted() {
        let ns = vec![node(0, TSNI_MAX_LEVEL, 0xAA)];
        assert_eq!(validate_sibling_independence(&ns), Ok(()));
    }

    /// **TSNI-10** — large tree accepted.
    #[test]
    fn tsni_10_large_tree_accepted() {
        let ns: Vec<TreeNode> = (0..256u32)
            .map(|i| {
                let mut h = [0u8; TSNI_KEY_HASH_LEN];
                let val = (i as u64) + 1;
                h[0..8].copy_from_slice(&val.to_be_bytes());
                TreeNode { node_index: i, level: 0, key_hash: h }
            })
            .collect();
        assert_eq!(validate_sibling_independence(&ns), Ok(()));
    }
}
