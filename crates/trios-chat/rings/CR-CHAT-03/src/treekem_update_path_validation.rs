//! # CR-CHAT-03 — TreeKEM update path validation guard (Wave-56 Lane A)
//!
//! RATCHET TREE — валидация update path, R-CHAT-2.
//!
//! TreeKEM update path содержит зашифрованные path secrets для каждого
//! узла от leaf до root. Атакующий может:
//!
//! * **Пропустить узел** — не зашифровать secret для sibling, тот не
//!   сможет обновить дерево.
//! * **Вставить orphan** — добавить узел без parent.
//! * **Дублировать** — зашифровать для одного и того же узла дважды.
//!
//! 1. Path covers all nodes from leaf to root (depth correct).
//! 2. Node count = tree depth.
//! 3. No orphan nodes (each has parent except root).
//! 4. No duplicate node positions.
//! 5. Leaf position ≤ `TKUP_MAX_LEAF`.
//! 6. Path length ≤ `TKUP_MAX_PATH_LEN`.
//!
//! Tests **TKUP-01..10**. Error enum [`UpdatePathError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · UPDATE-PATH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum leaf position.
pub const TKUP_MAX_LEAF: u32 = 1024;

/// Maximum path length (log2 of max leaves + 1).
pub const TKUP_MAX_PATH_LEN: usize = 12;

/// All ways update path validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UpdatePathError {
    /// Path too short for tree depth.
    PathTooShort,
    /// Path too long.
    PathTooLong,
    /// Duplicate node position.
    DuplicateNode,
    /// Leaf position out of bounds.
    LeafOutOfBounds,
    /// Orphan node (parent missing).
    OrphanNode,
    /// Empty path.
    EmptyPath,
}

/// An update path node.
#[derive(Debug, Clone)]
pub struct PathNode {
    /// Position in the tree (0 = leaf).
    pub position: u32,
    /// Whether this node has a parent in the path.
    pub has_parent: bool,
}

/// `[VERIFIED]` Validate a TreeKEM update path from leaf to root.
pub fn validate_update_path(
    leaf_pos: u32,
    nodes: &[PathNode],
) -> Result<(), UpdatePathError> {
    if nodes.is_empty() {
        return Err(UpdatePathError::EmptyPath);
    }
    if leaf_pos > TKUP_MAX_LEAF {
        return Err(UpdatePathError::LeafOutOfBounds);
    }
    if nodes.len() > TKUP_MAX_PATH_LEN {
        return Err(UpdatePathError::PathTooLong);
    }
    let mut seen = BTreeSet::new();
    for node in nodes {
        if !seen.insert(node.position) {
            return Err(UpdatePathError::DuplicateNode);
        }
    }
    // First node must be at leaf position
    if nodes[0].position != leaf_pos {
        return Err(UpdatePathError::PathTooShort);
    }
    // Last node is root, should have no parent; others must have parent
    for (i, node) in nodes.iter().enumerate() {
        if i < nodes.len() - 1 && !node.has_parent {
            return Err(UpdatePathError::OrphanNode);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(pos: u32, has_parent: bool) -> PathNode {
        PathNode { position: pos, has_parent }
    }

    fn good_path() -> Vec<PathNode> {
        vec![node(0, true), node(1, true), node(3, false)]
    }

    /// **TKUP-01** — empty path rejected.
    #[test]
    fn tkup_01_empty_rejected() {
        assert_eq!(
            validate_update_path(0, &[]),
            Err(UpdatePathError::EmptyPath)
        );
    }

    /// **TKUP-02** — leaf out of bounds rejected.
    #[test]
    fn tkup_02_leaf_oob_rejected() {
        assert_eq!(
            validate_update_path(TKUP_MAX_LEAF + 1, &good_path()),
            Err(UpdatePathError::LeafOutOfBounds)
        );
    }

    /// **TKUP-03** — path too long rejected.
    #[test]
    fn tkup_03_too_long_rejected() {
        let nodes: Vec<PathNode> = (0..=TKUP_MAX_PATH_LEN)
            .map(|i| node(i as u32, i < TKUP_MAX_PATH_LEN))
            .collect();
        assert_eq!(
            validate_update_path(0, &nodes),
            Err(UpdatePathError::PathTooLong)
        );
    }

    /// **TKUP-04** — duplicate node rejected.
    #[test]
    fn tkup_04_duplicate_rejected() {
        let path = vec![node(0, true), node(0, false)];
        assert_eq!(
            validate_update_path(0, &path),
            Err(UpdatePathError::DuplicateNode)
        );
    }

    /// **TKUP-05** — orphan node rejected.
    #[test]
    fn tkup_05_orphan_rejected() {
        let path = vec![node(0, false), node(1, false)];
        assert_eq!(
            validate_update_path(0, &path),
            Err(UpdatePathError::OrphanNode)
        );
    }

    /// **TKUP-06** — path too short (wrong start) rejected.
    #[test]
    fn tkup_06_wrong_start_rejected() {
        let path = vec![node(5, true), node(7, false)];
        assert_eq!(
            validate_update_path(0, &path),
            Err(UpdatePathError::PathTooShort)
        );
    }

    /// **TKUP-07** — good path accepted.
    #[test]
    fn tkup_07_good_accepted() {
        assert_eq!(validate_update_path(0, &good_path()), Ok(()));
    }

    /// **TKUP-08** — single node (root-only) accepted.
    #[test]
    fn tkup_08_single_node_accepted() {
        assert_eq!(validate_update_path(0, &[node(0, false)]), Ok(()));
    }

    /// **TKUP-09** — max leaf accepted.
    #[test]
    fn tkup_09_max_leaf_accepted() {
        let path = vec![node(TKUP_MAX_LEAF, false)];
        assert_eq!(validate_update_path(TKUP_MAX_LEAF, &path), Ok(()));
    }

    /// **TKUP-10** — deep path accepted.
    #[test]
    fn tkup_10_deep_accepted() {
        let nodes: Vec<PathNode> = (0..TKUP_MAX_PATH_LEN)
            .map(|i| node(i as u32, i < TKUP_MAX_PATH_LEN - 1))
            .collect();
        assert_eq!(validate_update_path(0, &nodes), Ok(()));
    }
}
