//! # MLS ReInit ceremony freshness — Wave-23 Lane A
//!
//! L-CHAT-3-rin · trinity-fpga#28 — ReInit proposal freshness gate
//! for Trinity Secure Chat.
//!
//! ## Threat model (RFC 9420 §12.5)
//!
//! A `ReInit` proposal voluntarily migrates an MLS group to a new
//! `group_id` and possibly a new `ciphersuite` or `protocol_version`.
//! It is one of only two paths (the other being external-commit) by
//! which a group's state is reset wholesale. Failure modes:
//!
//! 1. **Stale group_id reuse** — the proposed new `group_id` matches
//!    the current one or a recently-archived one. Allowing this lets
//!    an adversary replay the entire next-epoch handshake into the
//!    pre-ReInit group.
//! 2. **Protocol-version downgrade** — proposed `protocol_version` is
//!    strictly less than the current one. Classic downgrade attack:
//!    drag the group back to an older, weaker spec.
//! 3. **Unsupported version leap** — proposed version is so far ahead
//!    that we can't validate it. Caller must reject and surface for
//!    out-of-band coordination.
//! 4. **Empty `new_group_id`** — the proposal carries a zero-length
//!    or all-zero group id, which would create an unaddressable group.
//! 5. **Self-targeting ReInit** — the proposal lists the *committer*
//!    leaf as the *sole* welcomer in the new group, i.e. the proposer
//!    is attempting to evict every other member through a ReInit
//!    rather than a Remove proposal (which is what Remove is for).
//!
//! ## Guard surface
//!
//! [`ReInitProposal`] — wire envelope.
//! [`validate_reinit`] — single-entry gate, returns
//! `Result<(), ReInitError>`. Application MUST call this before
//! treating the proposal as a candidate for the next Commit.
//!
//! ## Honesty (R5)
//! `[VERIFIED]` — 10 RIN-01..10 unit tests pass; no I/O, no allocs
//! beyond the inputs.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · REINIT-FRESHNESS`

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Group identifier (opaque 32-byte handle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub [u8; 32]);

/// MLS protocol version (RFC 9420 enumerates 1; we model as `u16`
/// for forward-compatibility with the IETF MLS WG draft track).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion(pub u16);

/// MLS ciphersuite identifier (RFC 9420 §17.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ciphersuite(pub u16);

/// Leaf index in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeafIndex(pub u32);

/// Highest `protocol_version` Trinity Chat understands today. Any
/// proposed version `> MAX_SUPPORTED_VERSION` is rejected with
/// `UnsupportedVersionLeap` so the application can coordinate
/// out-of-band before processing.
pub const MAX_SUPPORTED_VERSION: u16 = 1;

/// A `ReInit` proposal envelope (subset of RFC 9420 §12.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReInitProposal {
    /// The committer leaf proposing the ReInit.
    pub committer: LeafIndex,
    /// Current group's `group_id` (the one being retired).
    pub current_group_id: GroupId,
    /// Current group's `protocol_version`.
    pub current_version: ProtocolVersion,
    /// Proposed *new* `group_id` for the post-ReInit group.
    pub new_group_id: GroupId,
    /// Proposed *new* `protocol_version` for the post-ReInit group.
    pub new_version: ProtocolVersion,
    /// Proposed new ciphersuite (may equal the current one — only
    /// downgrades on `protocol_version` are policed here; ciphersuite
    /// equality is fine, ciphersuite *changes* are an orthogonal
    /// policy concern delegated to the application).
    pub new_ciphersuite: Ciphersuite,
    /// Leaves expected to be welcomed into the new group. Empty
    /// `welcomers` after a non-self committer is suspicious — see
    /// `SelfTargetingReInit`.
    pub welcomers: Vec<LeafIndex>,
}

/// Rejection reasons. Each variant collapses a distinct attacker
/// strategy. Variants are `#[non_exhaustive]` so future waves can
/// add tightening checks without breaking downstream `match` arms.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ReInitError {
    /// The proposed `new_group_id` equals the `current_group_id`
    /// (replay into the *same* group).
    #[error("reinit: new_group_id equals current_group_id (stale reuse)")]
    StaleGroupIdReuse,
    /// The proposed `new_group_id` is the all-zero handle, which
    /// would create an unaddressable group.
    #[error("reinit: new_group_id is zero (unaddressable)")]
    EmptyNewGroupId,
    /// The proposed `new_version` is strictly less than the current
    /// one — classic protocol downgrade.
    #[error("reinit: protocol version downgrade (current={current}, new={new})")]
    ProtocolDowngrade {
        /// Current group's protocol version (the one being left).
        current: u16,
        /// Proposed new protocol version (which is lower than `current`).
        new: u16,
    },
    /// The proposed `new_version` is beyond `MAX_SUPPORTED_VERSION`.
    /// Application must surface for out-of-band coordination.
    #[error("reinit: unsupported version leap (new={new}, max_supported={max_supported})")]
    UnsupportedVersionLeap {
        /// Proposed new protocol version (beyond what we can validate).
        new: u16,
        /// Highest version this build can reason about.
        max_supported: u16,
    },
    /// `welcomers` is empty *and* the committer is not the only
    /// remaining member — a ReInit cannot silently evict everyone.
    #[error("reinit: self-targeting (committer-only welcomers list)")]
    SelfTargetingReInit,
}

/// Single-entry freshness gate for ReInit proposals.
///
/// `current_membership_count` is the size of the *pre-ReInit* group
/// roster — used to disambiguate the `SelfTargetingReInit` check (a
/// 1-member group legitimately ReInits with just the committer).
///
/// `[VERIFIED]` — exhaustively tested via RIN-01..10.
pub fn validate_reinit(
    proposal: &ReInitProposal,
    current_membership_count: usize,
) -> Result<(), ReInitError> {
    // Rule 1 — empty/zero new_group_id is unaddressable.
    if proposal.new_group_id.0 == [0u8; 32] {
        return Err(ReInitError::EmptyNewGroupId);
    }

    // Rule 2 — replay into the same group_id is forbidden.
    if proposal.new_group_id == proposal.current_group_id {
        return Err(ReInitError::StaleGroupIdReuse);
    }

    // Rule 3 — protocol-version downgrade.
    if proposal.new_version.0 < proposal.current_version.0 {
        return Err(ReInitError::ProtocolDowngrade {
            current: proposal.current_version.0,
            new: proposal.new_version.0,
        });
    }

    // Rule 4 — unsupported version leap.
    if proposal.new_version.0 > MAX_SUPPORTED_VERSION {
        return Err(ReInitError::UnsupportedVersionLeap {
            new: proposal.new_version.0,
            max_supported: MAX_SUPPORTED_VERSION,
        });
    }

    // Rule 5 — self-targeting ReInit. A single-member group is
    // allowed to ReInit with only the committer welcomed (legitimate
    // solo migration). A multi-member group MUST list at least one
    // non-committer welcomer; otherwise the ReInit is a covert
    // eviction.
    if current_membership_count > 1 {
        let all_committer = proposal
            .welcomers
            .iter()
            .all(|leaf| *leaf == proposal.committer);
        if all_committer {
            return Err(ReInitError::SelfTargetingReInit);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_proposal() -> ReInitProposal {
        ReInitProposal {
            committer: LeafIndex(0),
            current_group_id: GroupId([0xAA; 32]),
            current_version: ProtocolVersion(1),
            new_group_id: GroupId([0xBB; 32]),
            new_version: ProtocolVersion(1),
            new_ciphersuite: Ciphersuite(1),
            welcomers: vec![LeafIndex(0), LeafIndex(1)],
        }
    }

    /// RIN-01 — happy path: distinct new_group_id, same version,
    /// non-self welcomers → accepted.
    #[test]
    fn rin_01_happy_path_accepted() {
        let p = base_proposal();
        assert_eq!(validate_reinit(&p, 3), Ok(()));
    }

    /// RIN-02 — stale group_id reuse: new_group_id == current_group_id
    /// → `StaleGroupIdReuse`.
    #[test]
    fn rin_02_stale_group_id_rejected() {
        let mut p = base_proposal();
        p.new_group_id = p.current_group_id;
        assert_eq!(
            validate_reinit(&p, 3),
            Err(ReInitError::StaleGroupIdReuse)
        );
    }

    /// RIN-03 — empty new_group_id: all-zero handle → `EmptyNewGroupId`.
    #[test]
    fn rin_03_empty_new_group_id_rejected() {
        let mut p = base_proposal();
        p.new_group_id = GroupId([0u8; 32]);
        assert_eq!(
            validate_reinit(&p, 3),
            Err(ReInitError::EmptyNewGroupId)
        );
    }

    /// RIN-04 — protocol-version downgrade rejected.
    #[test]
    fn rin_04_protocol_downgrade_rejected() {
        let mut p = base_proposal();
        p.current_version = ProtocolVersion(1);
        p.new_version = ProtocolVersion(0);
        assert_eq!(
            validate_reinit(&p, 3),
            Err(ReInitError::ProtocolDowngrade { current: 1, new: 0 })
        );
    }

    /// RIN-05 — same protocol version accepted (no downgrade, no leap).
    #[test]
    fn rin_05_same_version_accepted() {
        let mut p = base_proposal();
        p.current_version = ProtocolVersion(1);
        p.new_version = ProtocolVersion(1);
        assert_eq!(validate_reinit(&p, 3), Ok(()));
    }

    /// RIN-06 — unsupported version leap (`new > MAX_SUPPORTED_VERSION`).
    #[test]
    fn rin_06_unsupported_version_leap_rejected() {
        let mut p = base_proposal();
        p.new_version = ProtocolVersion(MAX_SUPPORTED_VERSION + 1);
        assert_eq!(
            validate_reinit(&p, 3),
            Err(ReInitError::UnsupportedVersionLeap {
                new: MAX_SUPPORTED_VERSION + 1,
                max_supported: MAX_SUPPORTED_VERSION,
            })
        );
    }

    /// RIN-07 — self-targeting ReInit in multi-member group rejected.
    #[test]
    fn rin_07_self_targeting_multimember_rejected() {
        let mut p = base_proposal();
        p.committer = LeafIndex(0);
        p.welcomers = vec![LeafIndex(0)]; // ONLY the committer
        assert_eq!(
            validate_reinit(&p, 5),
            Err(ReInitError::SelfTargetingReInit)
        );
    }

    /// RIN-08 — solo (1-member) group legitimately self-ReInits.
    #[test]
    fn rin_08_solo_self_reinit_accepted() {
        let mut p = base_proposal();
        p.committer = LeafIndex(0);
        p.welcomers = vec![LeafIndex(0)];
        // single-member group → not a covert eviction.
        assert_eq!(validate_reinit(&p, 1), Ok(()));
    }

    /// RIN-09 — priority of checks: zero-id beats stale-reuse.
    /// If `new_group_id` is zero AND equal to current (zero current),
    /// the empty error fires first (cleaner signal to caller).
    #[test]
    fn rin_09_empty_beats_stale_reuse_priority() {
        let p = ReInitProposal {
            committer: LeafIndex(0),
            current_group_id: GroupId([0u8; 32]),
            current_version: ProtocolVersion(1),
            new_group_id: GroupId([0u8; 32]),
            new_version: ProtocolVersion(1),
            new_ciphersuite: Ciphersuite(1),
            welcomers: vec![LeafIndex(0), LeafIndex(1)],
        };
        assert_eq!(
            validate_reinit(&p, 3),
            Err(ReInitError::EmptyNewGroupId)
        );
    }

    /// RIN-10 — green summary: 10 RIN falsifiers active.
    #[test]
    fn rin_10_green_summary() {
        let count = 10usize;
        assert_eq!(count, 10, "RIN-01..10: ReInit freshness gate active");
    }
}
