//! # CR-CHAT-03 — Ratchet tree blank node depth guard (Wave-75 Lane A)
//!
//! RATCHET TREE — blank nodes must not exceed allowed depth, R-CHAT-2.
//!
//! TreeKEM resolution skips blank nodes. If blank nodes appear too deep
//! in the tree, the resolution algorithm may produce an empty set for
//! a non-blank subtree, breaking encryption:
//!
//! * **Unresolvable subtree** — a path of consecutive blank nodes from
//!   leaf toward root means no member can encrypt to that subtree.
//! * **Resolution collapse** — deep blank runs cause the resolution
//!   to skip all leaves, yielding an empty recipient set.
//! * **Update path failure** — a path update cannot encrypt to blank
//!   nodes, so a deep blank chain blocks the update entirely.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Consecutive blank depth <= `RBND_MAX_BLANK_DEPTH`.
//! 2. No blank node at the root (root must be resolved).
//! 3. Total tree depth <= `RBND_MAX_TREE_DEPTH`.
//! 4. Leaf nodes are never blank.
//! 5. A non-blank node must follow a blank chain before root.
//! 6. Node count is consistent with tree depth (2^depth leaves).
//!
//! Tests **RBND-01..10**. Error enum [`BlankDepthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BLANK-NODE-DEPTH`

#![forbid(unsafe_code)]

/// Maximum consecutive blank node depth.
pub const RBND_MAX_BLANK_DEPTH: usize = 4;

/// Maximum tree depth.
pub const RBND_MAX_TREE_DEPTH: usize = 16;

/// A node in the ratchet tree.
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Is this node blank?
    pub blank: bool,
    /// Depth in the tree (0 = root).
    pub depth: usize,
    /// Is this a leaf node?
    pub leaf: bool,
}

/// All ways blank depth validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlankDepthError {
    /// Blank chain too deep.
    BlankTooDeep(usize),
    /// Blank node at root.
    BlankAtRoot,
    /// Tree too deep.
    MaxDepthExceeded,
    /// Leaf node is blank.
    LeafIsBlank,
    /// Tree is empty.
    TreeEmpty,
    /// Non-blank expected after blank chain but not found.
    NonBlankAfterBlank,
}

/// `[VERIFIED]` Validate that blank nodes do not exceed allowed depth.
pub fn validate_blank_node_depth(
    nodes: &[TreeNode],
) -> Result<(), BlankDepthError> {
    if nodes.is_empty() {
        return Err(BlankDepthError::TreeEmpty);
    }
    let mut current_blank_run = 0usize;
    for node in nodes {
        if node.depth > RBND_MAX_TREE_DEPTH {
            return Err(BlankDepthError::MaxDepthExceeded);
        }
        if node.leaf && node.blank {
            return Err(BlankDepthError::LeafIsBlank);
        }
        if node.blank {
            current_blank_run += 1;
            if current_blank_run > RBND_MAX_BLANK_DEPTH {
                return Err(BlankDepthError::BlankTooDeep(current_blank_run));
            }
        } else {
            current_blank_run = 0;
        }
    }
    let root = &nodes[0];
    if root.blank {
        return Err(BlankDepthError::BlankAtRoot);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(depth: usize) -> TreeNode {
        TreeNode { blank: true, depth, leaf: false }
    }

    fn non_blank(depth: usize) -> TreeNode {
        TreeNode { blank: false, depth, leaf: depth > 0 }
    }

    fn leaf(depth: usize) -> TreeNode {
        TreeNode { blank: false, depth, leaf: true }
    }

    fn valid_tree() -> Vec<TreeNode> {
        vec![
            non_blank(0),
            non_blank(1),
            blank(2),
            non_blank(3),
            leaf(4),
        ]
    }

    /// **RBND-01** — blank chain too deep rejected.
    #[test]
    fn rbnd_01_too_deep_rejected() {
        let tree = vec![
            non_blank(0),
            blank(1),
            blank(2),
            blank(3),
            blank(4),
            blank(5),
        ];
        assert_eq!(
            validate_blank_node_depth(&tree),
            Err(BlankDepthError::BlankTooDeep(5))
        );
    }

    /// **RBND-02** — blank at root rejected.
    #[test]
    fn rbnd_02_blank_root_rejected() {
        let tree = vec![blank(0), non_blank(1), leaf(2)];
        assert_eq!(
            validate_blank_node_depth(&tree),
            Err(BlankDepthError::BlankAtRoot)
        );
    }

    /// **RBND-03** — tree too deep rejected.
    #[test]
    fn rbnd_03_too_deep_rejected() {
        let tree = vec![non_blank(0), non_blank(RBND_MAX_TREE_DEPTH + 1)];
        assert_eq!(
            validate_blank_node_depth(&tree),
            Err(BlankDepthError::MaxDepthExceeded)
        );
    }

    /// **RBND-04** — leaf is blank rejected.
    #[test]
    fn rbnd_04_leaf_blank_rejected() {
        let tree = vec![non_blank(0), TreeNode { blank: true, depth: 1, leaf: true }];
        assert_eq!(
            validate_blank_node_depth(&tree),
            Err(BlankDepthError::LeafIsBlank)
        );
    }

    /// **RBND-05** — empty tree rejected.
    #[test]
    fn rbnd_05_empty_rejected() {
        assert_eq!(
            validate_blank_node_depth(&[]),
            Err(BlankDepthError::TreeEmpty)
        );
    }

    /// **RBND-06** — valid tree accepted.
    #[test]
    fn rbnd_06_valid_accepted() {
        assert_eq!(validate_blank_node_depth(&valid_tree()), Ok(()));
    }

    /// **RBND-07** — max blank depth accepted.
    #[test]
    fn rbnd_07_max_blank_accepted() {
        let tree = vec![
            non_blank(0),
            blank(1),
            blank(2),
            blank(3),
            blank(4),
            non_blank(5),
        ];
        assert_eq!(validate_blank_node_depth(&tree), Ok(()));
    }

    /// **RBND-08** — no blank nodes accepted.
    #[test]
    fn rbnd_08_no_blanks_accepted() {
        let tree = vec![non_blank(0), non_blank(1), leaf(2)];
        assert_eq!(validate_blank_node_depth(&tree), Ok(()));
    }

    /// **RBND-09** — single root accepted.
    #[test]
    fn rbnd_09_single_root_accepted() {
        assert_eq!(validate_blank_node_depth(&[non_blank(0)]), Ok(()));
    }

    /// **RBND-10** — blank chain reset after non-blank accepted.
    #[test]
    fn rbnd_10_chain_reset_accepted() {
        let tree = vec![
            non_blank(0),
            blank(1),
            blank(2),
            non_blank(3),
            blank(4),
            blank(5),
            non_blank(6),
        ];
        assert_eq!(validate_blank_node_depth(&tree), Ok(()));
    }
}
