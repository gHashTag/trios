//! # MLS external-commit forgery guard — Wave-14 Lane B
//!
//! L-CHAT-3-extern · trinity-fpga#28 — External-commit acceptance gate
//! for Trinity Secure Chat.
//!
//! ## Threat model (RFC 9420 §12.2)
//!
//! An *external commit* lets a non-member join an MLS group without a
//! Welcome — it is an atomic `Commit` that contains a `path_blob` and
//! a self-Add proposal, signed by the joiner's leaf key. This is
//! powerful but carries two failure modes:
//!
//! 1. **Forged `from_epoch`** — adversary lies about which epoch the
//!    group is currently in (replay or leap-ahead).
//! 2. **Stale Welcome rebound** — adversary replays an old, partially-
//!    used Welcome as if it were a fresh external-commit.
//! 3. **Self-Add scope-violation** — joiner attempts to add a *different*
//!    leaf instead of itself.
//! 4. **Sender / leaf mismatch** — the `sender` field claims a leaf that
//!    is already occupied by an existing member.
//!
//! ## Guard surface
//!
//! [`ExternalCommit`] — wire envelope (subset of MLS `MLSPlaintext` for
//! the external-commit content type).
//!
//! [`check_external_commit`] — single-entry gate; returns
//! `Result<(), ExternalCommitError>`. Application MUST call this before
//! handing the commit to [`super::Group::process_commit`].
//!
//! ## Honesty (R5)
//! `[VERIFIED]` — 6 EXT-01..06 unit tests pass; no I/O.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-EXTERNAL-COMMIT`

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{Epoch, GroupId, LeafIndex, Op};

/// External-commit envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCommit {
    /// Target group identifier.
    pub group_id: GroupId,
    /// Epoch the joiner *claims* the group is currently in.
    pub claimed_epoch: Epoch,
    /// Leaf the joiner is requesting (MUST be a fresh one).
    pub joining_leaf: LeafIndex,
    /// Sender field — for an external commit this MUST equal `joining_leaf`.
    pub sender: LeafIndex,
    /// Operations carried by this commit. For a well-formed external commit
    /// the ONLY proposal MUST be `Op::Add(joining_leaf)`.
    pub ops: Vec<Op>,
    /// Joiner's leaf-key public bytes — used for self-attribution.
    pub joiner_leaf_key: [u8; 32],
    /// Signature over the external-commit content (placeholder bytes; the
    /// real verifier lives behind the `openmls` feature flag).
    pub signature: Vec<u8>,
}

/// Failure cases for [`check_external_commit`].
#[derive(Debug, Error)]
pub enum ExternalCommitError {
    /// Claimed epoch does not match the group's actual epoch.
    #[error("external_commit: epoch mismatch (claimed={claimed:?}, actual={actual:?})")]
    EpochMismatch {
        /// What the commit claims.
        claimed: Epoch,
        /// What the local group state says.
        actual: Epoch,
    },
    /// Joiner is requesting a leaf that's already occupied.
    #[error("external_commit: leaf {leaf:?} already occupied")]
    LeafOccupied {
        /// Conflicting leaf.
        leaf: LeafIndex,
    },
    /// `sender` field disagrees with `joining_leaf` — only self-Add is allowed.
    #[error("external_commit: sender/leaf mismatch (sender={sender:?}, joining={joining:?})")]
    SenderMismatch {
        /// What the commit's sender field says.
        sender: LeafIndex,
        /// What the joiner requested.
        joining: LeafIndex,
    },
    /// Operations contain something other than the single self-Add.
    #[error("external_commit: ops MUST be exactly [Add(self)], got {0:?}")]
    OpsScopeViolation(Vec<Op>),
    /// Group ID does not match the local group.
    #[error("external_commit: group_id mismatch")]
    GroupIdMismatch,
    /// Signature verification failed.
    #[error("external_commit: signature verification failed")]
    InvalidSignature,
}

/// Verify an external-commit envelope before applying it.
///
/// Local state inputs:
/// - `local_group_id` — the group this peer participates in.
/// - `local_epoch` — the peer's view of the current epoch.
/// - `occupied` — slice of currently-active leaf indices.
///
/// Returns `Ok(())` iff every guard passes:
///
/// 1. `commit.group_id == local_group_id` ([`GroupIdMismatch`]).
/// 2. `commit.claimed_epoch == local_epoch` ([`EpochMismatch`]) — guards
///    against forged-epoch replay (#1) and stale Welcome rebound (#2).
/// 3. `commit.joining_leaf` is NOT in `occupied` ([`LeafOccupied`]).
/// 4. `commit.sender == commit.joining_leaf` ([`SenderMismatch`]) — guards
///    self-Add scope violation (#4).
/// 5. `commit.ops == [Add(joining_leaf)]` ([`OpsScopeViolation`]) — guards
///    self-Add scope violation (#3).
/// 6. `commit.signature` is non-empty (placeholder; real ed25519 verify
///    behind `openmls` flag) ([`InvalidSignature`]).
///
/// `[VERIFIED]` — covered by EXT-01..06.
///
/// [`GroupIdMismatch`]: ExternalCommitError::GroupIdMismatch
/// [`EpochMismatch`]: ExternalCommitError::EpochMismatch
/// [`LeafOccupied`]: ExternalCommitError::LeafOccupied
/// [`SenderMismatch`]: ExternalCommitError::SenderMismatch
/// [`OpsScopeViolation`]: ExternalCommitError::OpsScopeViolation
/// [`InvalidSignature`]: ExternalCommitError::InvalidSignature
pub fn check_external_commit(
    commit: &ExternalCommit,
    local_group_id: &GroupId,
    local_epoch: Epoch,
    occupied: &[LeafIndex],
) -> Result<(), ExternalCommitError> {
    if &commit.group_id != local_group_id {
        return Err(ExternalCommitError::GroupIdMismatch);
    }
    if commit.claimed_epoch != local_epoch {
        return Err(ExternalCommitError::EpochMismatch {
            claimed: commit.claimed_epoch,
            actual: local_epoch,
        });
    }
    if occupied.contains(&commit.joining_leaf) {
        return Err(ExternalCommitError::LeafOccupied {
            leaf: commit.joining_leaf,
        });
    }
    if commit.sender != commit.joining_leaf {
        return Err(ExternalCommitError::SenderMismatch {
            sender: commit.sender,
            joining: commit.joining_leaf,
        });
    }
    let expected = vec![Op::Add(commit.joining_leaf)];
    if commit.ops != expected {
        return Err(ExternalCommitError::OpsScopeViolation(commit.ops.clone()));
    }
    if commit.signature.is_empty() {
        return Err(ExternalCommitError::InvalidSignature);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid(b: u8) -> GroupId {
        let mut x = [0u8; 32];
        x[0] = b;
        GroupId(x)
    }

    fn good(joining: u32) -> ExternalCommit {
        ExternalCommit {
            group_id: gid(0xAA),
            claimed_epoch: Epoch(7),
            joining_leaf: LeafIndex(joining),
            sender: LeafIndex(joining),
            ops: vec![Op::Add(LeafIndex(joining))],
            joiner_leaf_key: [0x42; 32],
            signature: vec![0x99; 64],
        }
    }

    /// EXT-01 — well-formed external commit accepted.
    #[test]
    fn ext_01_accepts_wellformed() {
        let c = good(5);
        assert!(check_external_commit(&c, &gid(0xAA), Epoch(7), &[LeafIndex(0), LeafIndex(1)]).is_ok());
    }

    /// EXT-02 — epoch mismatch (forged-epoch replay) rejected.
    #[test]
    fn ext_02_epoch_mismatch_rejected() {
        let mut c = good(5);
        c.claimed_epoch = Epoch(3); // stale
        let err = check_external_commit(&c, &gid(0xAA), Epoch(7), &[]).unwrap_err();
        assert!(
            matches!(err, ExternalCommitError::EpochMismatch { .. }),
            "expected EpochMismatch, got {err:?}"
        );
    }

    /// EXT-03 — leaf already occupied: forgery rejected.
    #[test]
    fn ext_03_leaf_occupied_rejected() {
        let c = good(2);
        let err = check_external_commit(
            &c,
            &gid(0xAA),
            Epoch(7),
            &[LeafIndex(0), LeafIndex(1), LeafIndex(2)], // 2 already taken
        )
        .unwrap_err();
        assert!(matches!(err, ExternalCommitError::LeafOccupied { leaf } if leaf == LeafIndex(2)));
    }

    /// EXT-04 — sender ≠ joining_leaf (scope violation): rejected.
    #[test]
    fn ext_04_sender_mismatch_rejected() {
        let mut c = good(5);
        c.sender = LeafIndex(99); // attempting to impersonate someone else
        let err = check_external_commit(&c, &gid(0xAA), Epoch(7), &[]).unwrap_err();
        assert!(matches!(err, ExternalCommitError::SenderMismatch { .. }));
    }

    /// EXT-05 — ops MUST be exactly `[Add(self)]`; anything else is a scope violation.
    #[test]
    fn ext_05_ops_scope_violation_rejected() {
        let mut c = good(5);
        // Adversary tries to remove an existing member while joining.
        c.ops = vec![Op::Add(LeafIndex(5)), Op::Remove(LeafIndex(0))];
        let err = check_external_commit(&c, &gid(0xAA), Epoch(7), &[]).unwrap_err();
        assert!(matches!(err, ExternalCommitError::OpsScopeViolation(_)));

        // Update-only commit (no self-Add) also rejected.
        let mut c2 = good(5);
        c2.ops = vec![Op::Update];
        let err2 = check_external_commit(&c2, &gid(0xAA), Epoch(7), &[]).unwrap_err();
        assert!(matches!(err2, ExternalCommitError::OpsScopeViolation(_)));
    }

    /// EXT-06 — group_id mismatch (cross-group injection) AND empty signature
    /// (forgery without the joiner's key) BOTH rejected.
    #[test]
    fn ext_06_groupid_and_signature_guards() {
        // Cross-group injection.
        let c = good(5);
        let err = check_external_commit(&c, &gid(0xBB), Epoch(7), &[]).unwrap_err();
        assert!(matches!(err, ExternalCommitError::GroupIdMismatch));

        // Empty signature (placeholder verifier).
        let mut c2 = good(5);
        c2.signature = vec![];
        let err2 = check_external_commit(&c2, &gid(0xAA), Epoch(7), &[]).unwrap_err();
        assert!(matches!(err2, ExternalCommitError::InvalidSignature));
    }
}
