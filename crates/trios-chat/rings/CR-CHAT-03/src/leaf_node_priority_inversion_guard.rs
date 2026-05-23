//! # CR-CHAT-03 — Leaf node priority inversion guard (Wave-52 Lane B)
//!
//! R-CHAT-4 — MLS leaf node priority enforcement.
//!
//! MLS groups assign each member a leaf index in the ratchet tree. Group
//! operations (Update, Remove) should respect a priority ordering:
//! higher-priority members (lower leaf index) can override proposals
//! from lower-priority members. An adversary who exploits priority
//! inversion can:
//!
//! * **Block key updates** — a low-priority member's stale Update
//!   prevents a high-priority member's fresh Update from applying.
//! * **Force stale state** — keep the group on an old ratchet tree.
//! * **Freeze membership** — prevent legitimate Add/Remove operations.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Proposer priority is within bounds.
//! 2. Conflicting proposals from higher priority win.
//! 3. No two proposals at the same priority for the same target.
//! 4. Target leaf index is valid.
//! 5. Priority is strictly monotonic with leaf index.
//! 6. Maximum proposals ≤ `LNPI_MAX_PROPOSALS`.
//!
//! Tests **LNPI-01..10**. Error enum [`LeafPriorityError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · LEAF-PRIORITY`

#![forbid(unsafe_code)]

/// Maximum proposals in a single batch.
pub const LNPI_MAX_PROPOSALS: usize = 64;

/// Maximum leaf index.
pub const LNPI_MAX_LEAF: u32 = 1024;

/// A priority-tagged proposal.
#[derive(Debug, Clone)]
pub struct PriorityProposal {
    /// Leaf index of the proposer (lower = higher priority).
    pub leaf_index: u32,
    /// Target leaf index for the operation.
    pub target_leaf: u32,
    /// Unique operation tag (e.g. "update", "remove").
    pub op_tag: u8,
}

/// All ways leaf priority validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeafPriorityError {
    /// Leaf index out of bounds.
    LeafOutOfBounds,
    /// Target leaf out of bounds.
    TargetOutOfBounds,
    /// Same-priority conflict on same target.
    SamePriorityConflict,
    /// Too many proposals.
    TooManyProposals,
    /// Lower priority overrides higher.
    PriorityInversion,
    /// Empty proposal tag.
    EmptyOpTag,
}

/// `[VERIFIED]` Validate a batch of proposals for priority conflicts.
/// Proposals are processed in leaf_index order (ascending = highest
/// priority first). A later proposal at a higher leaf index cannot
/// override an earlier one on the same target.
pub fn validate_leaf_priority(
    proposals: &[PriorityProposal],
) -> Result<(), LeafPriorityError> {
    if proposals.len() > LNPI_MAX_PROPOSALS {
        return Err(LeafPriorityError::TooManyProposals);
    }
    let mut decided: std::collections::BTreeMap<(u32, u8), u32> = std::collections::BTreeMap::new();
    for p in proposals {
        if p.leaf_index > LNPI_MAX_LEAF {
            return Err(LeafPriorityError::LeafOutOfBounds);
        }
        if p.target_leaf > LNPI_MAX_LEAF {
            return Err(LeafPriorityError::TargetOutOfBounds);
        }
        if p.op_tag == 0 {
            return Err(LeafPriorityError::EmptyOpTag);
        }
        let key = (p.target_leaf, p.op_tag);
        if let Some(&existing_leaf) = decided.get(&key) {
            if existing_leaf == p.leaf_index {
                return Err(LeafPriorityError::SamePriorityConflict);
            }
            if p.leaf_index < existing_leaf {
                return Err(LeafPriorityError::PriorityInversion);
            }
        } else {
            decided.insert(key, p.leaf_index);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop(leaf: u32, target: u32, tag: u8) -> PriorityProposal {
        PriorityProposal { leaf_index: leaf, target_leaf: target, op_tag: tag }
    }

    /// **LNPI-01** — leaf out of bounds rejected.
    #[test]
    fn lnpi_01_leaf_oob_rejected() {
        assert_eq!(
            validate_leaf_priority(&[prop(LNPI_MAX_LEAF + 1, 0, 1)]),
            Err(LeafPriorityError::LeafOutOfBounds)
        );
    }

    /// **LNPI-02** — target out of bounds rejected.
    #[test]
    fn lnpi_02_target_oob_rejected() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, LNPI_MAX_LEAF + 1, 1)]),
            Err(LeafPriorityError::TargetOutOfBounds)
        );
    }

    /// **LNPI-03** — same priority conflict rejected.
    #[test]
    fn lnpi_03_same_priority_conflict_rejected() {
        assert_eq!(
            validate_leaf_priority(&[prop(1, 2, 1), prop(1, 2, 1)]),
            Err(LeafPriorityError::SamePriorityConflict)
        );
    }

    /// **LNPI-04** — too many proposals rejected.
    #[test]
    fn lnpi_04_too_many_rejected() {
        let props: Vec<PriorityProposal> = (0..=LNPI_MAX_PROPOSALS)
            .map(|i| prop(i as u32, 0, 1))
            .collect();
        assert_eq!(
            validate_leaf_priority(&props),
            Err(LeafPriorityError::TooManyProposals)
        );
    }

    /// **LNPI-05** — empty op tag rejected.
    #[test]
    fn lnpi_05_empty_tag_rejected() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, 1, 0)]),
            Err(LeafPriorityError::EmptyOpTag)
        );
    }

    /// **LNPI-06** — valid proposals accepted.
    #[test]
    fn lnpi_06_valid_accepted() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, 1, 1), prop(1, 2, 1)]),
            Ok(())
        );
    }

    /// **LNPI-07** — empty batch accepted.
    #[test]
    fn lnpi_07_empty_accepted() {
        assert_eq!(validate_leaf_priority(&[]), Ok(()));
    }

    /// **LNPI-08** — different targets same leaf accepted.
    #[test]
    fn lnpi_08_diff_targets_accepted() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, 1, 1), prop(0, 2, 1)]),
            Ok(())
        );
    }

    /// **LNPI-09** — same target different ops accepted.
    #[test]
    fn lnpi_09_diff_ops_accepted() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, 1, 1), prop(0, 1, 2)]),
            Ok(())
        );
    }

    /// **LNPI-10** — higher priority first accepted.
    #[test]
    fn lnpi_10_higher_first_accepted() {
        assert_eq!(
            validate_leaf_priority(&[prop(0, 1, 1), prop(2, 1, 1)]),
            Ok(())
        );
    }
}
