//! # CR-CHAT-03 — TreeKEM blank leaf count bound guard (Wave-109 Lane A)
//!
//! RATCHET TREE — blank leaf count must be bounded.
//!
//! When members leave a group, their leaf nodes are blanked. If too
//! many leaves are blank:
//!
//! * **Tree width attack** — an adversary forces many members to leave,
//!   creating a sparse tree where the remaining members' paths are
//!   easily traceable.
//! * **Resolution cost** — blank nodes require resolution during every
//!   TreeKEM update, increasing computation from O(log n) to O(n).
//! * **Group state leakage** — the ratio of blank to occupied leaves
//!   reveals the group's churn rate and membership dynamics.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Blank count <= `BLCB_MAX_BLANK_RATIO` of total leaves.
//! 2. Total leaves must be >= `BLCB_MIN_LEAVES`.
//! 3. Total leaves must be <= `BLCB_MAX_LEAVES`.
//! 4. Total leaves must be a power of 2.
//! 5. Blank count must not exceed total leaves.
//! 6. Total trees <= `BLCB_MAX_TREES`.
//!
//! Tests **BLCB-01..10**. Error enum [`BlankLeafError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BLANK-LEAF-BOUND`

#![forbid(unsafe_code)]

/// Maximum blank leaf ratio numerator.
pub const BLCB_MAX_BLANK_NUM: usize = 1;

/// Maximum blank leaf ratio denominator.
pub const BLCB_MAX_BLANK_DEN: usize = 2;

/// Minimum leaves in a tree.
pub const BLCB_MIN_LEAVES: usize = 2;

/// Maximum leaves in a tree.
pub const BLCB_MAX_LEAVES: usize = 1024;

/// Maximum tree snapshots per batch.
pub const BLCB_MAX_TREES: usize = 256;

/// A tree snapshot with blank leaf info.
#[derive(Debug, Clone)]
pub struct TreeSnapshot {
    /// Total leaf count (must be power of 2).
    pub total_leaves: usize,
    /// Number of blank (unoccupied) leaves.
    pub blank_count: usize,
}

/// All ways blank leaf count validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlankLeafError {
    /// Too many blank leaves.
    TooManyBlank { blank: usize, total: usize, max_ratio_num: usize, max_ratio_den: usize },
    /// Too few leaves.
    TooFewLeaves { got: usize, min: usize },
    /// Too many leaves.
    TooManyLeaves { got: usize, max: usize },
    /// Not power of 2.
    NotPowerOfTwo(usize),
    /// Blank exceeds total.
    BlankExceedsTotal { blank: usize, total: usize },
    /// Too many trees.
    TooManyTrees { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM blank leaf count bounds.
pub fn validate_blank_leaf_counts(
    trees: &[TreeSnapshot],
) -> Result<(), BlankLeafError> {
    if trees.len() > BLCB_MAX_TREES {
        return Err(BlankLeafError::TooManyTrees {
            got: trees.len(),
            max: BLCB_MAX_TREES,
        });
    }
    for t in trees {
        if t.total_leaves < BLCB_MIN_LEAVES {
            return Err(BlankLeafError::TooFewLeaves {
                got: t.total_leaves,
                min: BLCB_MIN_LEAVES,
            });
        }
        if t.total_leaves > BLCB_MAX_LEAVES {
            return Err(BlankLeafError::TooManyLeaves {
                got: t.total_leaves,
                max: BLCB_MAX_LEAVES,
            });
        }
        if !t.total_leaves.is_power_of_two() {
            return Err(BlankLeafError::NotPowerOfTwo(t.total_leaves));
        }
        if t.blank_count > t.total_leaves {
            return Err(BlankLeafError::BlankExceedsTotal {
                blank: t.blank_count,
                total: t.total_leaves,
            });
        }
        let max_blank = t.total_leaves * BLCB_MAX_BLANK_NUM / BLCB_MAX_BLANK_DEN;
        if t.blank_count > max_blank {
            return Err(BlankLeafError::TooManyBlank {
                blank: t.blank_count,
                total: t.total_leaves,
                max_ratio_num: BLCB_MAX_BLANK_NUM,
                max_ratio_den: BLCB_MAX_BLANK_DEN,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(total: usize, blank: usize) -> TreeSnapshot {
        TreeSnapshot { total_leaves: total, blank_count: blank }
    }

    fn valid_trees() -> Vec<TreeSnapshot> {
        vec![
            snapshot(8, 2),
            snapshot(16, 5),
            snapshot(32, 10),
        ]
    }

    /// **BLCB-01** — too many blank rejected.
    #[test]
    fn blcb_01_too_many_blank_rejected() {
        let t = snapshot(8, 5);
        assert_eq!(
            validate_blank_leaf_counts(&[t]),
            Err(BlankLeafError::TooManyBlank {
                blank: 5,
                total: 8,
                max_ratio_num: 1,
                max_ratio_den: 2,
            })
        );
    }

    /// **BLCB-02** — too few leaves rejected.
    #[test]
    fn blcb_02_too_few_rejected() {
        let t = snapshot(1, 0);
        assert_eq!(
            validate_blank_leaf_counts(&[t]),
            Err(BlankLeafError::TooFewLeaves { got: 1, min: 2 })
        );
    }

    /// **BLCB-03** — too many leaves rejected.
    #[test]
    fn blcb_03_too_many_leaves_rejected() {
        let t = snapshot(2048, 0);
        assert_eq!(
            validate_blank_leaf_counts(&[t]),
            Err(BlankLeafError::TooManyLeaves { got: 2048, max: 1024 })
        );
    }

    /// **BLCB-04** — not power of 2 rejected.
    #[test]
    fn blcb_04_not_power_of_two_rejected() {
        let t = snapshot(6, 1);
        assert_eq!(
            validate_blank_leaf_counts(&[t]),
            Err(BlankLeafError::NotPowerOfTwo(6))
        );
    }

    /// **BLCB-05** — blank exceeds total rejected.
    #[test]
    fn blcb_05_blank_exceeds_total_rejected() {
        let t = snapshot(4, 5);
        assert_eq!(
            validate_blank_leaf_counts(&[t]),
            Err(BlankLeafError::BlankExceedsTotal { blank: 5, total: 4 })
        );
    }

    /// **BLCB-06** — too many trees rejected.
    #[test]
    fn blcb_06_too_many_rejected() {
        let ts: Vec<TreeSnapshot> = (0..=BLCB_MAX_TREES)
            .map(|_| snapshot(8, 2))
            .collect();
        assert_eq!(
            validate_blank_leaf_counts(&ts),
            Err(BlankLeafError::TooManyTrees {
                got: BLCB_MAX_TREES + 1,
                max: BLCB_MAX_TREES,
            })
        );
    }

    /// **BLCB-07** — valid accepted.
    #[test]
    fn blcb_07_valid_accepted() {
        assert_eq!(validate_blank_leaf_counts(&valid_trees()), Ok(()));
    }

    /// **BLCB-08** — empty accepted.
    #[test]
    fn blcb_08_empty_accepted() {
        assert_eq!(validate_blank_leaf_counts(&[]), Ok(()));
    }

    /// **BLCB-09** — zero blank accepted.
    #[test]
    fn blcb_09_zero_blank_accepted() {
        let t = snapshot(8, 0);
        assert_eq!(validate_blank_leaf_counts(&[t]), Ok(()));
    }

    /// **BLCB-10** — boundary blank accepted.
    #[test]
    fn blcb_10_boundary_accepted() {
        let t = snapshot(8, 4);
        assert_eq!(validate_blank_leaf_counts(&[t]), Ok(()));
    }
}
