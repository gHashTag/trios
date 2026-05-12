//! # RatchetTree extension tampering — Welcome extension defense
//!
//! Wave-27 · L-CHAT-9-rtx · R-CHAT-12 · trinity-fpga#28
//!
//! ## Purpose
//!
//! RFC 9420 §12.4.3.3 lets a Welcome message carry an optional
//! `ratchet_tree` extension so the joiner can reconstruct the tree
//! without downloading it from a directory. The extension is a list of
//! `LeafNode | ParentNode` entries indexed by tree position; a
//! malicious sender can tamper with this list in five distinct ways
//! that all must be rejected:
//!
//! 1. **Empty extension** — a Welcome without a tree is fine (joiner
//!    fetches it elsewhere). But if the extension is *present*, an
//!    empty list inside it is a malformed downgrade and must reject.
//! 2. **Non-canonical leaf-count** — the number of leaf entries must
//!    match `expected_leaf_count` from the welcomer's `GroupContext`.
//!    Adversary inflates the leaf count to inject a phantom leaf.
//! 3. **Duplicate node index** — every populated index in the
//!    extension must be unique. A duplicate lets the sender swap a
//!    real leaf for a chosen attacker key at the same index after the
//!    joiner has already validated the first occurrence.
//! 4. **Out-of-range node index** — every index must be less than the
//!    tree's `node_count = 2 * expected_leaf_count - 1`. Out-of-range
//!    indices silently drop nodes from validation.
//! 5. **Unsigned leaf node** — every present leaf must have a non-empty
//!    `signature_blob` (the `LeafNodeSource = Update | Commit | Add`
//!    signature). An empty signature lets the sender insert an
//!    unauthenticated chosen key.
//!
//! All five rules trip *before* the joiner derives any key from the
//! tree.
//!
//! ## Honesty (R5)
//!
//! `[VERIFIED]` — 10 unit tests RTX-01..10 exercise every error path
//! and the canonical happy path. The validator is pure: no I/O, no
//! randomness, no time. Aligned to RFC 9420 §12.4.3.3.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · UNLINKABLE · MLS-RATCHET-TREE`

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

use trios_chat_cr_chat_00::{Error, Result};

/// Minimum acceptable leaf count in a ratchet_tree extension. A group
/// must have at least one member.
pub const RTX_MIN_LEAVES: u32 = 1;

/// What kind of tree entry the joiner sees at a given index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RatchetTreeNode {
    /// Present leaf node at the index.
    Leaf {
        /// Leaf index in the tree (0-based).
        index: u32,
        /// Signature blob over the leaf (non-empty for canonical leaves).
        signature_blob: Vec<u8>,
    },
    /// Present parent node at the index.
    Parent {
        /// Parent node index in the tree.
        index: u32,
    },
    /// Blank (absent) entry — the joiner treats it as "unknown".
    Blank,
}

/// Welcome `ratchet_tree` extension payload (RFC 9420 §12.4.3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RatchetTreeExtension {
    /// All present nodes in tree order. Length up to
    /// `2 * expected_leaf_count - 1`; `Blank` allowed for unknown slots.
    pub nodes: Vec<RatchetTreeNode>,
}

/// Welcomer-side view used to validate the extension.
#[derive(Debug, Clone)]
pub struct RatchetTreeView {
    /// Leaf count claimed by the welcomer's `GroupContext`.
    pub expected_leaf_count: u32,
}

/// Failure modes for [`validate_ratchet_tree_extension`].
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RatchetTreeExtError {
    /// Extension list is empty (downgrade attempt — joiner expected
    /// to be able to reconstruct the tree).
    EmptyExtension,
    /// Counted leaves disagree with the welcomer's `expected_leaf_count`.
    LeafCountMismatch,
    /// The same `index` appears more than once across non-Blank nodes.
    DuplicateNodeIndex,
    /// A node's `index` is outside `[0 .. 2 * expected_leaf_count - 1)`.
    NodeIndexOutOfRange,
    /// A `Leaf` node has an empty `signature_blob`.
    UnsignedLeafNode,
}

impl From<RatchetTreeExtError> for Error {
    fn from(e: RatchetTreeExtError) -> Self {
        Error::Encoding(match e {
            RatchetTreeExtError::EmptyExtension => "RTX: empty extension",
            RatchetTreeExtError::LeafCountMismatch => "RTX: leaf-count mismatch",
            RatchetTreeExtError::DuplicateNodeIndex => "RTX: duplicate node index",
            RatchetTreeExtError::NodeIndexOutOfRange => "RTX: node index out of range",
            RatchetTreeExtError::UnsignedLeafNode => "RTX: unsigned leaf node",
        })
    }
}

/// Validate a `ratchet_tree` extension against the welcomer's view.
///
/// `[VERIFIED]` — covered by RTX-01..10.
pub fn validate_ratchet_tree_extension(
    ext: &RatchetTreeExtension,
    view: &RatchetTreeView,
) -> std::result::Result<(), RatchetTreeExtError> {
    // Rule 1: present-but-empty extension is a downgrade.
    if ext.nodes.is_empty() {
        return Err(RatchetTreeExtError::EmptyExtension);
    }
    // Rule 2 (pre-pass): leaf count must match the GroupContext.
    // Counted before per-node traversal because RFC 9420 §12.4.3.3
    // requires the tree-size header to validate before any index
    // arithmetic is meaningful.
    if view.expected_leaf_count < RTX_MIN_LEAVES {
        return Err(RatchetTreeExtError::LeafCountMismatch);
    }
    let counted_leaves: u32 = ext
        .nodes
        .iter()
        .filter(|n| matches!(n, RatchetTreeNode::Leaf { .. }))
        .count() as u32;
    if counted_leaves != view.expected_leaf_count {
        return Err(RatchetTreeExtError::LeafCountMismatch);
    }
    // Rule 4: derive valid index range from validated leaf count.
    let node_count = 2u32
        .saturating_mul(view.expected_leaf_count)
        .saturating_sub(1);
    let mut seen_indices: BTreeSet<u32> = BTreeSet::new();
    for node in &ext.nodes {
        match node {
            RatchetTreeNode::Leaf { index, signature_blob } => {
                // Rule 4: index range.
                if *index >= node_count {
                    return Err(RatchetTreeExtError::NodeIndexOutOfRange);
                }
                // Rule 3: duplicate.
                if !seen_indices.insert(*index) {
                    return Err(RatchetTreeExtError::DuplicateNodeIndex);
                }
                // Rule 5: unsigned leaf.
                if signature_blob.is_empty() {
                    return Err(RatchetTreeExtError::UnsignedLeafNode);
                }
            }
            RatchetTreeNode::Parent { index } => {
                if *index >= node_count {
                    return Err(RatchetTreeExtError::NodeIndexOutOfRange);
                }
                if !seen_indices.insert(*index) {
                    return Err(RatchetTreeExtError::DuplicateNodeIndex);
                }
            }
            RatchetTreeNode::Blank => {
                // Blank entries hold no index claim — skipped.
            }
        }
    }
    Ok(())
}

/// Convenience adapter producing the crate-wide `Result` type.
pub fn validate(
    ext: &RatchetTreeExtension,
    view: &RatchetTreeView,
) -> Result<()> {
    validate_ratchet_tree_extension(ext, view).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_leaf(i: u32) -> RatchetTreeNode {
        RatchetTreeNode::Leaf {
            index: i,
            signature_blob: vec![0xAA; 64],
        }
    }

    fn parent(i: u32) -> RatchetTreeNode {
        RatchetTreeNode::Parent { index: i }
    }

    /// RTX-01 valid 3-leaf ratchet_tree extension accepted.
    #[test]
    fn rtx_01_valid_extension_accepted() {
        let ext = RatchetTreeExtension {
            nodes: vec![
                signed_leaf(0),
                parent(1),
                signed_leaf(2),
                parent(3),
                signed_leaf(4),
            ],
        };
        let view = RatchetTreeView {
            expected_leaf_count: 3,
        };
        assert_eq!(validate_ratchet_tree_extension(&ext, &view), Ok(()));
    }

    /// RTX-02 empty extension rejected — `EmptyExtension`.
    #[test]
    fn rtx_02_empty_extension_rejected() {
        let ext = RatchetTreeExtension { nodes: vec![] };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::EmptyExtension)
        );
    }

    /// RTX-03 under-count leaves rejected — `LeafCountMismatch`.
    #[test]
    fn rtx_03_under_count_leaves_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![signed_leaf(0), parent(1), signed_leaf(2)],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        // Only 2 leaves seen, expected 3.
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::LeafCountMismatch)
        );
    }

    /// RTX-04 over-count leaves rejected — `LeafCountMismatch`.
    #[test]
    fn rtx_04_over_count_leaves_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![
                signed_leaf(0),
                signed_leaf(2),
                signed_leaf(4),
                signed_leaf(6),
            ],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::LeafCountMismatch)
        );
    }

    /// RTX-05 duplicate node index rejected.
    #[test]
    fn rtx_05_duplicate_index_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![signed_leaf(0), signed_leaf(0), signed_leaf(4)],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::DuplicateNodeIndex)
        );
    }

    /// RTX-06 node index out of range rejected.
    #[test]
    fn rtx_06_index_out_of_range_rejected() {
        // expected_leaf_count = 3 ⇒ node_count = 5 ⇒ valid indices 0..5.
        let ext = RatchetTreeExtension {
            nodes: vec![signed_leaf(0), signed_leaf(2), signed_leaf(9)],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::NodeIndexOutOfRange)
        );
    }

    /// RTX-07 unsigned leaf rejected.
    #[test]
    fn rtx_07_unsigned_leaf_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![
                signed_leaf(0),
                RatchetTreeNode::Leaf {
                    index: 2,
                    signature_blob: vec![],
                },
                signed_leaf(4),
            ],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::UnsignedLeafNode)
        );
    }

    /// RTX-08 blank-only extension rejected as leaf-count mismatch.
    #[test]
    fn rtx_08_blank_only_extension_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![
                RatchetTreeNode::Blank,
                RatchetTreeNode::Blank,
                RatchetTreeNode::Blank,
            ],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        // 0 leaves seen, expected 3.
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::LeafCountMismatch)
        );
    }

    /// RTX-09 single-leaf valid group accepted (minimum legal group).
    #[test]
    fn rtx_09_single_leaf_accepted() {
        let ext = RatchetTreeExtension {
            nodes: vec![signed_leaf(0)],
        };
        let view = RatchetTreeView { expected_leaf_count: 1 };
        assert_eq!(validate_ratchet_tree_extension(&ext, &view), Ok(()));
    }

    /// RTX-10 duplicate parent index rejected.
    #[test]
    fn rtx_10_duplicate_parent_index_rejected() {
        let ext = RatchetTreeExtension {
            nodes: vec![
                signed_leaf(0),
                parent(1),
                signed_leaf(2),
                parent(1),
                signed_leaf(4),
            ],
        };
        let view = RatchetTreeView { expected_leaf_count: 3 };
        assert_eq!(
            validate_ratchet_tree_extension(&ext, &view),
            Err(RatchetTreeExtError::DuplicateNodeIndex)
        );
    }
}
