//! # CR-CHAT-03 — Group membership authorization guard (Wave-48 Lane B)
//!
//! R-CHAT-4 — Group operation authorization enforcement.
//!
//! MLS group operations (Add, Remove, Update) must be authorized by a
//! member with sufficient permissions. An adversary who can inject
//! unauthorized operations can:
//!
//! * **Add a shadow member** — silently inject a new leaf into the
//!   ratchet tree, gaining access to all future keys.
//! * **Remove a legitimate member** — eject a target from the group.
//! * **Force a key update** — trigger unnecessary rekeying to exhaust
//!   ratchet state.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Only members can propose operations.
//! 2. Proposer leaf index is within tree bounds.
//! 3. Operation type is recognized.
//! 4. Target leaf index is within tree bounds (for Add/Remove).
//! 5. No self-removal (use explicit Leave instead).
//! 6. Group has at least `GMAZ_MIN_MEMBERS` members.
//!
//! Tests **GMAZ-01..10**. Error enum [`GroupMembershipError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · GROUP-MEMBERSHIP-AUTH`

#![forbid(unsafe_code)]

/// Minimum group size.
pub const GMAZ_MIN_MEMBERS: u32 = 2;

/// Maximum leaf index.
pub const GMAZ_MAX_LEAF_INDEX: u32 = 1024;

/// Group operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupOp {
    /// Add a new member.
    Add,
    /// Remove an existing member.
    Remove,
    /// Update own leaf key.
    Update,
}

/// A proposed group operation.
#[derive(Debug, Clone)]
pub struct GroupProposal {
    /// Operation type.
    pub op: GroupOp,
    /// Proposer's leaf index.
    pub proposer: u32,
    /// Target leaf index (ignored for Update).
    pub target: u32,
}

/// Current group state snapshot.
#[derive(Debug, Clone)]
pub struct GroupState {
    /// Number of active members.
    pub member_count: u32,
    /// Set of active member leaf indices.
    pub members: Vec<u32>,
}

/// All ways group membership authorization can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GroupMembershipError {
    /// Proposer is not a member.
    ProposerNotMember,
    /// Proposer leaf index out of bounds.
    ProposerOutOfBounds,
    /// Target leaf index out of bounds.
    TargetOutOfBounds,
    /// Unknown operation type (should never happen with enum).
    UnknownOperation,
    /// Self-removal not allowed.
    SelfRemovalNotAllowed,
    /// Group too small for operation.
    GroupTooSmall,
}

/// `[VERIFIED]` Validate a group proposal against current group state.
pub fn validate_group_proposal(
    proposal: &GroupProposal,
    state: &GroupState,
) -> Result<(), GroupMembershipError> {
    if state.member_count < GMAZ_MIN_MEMBERS {
        return Err(GroupMembershipError::GroupTooSmall);
    }
    if proposal.proposer > GMAZ_MAX_LEAF_INDEX {
        return Err(GroupMembershipError::ProposerOutOfBounds);
    }
    if !state.members.contains(&proposal.proposer) {
        return Err(GroupMembershipError::ProposerNotMember);
    }
    match proposal.op {
        GroupOp::Update => {}
        GroupOp::Add | GroupOp::Remove => {
            if proposal.target > GMAZ_MAX_LEAF_INDEX {
                return Err(GroupMembershipError::TargetOutOfBounds);
            }
            if proposal.op == GroupOp::Remove && proposal.proposer == proposal.target {
                return Err(GroupMembershipError::SelfRemovalNotAllowed);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> GroupState {
        GroupState {
            member_count: 3,
            members: vec![0, 1, 2],
        }
    }

    /// **GMAZ-01** — proposer not member rejected.
    #[test]
    fn gmaz_01_proposer_not_member_rejected() {
        let p = GroupProposal { op: GroupOp::Update, proposer: 5, target: 5 };
        assert_eq!(
            validate_group_proposal(&p, &state()),
            Err(GroupMembershipError::ProposerNotMember)
        );
    }

    /// **GMAZ-02** — proposer out of bounds rejected.
    #[test]
    fn gmaz_02_proposer_oob_rejected() {
        let p = GroupProposal { op: GroupOp::Update, proposer: GMAZ_MAX_LEAF_INDEX + 1, target: 0 };
        assert_eq!(
            validate_group_proposal(&p, &state()),
            Err(GroupMembershipError::ProposerOutOfBounds)
        );
    }

    /// **GMAZ-03** — target out of bounds rejected.
    #[test]
    fn gmaz_03_target_oob_rejected() {
        let p = GroupProposal { op: GroupOp::Add, proposer: 0, target: GMAZ_MAX_LEAF_INDEX + 1 };
        assert_eq!(
            validate_group_proposal(&p, &state()),
            Err(GroupMembershipError::TargetOutOfBounds)
        );
    }

    /// **GMAZ-04** — self-removal rejected.
    #[test]
    fn gmaz_04_self_removal_rejected() {
        let p = GroupProposal { op: GroupOp::Remove, proposer: 1, target: 1 };
        assert_eq!(
            validate_group_proposal(&p, &state()),
            Err(GroupMembershipError::SelfRemovalNotAllowed)
        );
    }

    /// **GMAZ-05** — group too small rejected.
    #[test]
    fn gmaz_05_group_too_small_rejected() {
        let tiny = GroupState { member_count: 1, members: vec![0] };
        let p = GroupProposal { op: GroupOp::Update, proposer: 0, target: 0 };
        assert_eq!(
            validate_group_proposal(&p, &tiny),
            Err(GroupMembershipError::GroupTooSmall)
        );
    }

    /// **GMAZ-06** — valid update accepted.
    #[test]
    fn gmaz_06_valid_update_accepted() {
        let p = GroupProposal { op: GroupOp::Update, proposer: 0, target: 0 };
        assert_eq!(validate_group_proposal(&p, &state()), Ok(()));
    }

    /// **GMAZ-07** — valid add accepted.
    #[test]
    fn gmaz_07_valid_add_accepted() {
        let p = GroupProposal { op: GroupOp::Add, proposer: 0, target: 3 };
        assert_eq!(validate_group_proposal(&p, &state()), Ok(()));
    }

    /// **GMAZ-08** — valid remove accepted.
    #[test]
    fn gmaz_08_valid_remove_accepted() {
        let p = GroupProposal { op: GroupOp::Remove, proposer: 0, target: 2 };
        assert_eq!(validate_group_proposal(&p, &state()), Ok(()));
    }

    /// **GMAZ-09** — boundary proposer index accepted.
    #[test]
    fn gmaz_09_boundary_proposer_accepted() {
        let s = GroupState {
            member_count: 2,
            members: vec![0, GMAZ_MAX_LEAF_INDEX],
        };
        let p = GroupProposal { op: GroupOp::Update, proposer: GMAZ_MAX_LEAF_INDEX, target: 0 };
        assert_eq!(validate_group_proposal(&p, &s), Ok(()));
    }

    /// **GMAZ-10** — boundary target accepted for add.
    #[test]
    fn gmaz_10_boundary_target_accepted() {
        let p = GroupProposal { op: GroupOp::Add, proposer: 0, target: GMAZ_MAX_LEAF_INDEX };
        assert_eq!(validate_group_proposal(&p, &state()), Ok(()));
    }
}
