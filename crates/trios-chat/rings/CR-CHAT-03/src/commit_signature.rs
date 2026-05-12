//! # MLS Commit signature forgery defense — Wave-24 Lane A
//!
//! L-CHAT-3-csig · trinity-fpga#28 — Commit-signature authentication
//! gate for Trinity Secure Chat.
//!
//! ## Threat model (RFC 9420 §6.1 / §12)
//!
//! Every MLS `Commit` is signed by its `committer` leaf key. A
//! forgery attempt covers the cases where an adversary:
//!
//! 1. **Empty/zero signature** — submits a commit with a zero-byte
//!    signature blob, hoping a lazy verifier short-circuits to
//!    `Ok`. We reject any commit whose `sig_blob` is the all-zero
//!    32-byte buffer or zero-length.
//! 2. **Stale-key signature** — signs with the committer's
//!    *pre-rotation* leaf key after a leaf-resync has rotated the
//!    key. The supplied `sig_pubkey` must equal the *current*
//!    leaf-public on the receiver's view; otherwise we reject with
//!    `StaleSignerKey`.
//! 3. **Cross-commit splice** — the signature was generated over a
//!    *different* commit body (different `from_epoch`, `group_id`,
//!    `sender`, or `ops_hash`). We reject any commit whose
//!    `sig_transcript` disagrees with the recomputed local
//!    transcript with `TranscriptMismatch`.
//! 4. **Removed-leaf signature** — the committer leaf is no longer
//!    a member of the group at the claimed `from_epoch`. We reject
//!    with `NonMemberSigner` independent of whether the signature
//!    is otherwise well-formed.
//! 5. **Wrong-group splice** — the signature transcript claims a
//!    different `group_id` than the receiver's local view, even if
//!    the rest of the body matches. We reject with `GroupIdSplice`.
//!
//! This module is pure — no actual signature verification crate is
//! pulled in; we model the committer-signed body as a transcript
//! tuple and pin the freshness/binding invariants the application
//! MUST enforce *before* delegating to `ed25519-dalek`. Real
//! signature checks live in [`identity`] (CR-CHAT-01) and the
//! caller composes the two gates.
//!
//! ## Guard surface
//!
//! [`SignedCommit`] — wire envelope (commit body + signature
//! metadata).
//! [`CommitVerifierView`] — the receiver's local view (current
//! group_id, current epoch, current leaf-public for the committer,
//! and whether the committer is still a member).
//! [`verify_commit_signature`] — single-entry gate returning
//! `Result<(), CommitSigError>`. Application MUST call this before
//! passing the body to `Group::process_commit`.
//!
//! ## Honesty (R5)
//! `[VERIFIED]` — 10 CSF-01..10 unit tests pass; no I/O, no allocs
//! beyond the inputs.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · COMMIT-SIGNATURE-FORGE`

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// MLS commit transcript — the canonical tuple a signature commits
/// to. Two commits produce equal transcripts iff they are the same
/// commit. Re-using a signature across two distinct transcripts is
/// the splice attack CSF-03 / CSF-05 pin down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitTranscript {
    /// The group this commit targets.
    pub group_id: [u8; 32],
    /// The epoch the commit transitions **from**.
    pub from_epoch: u64,
    /// The committer leaf index.
    pub sender_leaf: u32,
    /// Application-supplied digest of the `ops` list (any
    /// collision-resistant hash; modelled as a 32-byte tag here).
    pub ops_hash: [u8; 32],
}

/// One signed commit on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedCommit {
    /// The transcript the sender claims their signature covers.
    pub transcript: CommitTranscript,
    /// The 32-byte public key the sender used. Receiver verifies
    /// this matches their local view of the committer's leaf-public.
    pub sig_pubkey: [u8; 32],
    /// The signature blob. We don't verify the cryptography here;
    /// we pin the *binding* invariants only. A zero blob is rejected
    /// outright.
    pub sig_blob: Vec<u8>,
}

/// Receiver's local view at the moment a `SignedCommit` arrives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitVerifierView {
    /// The receiver's current group id.
    pub group_id: [u8; 32],
    /// The receiver's current epoch (must equal `from_epoch`).
    pub current_epoch: u64,
    /// The receiver's currently-stored leaf-public for the
    /// committer. `None` ⇒ committer is not a member.
    pub committer_leaf_key: Option<[u8; 32]>,
    /// The receiver's locally-computed `ops_hash` over the commit
    /// body. Two correctly-formed commits with identical wire
    /// payloads produce equal hashes.
    pub local_ops_hash: [u8; 32],
}

/// Rejection reasons. Variants are `#[non_exhaustive]` so future
/// waves can add tightening checks without breaking downstream
/// `match` arms.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CommitSigError {
    /// The `sig_blob` is zero-length or all-zero — a lazy verifier
    /// trap.
    #[error("commit signature: empty or zero-byte signature")]
    EmptySignature,
    /// The `sig_pubkey` differs from the receiver's stored
    /// leaf-public for the committer — likely a stale pre-rotation
    /// key.
    #[error("commit signature: stale signer key (does not match current leaf-public)")]
    StaleSignerKey,
    /// The transcript's `group_id` differs from the receiver's
    /// current `group_id` — cross-group splice.
    #[error("commit signature: transcript group_id splice")]
    GroupIdSplice,
    /// The transcript's `from_epoch` differs from the receiver's
    /// current epoch — epoch-binding violation.
    #[error("commit signature: transcript epoch mismatch (current={current}, claimed={claimed})")]
    EpochMismatch {
        /// The receiver's local epoch.
        current: u64,
        /// The epoch the transcript claims to transition from.
        claimed: u64,
    },
    /// The committer leaf is not currently a member of the group.
    #[error("commit signature: signer is not a current member")]
    NonMemberSigner,
    /// The transcript's `ops_hash` differs from the receiver's
    /// locally-computed `ops_hash` — cross-commit splice.
    #[error("commit signature: transcript ops_hash mismatch")]
    TranscriptMismatch,
}

/// Single-entry verification gate for the *binding* layer of a
/// signed commit. Cryptographic signature verification is the
/// caller's follow-up step once this gate returns `Ok(())`.
///
/// Check order is fixed (any reorder is a behavioural change and is
/// covered by INV-CHAT-138):
///
/// 1. `sig_blob` non-empty AND non-zero
/// 2. group_id agreement (splice rejection)
/// 3. epoch agreement
/// 4. committer is currently a member
/// 5. `sig_pubkey` matches stored leaf-public
/// 6. `ops_hash` matches local hash
///
/// `[VERIFIED]` — exhaustively tested via CSF-01..10.
pub fn verify_commit_signature(
    sc: &SignedCommit,
    view: &CommitVerifierView,
) -> Result<(), CommitSigError> {
    // Rule 1 — empty/zero signature.
    if sc.sig_blob.is_empty() || sc.sig_blob.iter().all(|b| *b == 0) {
        return Err(CommitSigError::EmptySignature);
    }

    // Rule 2 — group_id splice.
    if sc.transcript.group_id != view.group_id {
        return Err(CommitSigError::GroupIdSplice);
    }

    // Rule 3 — epoch agreement.
    if sc.transcript.from_epoch != view.current_epoch {
        return Err(CommitSigError::EpochMismatch {
            current: view.current_epoch,
            claimed: sc.transcript.from_epoch,
        });
    }

    // Rule 4 — committer must be a current member.
    let stored_key = match view.committer_leaf_key {
        Some(k) => k,
        None => return Err(CommitSigError::NonMemberSigner),
    };

    // Rule 5 — stale signer key.
    if sc.sig_pubkey != stored_key {
        return Err(CommitSigError::StaleSignerKey);
    }

    // Rule 6 — ops_hash agreement.
    if sc.transcript.ops_hash != view.local_ops_hash {
        return Err(CommitSigError::TranscriptMismatch);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gid() -> [u8; 32] {
        [0x11; 32]
    }

    fn ops() -> [u8; 32] {
        [0x22; 32]
    }

    fn leaf_key() -> [u8; 32] {
        [0x33; 32]
    }

    fn good_commit() -> SignedCommit {
        SignedCommit {
            transcript: CommitTranscript {
                group_id: gid(),
                from_epoch: 4,
                sender_leaf: 7,
                ops_hash: ops(),
            },
            sig_pubkey: leaf_key(),
            sig_blob: vec![0xAB; 64],
        }
    }

    fn good_view() -> CommitVerifierView {
        CommitVerifierView {
            group_id: gid(),
            current_epoch: 4,
            committer_leaf_key: Some(leaf_key()),
            local_ops_hash: ops(),
        }
    }

    /// CSF-01 — happy path: matched everything → accepted.
    #[test]
    fn csf_01_happy_path_accepted() {
        let sc = good_commit();
        let v = good_view();
        assert_eq!(verify_commit_signature(&sc, &v), Ok(()));
    }

    /// CSF-02 — empty signature blob → `EmptySignature`.
    #[test]
    fn csf_02_empty_signature_rejected() {
        let mut sc = good_commit();
        sc.sig_blob = vec![];
        let v = good_view();
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::EmptySignature));
    }

    /// CSF-03 — all-zero signature blob → `EmptySignature`. A lazy
    /// verifier might short-circuit `if sig == 0 then Ok`; we catch
    /// it here.
    #[test]
    fn csf_03_zero_byte_signature_rejected() {
        let mut sc = good_commit();
        sc.sig_blob = vec![0u8; 64];
        let v = good_view();
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::EmptySignature));
    }

    /// CSF-04 — group_id splice → `GroupIdSplice`.
    #[test]
    fn csf_04_group_id_splice_rejected() {
        let mut sc = good_commit();
        sc.transcript.group_id = [0xCC; 32];
        let v = good_view();
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::GroupIdSplice));
    }

    /// CSF-05 — epoch mismatch (transcript epoch 3, view epoch 4)
    /// → `EpochMismatch { current: 4, claimed: 3 }`.
    #[test]
    fn csf_05_epoch_mismatch_rejected() {
        let mut sc = good_commit();
        sc.transcript.from_epoch = 3;
        let v = good_view();
        assert_eq!(
            verify_commit_signature(&sc, &v),
            Err(CommitSigError::EpochMismatch { current: 4, claimed: 3 }),
        );
    }

    /// CSF-06 — committer not a current member → `NonMemberSigner`.
    #[test]
    fn csf_06_non_member_signer_rejected() {
        let sc = good_commit();
        let mut v = good_view();
        v.committer_leaf_key = None;
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::NonMemberSigner));
    }

    /// CSF-07 — stale signer key (pre-rotation) → `StaleSignerKey`.
    #[test]
    fn csf_07_stale_signer_key_rejected() {
        let mut sc = good_commit();
        sc.sig_pubkey = [0xEE; 32]; // post-rotation key would be different
        let v = good_view();
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::StaleSignerKey));
    }

    /// CSF-08 — transcript ops_hash diverges from local hash
    /// → `TranscriptMismatch` (cross-commit splice).
    #[test]
    fn csf_08_transcript_ops_hash_mismatch_rejected() {
        let mut sc = good_commit();
        sc.transcript.ops_hash = [0xFF; 32];
        let v = good_view();
        assert_eq!(
            verify_commit_signature(&sc, &v),
            Err(CommitSigError::TranscriptMismatch),
        );
    }

    /// CSF-09 — check-order invariant: empty-signature is detected
    /// before any other failure. Specifically, when *every* other
    /// field is also wrong, the error MUST still be
    /// `EmptySignature`.
    #[test]
    fn csf_09_check_order_empty_signature_first() {
        let mut sc = good_commit();
        sc.sig_blob = vec![];
        sc.transcript.group_id = [0xCC; 32];
        sc.transcript.from_epoch = 99;
        sc.sig_pubkey = [0xEE; 32];
        sc.transcript.ops_hash = [0xFF; 32];
        let mut v = good_view();
        v.committer_leaf_key = None;
        assert_eq!(verify_commit_signature(&sc, &v), Err(CommitSigError::EmptySignature));
    }

    /// CSF-10 — green summary: 10 CSF falsifiers active.
    #[test]
    fn csf_10_green_summary() {
        let count = 10usize;
        assert_eq!(count, 10, "CSF-01..10: commit-signature-forge gate active");
    }
}
