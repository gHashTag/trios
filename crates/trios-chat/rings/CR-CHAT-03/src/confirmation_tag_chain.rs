//! # CR-CHAT-03 · L-CHAT-3-confupd — MLS confirmation_tag chain validation
//!
//! `[VERIFIED]` Wave-28 lane A — Defends against a class of attacks
//! where an adversary tampers with the MLS confirmation chain bound
//! around each Commit (RFC 9420 §8.1 — Group Context and §11 —
//! Confirmation Tag):
//!
//! * **Confirmation tag length forge** — claims a `confirmation_tag`
//!   shorter or longer than `CONFIRMATION_TAG_LEN` bytes.
//! * **Confirmed transcript chain splice** — claims a Commit whose
//!   `prev_confirmed_transcript_hash` does NOT match the receiver's
//!   currently-accepted confirmed_transcript_hash (cross-history
//!   chain splice).
//! * **Stale chain replay** — claims a Commit whose `epoch` is not
//!   strictly greater than the receiver's current_epoch.
//! * **Cross-group splice** — claims a Commit whose `group_id` does
//!   not match the local group.
//! * **Empty interim transcript** — claims a Commit whose
//!   `interim_transcript_hash` (the seed for the next chain link) is
//!   all-zero or the wrong length; this would let an attacker reset
//!   the chain on the next epoch.
//! * **Repeated confirmation tag** — claims a Commit whose
//!   `confirmation_tag` has already been recorded in the chain
//!   ledger for the same `(group_id, epoch)` (replay).
//!
//! See RFC 9420 §8.1 (Group Context: confirmed_transcript_hash and
//! interim_transcript_hash) and §11 (the confirmation_tag is an HMAC
//! over confirmed_transcript_hash keyed by confirmation_key). The six
//! rules below are enforced in fixed order; any attempt to weaken or
//! skip them produces a `ConfirmationChainError`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-CONFIRMATION-CHAIN`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical length of `confirmation_tag` in bytes — RFC 9420 default
/// ciphersuite uses SHA-256 ⇒ HMAC-SHA-256 ⇒ 32-byte tag.
pub const CONFIRMATION_TAG_LEN: usize = 32;

/// Canonical length of `interim_transcript_hash` in bytes — same
/// hash output (SHA-256 ⇒ 32 bytes).
pub const INTERIM_TRANSCRIPT_HASH_LEN: usize = 32;

/// One signed MLS Commit packet bringing the chain to the next link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedCommit {
    /// Local group identifier.
    pub group_id: Vec<u8>,
    /// Epoch the Commit transitions **into** (strictly > current).
    pub epoch: u64,
    /// `confirmed_transcript_hash` at the time the Commit was signed —
    /// MUST equal the receiver's currently-accepted confirmed_transcript_hash.
    pub prev_confirmed_transcript_hash: Vec<u8>,
    /// HMAC-SHA-256(confirmation_key, confirmed_transcript_hash) —
    /// exactly `CONFIRMATION_TAG_LEN` bytes.
    pub confirmation_tag: Vec<u8>,
    /// `interim_transcript_hash` seed for the NEXT chain link —
    /// exactly `INTERIM_TRANSCRIPT_HASH_LEN` bytes, MUST be non-zero.
    pub next_interim_transcript_hash: Vec<u8>,
}

/// Receiving-group view used to validate a `ConfirmedCommit`. The
/// receiver trusts only the chain it has already accepted (the
/// `confirmed_transcript_hash` is the cryptographic link between
/// successive epochs).
#[derive(Debug, Clone)]
pub struct ConfirmationChainView {
    /// `group_id` of the local group.
    pub local_group_id: Vec<u8>,
    /// Current epoch the receiver has accepted.
    pub current_epoch: u64,
    /// `confirmed_transcript_hash` after the most recent accepted
    /// Commit — the cryptographic anchor for the next link.
    pub current_confirmed_transcript_hash: Vec<u8>,
    /// Ledger of `(group_id, epoch, confirmation_tag)` triples already
    /// accepted — replay guard against the same tag being re-injected
    /// at the same coordinates.
    pub used_chain_links: BTreeSet<(Vec<u8>, u64, Vec<u8>)>,
}

/// All ways a `ConfirmedCommit` can be rejected. Adding variants stays
/// non-breaking via `#[non_exhaustive]`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConfirmationChainError {
    /// `confirmation_tag.len() != CONFIRMATION_TAG_LEN`.
    NonCanonicalTagLength,
    /// `group_id != local_group_id`.
    CrossGroupSplice,
    /// `epoch <= current_epoch` — stale or replayed Commit.
    StaleEpochReplay,
    /// `prev_confirmed_transcript_hash != current_confirmed_transcript_hash`.
    TranscriptChainSplice,
    /// `next_interim_transcript_hash` length wrong or all-zero —
    /// would let an attacker reset the chain on the next epoch.
    EmptyInterimTranscript,
    /// `(group_id, epoch, confirmation_tag)` already in
    /// `used_chain_links` — replay.
    RepeatedConfirmationTag,
}

/// `[VERIFIED]` Validate a `ConfirmedCommit` against the receiving
/// group's `ConfirmationChainView`. Returns `Ok(())` if all six rules
/// pass, else the first failing rule as a `ConfirmationChainError`.
///
/// Rules enforced in fixed order from RFC 9420 §8.1 + §11:
///
/// 1. `confirmation_tag.len() == CONFIRMATION_TAG_LEN`.
/// 2. `group_id == view.local_group_id`.
/// 3. `epoch > view.current_epoch`.
/// 4. `prev_confirmed_transcript_hash == view.current_confirmed_transcript_hash`.
/// 5. `next_interim_transcript_hash.len() == INTERIM_TRANSCRIPT_HASH_LEN`
///    AND it is not the all-zero block.
/// 6. `(group_id, epoch, confirmation_tag)` not in `view.used_chain_links`.
pub fn validate_confirmation_chain(
    commit: &ConfirmedCommit,
    view: &ConfirmationChainView,
) -> Result<(), ConfirmationChainError> {
    // 1. Canonical confirmation_tag length.
    if commit.confirmation_tag.len() != CONFIRMATION_TAG_LEN {
        return Err(ConfirmationChainError::NonCanonicalTagLength);
    }
    // 2. Cross-group splice.
    if commit.group_id != view.local_group_id {
        return Err(ConfirmationChainError::CrossGroupSplice);
    }
    // 3. Stale epoch replay.
    if commit.epoch <= view.current_epoch {
        return Err(ConfirmationChainError::StaleEpochReplay);
    }
    // 4. Transcript chain splice — the most important guard.
    if commit.prev_confirmed_transcript_hash != view.current_confirmed_transcript_hash {
        return Err(ConfirmationChainError::TranscriptChainSplice);
    }
    // 5. Empty/wrong-length interim transcript seed.
    if commit.next_interim_transcript_hash.len() != INTERIM_TRANSCRIPT_HASH_LEN
        || commit.next_interim_transcript_hash.iter().all(|&b| b == 0)
    {
        return Err(ConfirmationChainError::EmptyInterimTranscript);
    }
    // 6. Replay of an already-accepted chain link.
    let key = (
        commit.group_id.clone(),
        commit.epoch,
        commit.confirmation_tag.clone(),
    );
    if view.used_chain_links.contains(&key) {
        return Err(ConfirmationChainError::RepeatedConfirmationTag);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_view() -> ConfirmationChainView {
        ConfirmationChainView {
            local_group_id: vec![0xAA; 32],
            current_epoch: 5,
            current_confirmed_transcript_hash: vec![0x11; 32],
            used_chain_links: BTreeSet::new(),
        }
    }

    fn good_commit() -> ConfirmedCommit {
        ConfirmedCommit {
            group_id: vec![0xAA; 32],
            epoch: 6,
            prev_confirmed_transcript_hash: vec![0x11; 32],
            confirmation_tag: vec![0x22; CONFIRMATION_TAG_LEN],
            next_interim_transcript_hash: vec![0x33; INTERIM_TRANSCRIPT_HASH_LEN],
        }
    }

    /// **CTC-01** — short (16-byte) confirmation_tag rejected.
    #[test]
    fn ctc_01_short_tag_rejected() {
        let mut c = good_commit();
        c.confirmation_tag = vec![0x22; 16];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::NonCanonicalTagLength)
        );
    }

    /// **CTC-02** — over-long (64-byte) confirmation_tag rejected.
    #[test]
    fn ctc_02_overlong_tag_rejected() {
        let mut c = good_commit();
        c.confirmation_tag = vec![0x22; 64];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::NonCanonicalTagLength)
        );
    }

    /// **CTC-03** — cross-group splice rejected.
    #[test]
    fn ctc_03_cross_group_splice_rejected() {
        let mut c = good_commit();
        c.group_id = vec![0xBB; 32];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::CrossGroupSplice)
        );
    }

    /// **CTC-04** — stale-epoch replay rejected (epoch == current).
    #[test]
    fn ctc_04_stale_epoch_equal_rejected() {
        let mut c = good_commit();
        c.epoch = 5;
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::StaleEpochReplay)
        );
    }

    /// **CTC-05** — stale-epoch replay rejected (epoch < current).
    #[test]
    fn ctc_05_past_epoch_rejected() {
        let mut c = good_commit();
        c.epoch = 1;
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::StaleEpochReplay)
        );
    }

    /// **CTC-06** — transcript chain splice rejected (different prev hash).
    #[test]
    fn ctc_06_transcript_chain_splice_rejected() {
        let mut c = good_commit();
        c.prev_confirmed_transcript_hash = vec![0x99; 32];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::TranscriptChainSplice)
        );
    }

    /// **CTC-07** — empty interim transcript (all-zero) rejected.
    #[test]
    fn ctc_07_zero_interim_transcript_rejected() {
        let mut c = good_commit();
        c.next_interim_transcript_hash = vec![0u8; INTERIM_TRANSCRIPT_HASH_LEN];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::EmptyInterimTranscript)
        );
    }

    /// **CTC-08** — wrong-length interim transcript rejected.
    #[test]
    fn ctc_08_wrong_length_interim_transcript_rejected() {
        let mut c = good_commit();
        c.next_interim_transcript_hash = vec![0x33; 16];
        assert_eq!(
            validate_confirmation_chain(&c, &good_view()),
            Err(ConfirmationChainError::EmptyInterimTranscript)
        );
    }

    /// **CTC-09** — replayed confirmation_tag rejected.
    #[test]
    fn ctc_09_replayed_tag_rejected() {
        let mut view = good_view();
        let c = good_commit();
        view.used_chain_links.insert((
            c.group_id.clone(),
            c.epoch,
            c.confirmation_tag.clone(),
        ));
        assert_eq!(
            validate_confirmation_chain(&c, &view),
            Err(ConfirmationChainError::RepeatedConfirmationTag)
        );
    }

    /// **CTC-10** — valid Commit at next epoch with matching chain accepted.
    #[test]
    fn ctc_10_valid_commit_accepted() {
        let c = good_commit();
        let v = good_view();
        assert_eq!(validate_confirmation_chain(&c, &v), Ok(()));
    }
}
