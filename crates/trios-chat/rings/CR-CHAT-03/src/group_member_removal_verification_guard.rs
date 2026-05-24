//! # CR-CHAT-03 — Group member removal verification guard (Wave-95 Lane A)
//!
//! RATCHET TREE — removed members must be fully evicted, R-CHAT-3.
//!
//! When a member is removed from the group:
//!
//! * **Incomplete blanking** — the leaf is removed but path secrets
//!   above it are not rotated, so the removed member can still derive
//!   group keys from cached path secrets.
//! * **Stale leaf key** — the removed member's public key remains in
//!   the tree, allowing them to receive future commits.
//! * **Unupdated parent** — parent nodes along the direct path still
//!   use secret material the removed member can derive.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Removed leaf must be marked as blanked.
//! 2. All ancestors must have updated secret hashes.
//! 3. Removed member count must match expected.
//! 4. No removed member retains an active leaf key.
//! 5. Removal epoch must be > 0.
//! 6. Maximum removals <= `GMRV_MAX_REMOVALS`.
//!
//! Tests **GMRV-01..10**. Error enum [`RemovalError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MEMBER-REMOVAL`

#![forbid(unsafe_code)]

/// Maximum removals per batch.
pub const GMRV_MAX_REMOVALS: usize = 256;

/// A member removal record.
#[derive(Debug, Clone)]
pub struct MemberRemoval {
    /// Leaf index of the removed member.
    pub leaf_index: u32,
    /// Whether the leaf has been blanked.
    pub leaf_blanked: bool,
    /// Whether the leaf key has been cleared.
    pub key_cleared: bool,
    /// Whether ancestor path secrets were updated.
    pub ancestors_updated: bool,
    /// Epoch at which the removal occurred.
    pub epoch: u64,
}

/// All ways removal validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RemovalError {
    /// Leaf not blanked.
    LeafNotBlanked(u32),
    /// Ancestors not updated.
    AncestorsNotUpdated(u32),
    /// Count mismatch.
    CountMismatch { expected: usize, got: usize },
    /// Key not cleared.
    KeyNotCleared(u32),
    /// Zero epoch.
    ZeroEpoch(u32),
    /// Too many removals.
    TooManyRemovals,
}

/// `[VERIFIED]` Validate group member removal verification.
pub fn validate_member_removals(
    removals: &[MemberRemoval],
    expected_count: usize,
) -> Result<(), RemovalError> {
    if removals.len() > GMRV_MAX_REMOVALS {
        return Err(RemovalError::TooManyRemovals);
    }
    if removals.len() != expected_count {
        return Err(RemovalError::CountMismatch {
            expected: expected_count,
            got: removals.len(),
        });
    }
    for r in removals {
        if r.epoch == 0 {
            return Err(RemovalError::ZeroEpoch(r.leaf_index));
        }
        if !r.leaf_blanked {
            return Err(RemovalError::LeafNotBlanked(r.leaf_index));
        }
        if !r.key_cleared {
            return Err(RemovalError::KeyNotCleared(r.leaf_index));
        }
        if !r.ancestors_updated {
            return Err(RemovalError::AncestorsNotUpdated(r.leaf_index));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn removal(leaf: u32, blanked: bool, key_cleared: bool, ancestors: bool) -> MemberRemoval {
        MemberRemoval {
            leaf_index: leaf,
            leaf_blanked: blanked,
            key_cleared,
            ancestors_updated: ancestors,
            epoch: 1,
        }
    }

    fn valid_removals() -> Vec<MemberRemoval> {
        vec![removal(1, true, true, true), removal(2, true, true, true)]
    }

    /// **GMRV-01** — leaf not blanked rejected.
    #[test]
    fn gmrv_01_leaf_not_blanked_rejected() {
        let r = removal(1, false, true, true);
        assert_eq!(
            validate_member_removals(&[r], 1),
            Err(RemovalError::LeafNotBlanked(1))
        );
    }

    /// **GMRV-02** — ancestors not updated rejected.
    #[test]
    fn gmrv_02_ancestors_not_updated_rejected() {
        let r = removal(1, true, true, false);
        assert_eq!(
            validate_member_removals(&[r], 1),
            Err(RemovalError::AncestorsNotUpdated(1))
        );
    }

    /// **GMRV-03** — count mismatch rejected.
    #[test]
    fn gmrv_03_count_mismatch_rejected() {
        assert_eq!(
            validate_member_removals(&valid_removals(), 3),
            Err(RemovalError::CountMismatch { expected: 3, got: 2 })
        );
    }

    /// **GMRV-04** — key not cleared rejected.
    #[test]
    fn gmrv_04_key_not_cleared_rejected() {
        let r = removal(1, true, false, true);
        assert_eq!(
            validate_member_removals(&[r], 1),
            Err(RemovalError::KeyNotCleared(1))
        );
    }

    /// **GMRV-05** — zero epoch rejected.
    #[test]
    fn gmrv_05_zero_epoch_rejected() {
        let mut r = removal(1, true, true, true);
        r.epoch = 0;
        assert_eq!(
            validate_member_removals(&[r], 1),
            Err(RemovalError::ZeroEpoch(1))
        );
    }

    /// **GMRV-06** — too many removals rejected.
    #[test]
    fn gmrv_06_too_many_rejected() {
        let rs: Vec<MemberRemoval> = (0..=GMRV_MAX_REMOVALS as u32)
            .map(|i| removal(i, true, true, true))
            .collect();
        assert_eq!(
            validate_member_removals(&rs, rs.len()),
            Err(RemovalError::TooManyRemovals)
        );
    }

    /// **GMRV-07** — valid removals accepted.
    #[test]
    fn gmrv_07_valid_accepted() {
        assert_eq!(validate_member_removals(&valid_removals(), 2), Ok(()));
    }

    /// **GMRV-08** — empty accepted.
    #[test]
    fn gmrv_08_empty_accepted() {
        assert_eq!(validate_member_removals(&[], 0), Ok(()));
    }

    /// **GMRV-09** — single accepted.
    #[test]
    fn gmrv_09_single_accepted() {
        assert_eq!(validate_member_removals(&[removal(1, true, true, true)], 1), Ok(()));
    }

    /// **GMRV-10** — max removals boundary accepted.
    #[test]
    fn gmrv_10_max_boundary_accepted() {
        let rs: Vec<MemberRemoval> = (0..GMRV_MAX_REMOVALS as u32)
            .map(|i| removal(i, true, true, true))
            .collect();
        assert_eq!(validate_member_removals(&rs, rs.len()), Ok(()));
    }
}
