//! # CR-CHAT-03 — Ratchet tree resolution guard (Wave-64 Lane A)
//!
//! RATCHET TREE — copath resolution must be correct, R-CHAT-2.
//!
//! TreeKEM encrypts path secrets to the "resolution" of each copath node
//! (the set of unmerged leaves in that subtree). If the resolution is
//! wrong, some members cannot decrypt the update:
//!
//! * **Missing member** — skip a leaf in resolution → member can't decrypt.
//! * **Duplicate member** — encrypt to same member twice → wastes slots.
//! * **Wrong subtree** — include leaves from wrong subtree → wrong keys.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Resolution contains only leaves from the correct subtree.
//! 2. No duplicate leaves in resolution.
//! 3. Resolution is non-empty for non-blank nodes.
//! 4. Leaf indices are within tree bounds.
//! 5. Resolution size <= `RTRS_MAX_RESOLUTION`.
//! 6. Tree size <= `RTRS_MAX_TREE`.
//!
//! Tests **RTRS-01..10**. Error enum [`TreeResolutionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TREE-RESOLUTION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum resolution size.
pub const RTRS_MAX_RESOLUTION: usize = 64;

/// Maximum tree size (leaf count).
pub const RTRS_MAX_TREE: u32 = 1024;

/// All ways tree resolution can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TreeResolutionError {
    /// Leaf not in subtree.
    LeafNotInSubtree,
    /// Duplicate leaf.
    DuplicateLeaf,
    /// Empty resolution for non-blank node.
    EmptyResolution,
    /// Leaf index out of bounds.
    LeafOutOfBounds,
    /// Resolution too large.
    ResolutionTooLarge,
    /// Tree too large.
    TreeTooLarge,
}

/// `[VERIFIED]` Validate a copath node resolution.
pub fn validate_tree_resolution(
    tree_size: u32,
    subtree_start: u32,
    subtree_end: u32,
    resolution: &[u32],
) -> Result<(), TreeResolutionError> {
    if tree_size > RTRS_MAX_TREE {
        return Err(TreeResolutionError::TreeTooLarge);
    }
    if resolution.len() > RTRS_MAX_RESOLUTION {
        return Err(TreeResolutionError::ResolutionTooLarge);
    }
    if resolution.is_empty() {
        return Err(TreeResolutionError::EmptyResolution);
    }
    let mut seen = BTreeSet::new();
    for &leaf in resolution {
        if leaf >= tree_size {
            return Err(TreeResolutionError::LeafOutOfBounds);
        }
        if leaf < subtree_start || leaf >= subtree_end {
            return Err(TreeResolutionError::LeafNotInSubtree);
        }
        if !seen.insert(leaf) {
            return Err(TreeResolutionError::DuplicateLeaf);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **RTRS-01** — leaf not in subtree rejected.
    #[test]
    fn rtrs_01_not_in_subtree_rejected() {
        assert_eq!(
            validate_tree_resolution(8, 0, 4, &[0, 1, 5]),
            Err(TreeResolutionError::LeafNotInSubtree)
        );
    }

    /// **RTRS-02** — duplicate leaf rejected.
    #[test]
    fn rtrs_02_duplicate_rejected() {
        assert_eq!(
            validate_tree_resolution(8, 0, 4, &[0, 1, 1]),
            Err(TreeResolutionError::DuplicateLeaf)
        );
    }

    /// **RTRS-03** — empty resolution rejected.
    #[test]
    fn rtrs_03_empty_rejected() {
        assert_eq!(
            validate_tree_resolution(8, 0, 4, &[]),
            Err(TreeResolutionError::EmptyResolution)
        );
    }

    /// **RTRS-04** — leaf out of bounds rejected.
    #[test]
    fn rtrs_04_oob_rejected() {
        assert_eq!(
            validate_tree_resolution(4, 0, 4, &[0, 1, 10]),
            Err(TreeResolutionError::LeafOutOfBounds)
        );
    }

    /// **RTRS-05** — resolution too large rejected.
    #[test]
    fn rtrs_05_too_large_rejected() {
        let r: Vec<u32> = (0..=RTRS_MAX_RESOLUTION as u32).collect();
        assert_eq!(
            validate_tree_resolution(RTRS_MAX_TREE, 0, RTRS_MAX_TREE, &r),
            Err(TreeResolutionError::ResolutionTooLarge)
        );
    }

    /// **RTRS-06** — tree too large rejected.
    #[test]
    fn rtrs_06_tree_large_rejected() {
        assert_eq!(
            validate_tree_resolution(RTRS_MAX_TREE + 1, 0, 1, &[0]),
            Err(TreeResolutionError::TreeTooLarge)
        );
    }

    /// **RTRS-07** — valid resolution accepted.
    #[test]
    fn rtrs_07_valid_accepted() {
        assert_eq!(
            validate_tree_resolution(8, 0, 4, &[0, 1, 2, 3]),
            Ok(())
        );
    }

    /// **RTRS-08** — single leaf accepted.
    #[test]
    fn rtrs_08_single_accepted() {
        assert_eq!(
            validate_tree_resolution(4, 0, 2, &[0]),
            Ok(())
        );
    }

    /// **RTRS-09** — right subtree accepted.
    #[test]
    fn rtrs_09_right_subtree_accepted() {
        assert_eq!(
            validate_tree_resolution(8, 4, 8, &[4, 5, 6, 7]),
            Ok(())
        );
    }

    /// **RTRS-10** — boundary leaf accepted.
    #[test]
    fn rtrs_10_boundary_accepted() {
        assert_eq!(
            validate_tree_resolution(RTRS_MAX_TREE, 0, RTRS_MAX_TREE, &[0, RTRS_MAX_TREE - 1]),
            Ok(())
        );
    }
}
