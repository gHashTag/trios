//! # CR-CHAT-03 — Unmerged leaves bound guard (Wave-70 Lane B)
//!
//! RATCHET TREE — unmerged leaves list is bounded and duplicate-free, R-CHAT-2.
//!
//! Each inner node in the TreeKEM ratchet tree maintains a list of
//! "unmerged leaves" — members added since the last path update. If
//! this list grows unbounded or contains duplicates:
//!
//! * **Unbounded growth** — memory exhaustion, DoS via repeated adds.
//! * **Duplicate entries** — same leaf encrypted-to twice, wasting slots.
//! * **Stale entries** — leaves not cleared after commit, leading to
//!   stale encryption targets.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Unmerged count <= `ULBG_MAX_UNMERGED`.
//! 2. No duplicate leaf indices.
//! 3. All leaf indices < tree size.
//! 4. No blank nodes in the list.
//! 5. List is cleared after commit (empty input = post-commit state).
//! 6. Tree size <= `ULBG_MAX_TREE`.
//!
//! Tests **ULBG-01..10**. Error enum [`UnmergedLeavesError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * UNMERGED-LEAVES-BOUND`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum unmerged leaves per node.
pub const ULBG_MAX_UNMERGED: usize = 64;

/// Maximum tree size.
pub const ULBG_MAX_TREE: u32 = 1024;

/// All ways unmerged leaves validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnmergedLeavesError {
    /// Too many unmerged leaves.
    TooManyUnmerged,
    /// Duplicate leaf index.
    DuplicateLeaf,
    /// Leaf index out of bounds.
    LeafOutOfBounds,
    /// Blank node in list (sentinel value).
    BlankNode,
    /// Tree too large.
    TreeTooLarge,
    /// List not cleared after commit (non-empty when expected empty).
    NotClearedAfterCommit,
}

/// `[VERIFIED]` Validate unmerged leaves list for an inner node.
pub fn validate_unmerged_leaves(
    tree_size: u32,
    unmerged: &[u32],
    expect_cleared: bool,
) -> Result<(), UnmergedLeavesError> {
    if tree_size > ULBG_MAX_TREE {
        return Err(UnmergedLeavesError::TreeTooLarge);
    }
    if expect_cleared && !unmerged.is_empty() {
        return Err(UnmergedLeavesError::NotClearedAfterCommit);
    }
    if unmerged.len() > ULBG_MAX_UNMERGED {
        return Err(UnmergedLeavesError::TooManyUnmerged);
    }
    let mut seen = BTreeSet::new();
    for &leaf in unmerged {
        if leaf == u32::MAX {
            return Err(UnmergedLeavesError::BlankNode);
        }
        if leaf >= tree_size {
            return Err(UnmergedLeavesError::LeafOutOfBounds);
        }
        if !seen.insert(leaf) {
            return Err(UnmergedLeavesError::DuplicateLeaf);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **ULBG-01** — too many unmerged rejected.
    #[test]
    fn ulbg_01_too_many_rejected() {
        let leaves: Vec<u32> = (0..=ULBG_MAX_UNMERGED as u32).collect();
        assert_eq!(
            validate_unmerged_leaves(256, &leaves, false),
            Err(UnmergedLeavesError::TooManyUnmerged)
        );
    }

    /// **ULBG-02** — duplicate leaf rejected.
    #[test]
    fn ulbg_02_duplicate_rejected() {
        assert_eq!(
            validate_unmerged_leaves(8, &[0, 1, 2, 1], false),
            Err(UnmergedLeavesError::DuplicateLeaf)
        );
    }

    /// **ULBG-03** — leaf out of bounds rejected.
    #[test]
    fn ulbg_03_oob_rejected() {
        assert_eq!(
            validate_unmerged_leaves(4, &[0, 1, 10], false),
            Err(UnmergedLeavesError::LeafOutOfBounds)
        );
    }

    /// **ULBG-04** — blank node rejected.
    #[test]
    fn ulbg_04_blank_rejected() {
        assert_eq!(
            validate_unmerged_leaves(8, &[0, u32::MAX], false),
            Err(UnmergedLeavesError::BlankNode)
        );
    }

    /// **ULBG-05** — tree too large rejected.
    #[test]
    fn ulbg_05_tree_large_rejected() {
        assert_eq!(
            validate_unmerged_leaves(ULBG_MAX_TREE + 1, &[0], false),
            Err(UnmergedLeavesError::TreeTooLarge)
        );
    }

    /// **ULBG-06** — not cleared after commit rejected.
    #[test]
    fn ulbg_06_not_cleared_rejected() {
        assert_eq!(
            validate_unmerged_leaves(8, &[1, 2], true),
            Err(UnmergedLeavesError::NotClearedAfterCommit)
        );
    }

    /// **ULBG-07** — valid list accepted.
    #[test]
    fn ulbg_07_valid_accepted() {
        assert_eq!(validate_unmerged_leaves(8, &[0, 1, 2, 3], false), Ok(()));
    }

    /// **ULBG-08** — empty list accepted.
    #[test]
    fn ulbg_08_empty_accepted() {
        assert_eq!(validate_unmerged_leaves(8, &[], false), Ok(()));
    }

    /// **ULBG-09** — cleared after commit accepted.
    #[test]
    fn ulbg_09_cleared_accepted() {
        assert_eq!(validate_unmerged_leaves(8, &[], true), Ok(()));
    }

    /// **ULBG-10** — max unmerged accepted.
    #[test]
    fn ulbg_10_max_accepted() {
        let leaves: Vec<u32> = (0..ULBG_MAX_UNMERGED as u32).collect();
        assert_eq!(
            validate_unmerged_leaves(ULBG_MAX_UNMERGED as u32 + 1, &leaves, false),
            Ok(())
        );
    }
}
