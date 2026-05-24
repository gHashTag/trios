//! # CR-CHAT-03 — Ratchet tree update integrity guard (Wave-84 Lane A)
//!
//! RATCHET TREE — every TreeKEM update must carry a valid parent hash,
//! R-CHAT-3.
//!
//! In TreeKEM, each node's parent hash binds the node to the tree
//! structure. Without integrity checks on updates:
//!
//! * **Orphan injection** — attacker inserts a node whose parent hash
//!   does not match the tree, creating a branch outside consensus.
//! * **Sibling swap** — attacker swaps left/right children and re-signs,
//!   causing recipients to derive different group keys.
//! * **Stale update replay** — old update from a prior epoch is replayed,
//!   rolling back the tree to a vulnerable state.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Parent hash must match computed hash.
//! 2. Node index must be valid (< `RTUI_MAX_NODES`).
//! 3. Epoch must match current.
//! 4. No duplicate node updates in same batch.
//! 5. Batch size <= `RTUI_MAX_BATCH`.
//! 6. Node signer must be a leaf member.
//!
//! Tests **RTUI-01..10**. Error enum [`TreeUpdateError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TREE-UPDATE-INTEGRITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum nodes in the tree.
pub const RTUI_MAX_NODES: u64 = 1024;

/// Maximum update batch size.
pub const RTUI_MAX_BATCH: usize = 256;

/// A tree update proposal.
#[derive(Debug, Clone)]
pub struct TreeUpdate {
    /// Node index being updated.
    pub node_index: u64,
    /// Claimed parent hash (32 bytes).
    pub parent_hash: [u8; 32],
    /// Computed parent hash (32 bytes).
    pub computed_hash: [u8; 32],
    /// Epoch of the update.
    pub epoch: u64,
    /// Signer leaf index.
    pub signer_leaf: u64,
}

/// All ways tree update validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TreeUpdateError {
    /// Parent hash mismatch.
    HashMismatch(u64),
    /// Invalid node index.
    InvalidNodeIndex(u64),
    /// Epoch mismatch.
    EpochMismatch { expected: u64, got: u64 },
    /// Duplicate node in batch.
    DuplicateNode(u64),
    /// Batch too large.
    BatchTooLarge,
    /// Signer not a leaf member.
    NonMemberSigner(u64),
}

/// `[VERIFIED]` Validate a batch of tree updates.
pub fn validate_tree_updates(
    updates: &[TreeUpdate],
    current_epoch: u64,
    member_leaves: &[u64],
) -> Result<(), TreeUpdateError> {
    if updates.len() > RTUI_MAX_BATCH {
        return Err(TreeUpdateError::BatchTooLarge);
    }
    let member_set: BTreeSet<u64> = member_leaves.iter().copied().collect();
    let mut seen = BTreeSet::new();
    for u in updates {
        if u.node_index >= RTUI_MAX_NODES {
            return Err(TreeUpdateError::InvalidNodeIndex(u.node_index));
        }
        if !seen.insert(u.node_index) {
            return Err(TreeUpdateError::DuplicateNode(u.node_index));
        }
        if u.parent_hash != u.computed_hash {
            return Err(TreeUpdateError::HashMismatch(u.node_index));
        }
        if u.epoch != current_epoch {
            return Err(TreeUpdateError::EpochMismatch {
                expected: current_epoch,
                got: u.epoch,
            });
        }
        if !member_set.contains(&u.signer_leaf) {
            return Err(TreeUpdateError::NonMemberSigner(u.signer_leaf));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(b: u8) -> [u8; 32] {
        [b; 32]
    }

    fn update(node: u64, epoch: u64, signer: u64) -> TreeUpdate {
        let h = hash((node % 256) as u8);
        TreeUpdate {
            node_index: node,
            parent_hash: h,
            computed_hash: h,
            epoch,
            signer_leaf: signer,
        }
    }

    fn members() -> Vec<u64> {
        vec![0, 2, 4, 6]
    }

    fn valid_batch() -> Vec<TreeUpdate> {
        vec![update(1, 5, 0), update(3, 5, 2)]
    }

    /// **RTUI-01** — hash mismatch rejected.
    #[test]
    fn rtui_01_hash_mismatch_rejected() {
        let mut u = update(1, 5, 0);
        u.parent_hash = hash(0xFF);
        assert_eq!(
            validate_tree_updates(&[u], 5, &members()),
            Err(TreeUpdateError::HashMismatch(1))
        );
    }

    /// **RTUI-02** — invalid node index rejected.
    #[test]
    fn rtui_02_invalid_node_rejected() {
        let u = update(RTUI_MAX_NODES, 5, 0);
        assert_eq!(
            validate_tree_updates(&[u], 5, &members()),
            Err(TreeUpdateError::InvalidNodeIndex(RTUI_MAX_NODES))
        );
    }

    /// **RTUI-03** — epoch mismatch rejected.
    #[test]
    fn rtui_03_epoch_mismatch_rejected() {
        let u = update(1, 4, 0);
        assert_eq!(
            validate_tree_updates(&[u], 5, &members()),
            Err(TreeUpdateError::EpochMismatch { expected: 5, got: 4 })
        );
    }

    /// **RTUI-04** — duplicate node rejected.
    #[test]
    fn rtui_04_duplicate_rejected() {
        let batch = vec![update(1, 5, 0), update(1, 5, 2)];
        assert_eq!(
            validate_tree_updates(&batch, 5, &members()),
            Err(TreeUpdateError::DuplicateNode(1))
        );
    }

    /// **RTUI-05** — batch too large rejected.
    #[test]
    fn rtui_05_batch_too_large_rejected() {
        let batch: Vec<TreeUpdate> = (0..=RTUI_MAX_BATCH as u64)
            .map(|i| update(i, 5, 0))
            .collect();
        assert_eq!(
            validate_tree_updates(&batch, 5, &members()),
            Err(TreeUpdateError::BatchTooLarge)
        );
    }

    /// **RTUI-06** — non-member signer rejected.
    #[test]
    fn rtui_06_non_member_rejected() {
        let u = update(1, 5, 99);
        assert_eq!(
            validate_tree_updates(&[u], 5, &members()),
            Err(TreeUpdateError::NonMemberSigner(99))
        );
    }

    /// **RTUI-07** — valid batch accepted.
    #[test]
    fn rtui_07_valid_accepted() {
        assert_eq!(validate_tree_updates(&valid_batch(), 5, &members()), Ok(()));
    }

    /// **RTUI-08** — empty batch accepted.
    #[test]
    fn rtui_08_empty_accepted() {
        assert_eq!(validate_tree_updates(&[], 5, &members()), Ok(()));
    }

    /// **RTUI-09** — single update accepted.
    #[test]
    fn rtui_09_single_accepted() {
        assert_eq!(validate_tree_updates(&[update(1, 5, 0)], 5, &members()), Ok(()));
    }

    /// **RTUI-10** — max batch size accepted.
    #[test]
    fn rtui_10_max_batch_accepted() {
        let batch: Vec<TreeUpdate> = (0..RTUI_MAX_BATCH as u64)
            .map(|i| update(i, 5, 0))
            .collect();
        assert_eq!(validate_tree_updates(&batch, 5, &members()), Ok(()));
    }
}
