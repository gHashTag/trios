//! Wave-32 / L-CHAT-3-pref (R-CHAT-11 / CR-CHAT-03) — Proposal
//! reference (`ProposalRef`) collision / replay / provenance per
//! RFC 9420 §12.1.1.
//!
//! Each MLS proposal is referenced by a MAC-based `ProposalRef`
//! (HMAC over the canonical proposal encoding under
//! `membership_key`). A hostile member may try to:
//!   - submit a `ProposalRef` of non-canonical length,
//!   - replay a `ProposalRef` already used in a prior Commit,
//!   - point at a `proposal_id` that the local store has never
//!     observed (referencing a phantom proposal),
//!   - cross-group splice a `ProposalRef` (CrossGroupProposalRef),
//!   - cross-epoch splice a `ProposalRef` (StaleEpochProposalRef),
//!   - submit the all-zero `ProposalRef` (degenerate HMAC output).
//!
//! Six rules enforced in fixed order (a single deny wins):
//!   1. NonCanonicalProposalRefLength — exactly 32 bytes (HMAC-SHA256
//!      output truncated to ciphersuite hash length pinned at 32 in
//!      W11 / RFC 9420 §5.2).
//!   2. EmptyProposalId — `proposal_id` must be non-empty (RFC 9420
//!      identifies each proposal by a non-empty canonical hash).
//!   3. UnknownProposalRef — `proposal_id` must be present in
//!      `view.known_proposal_ids` (no phantom references).
//!   4. CrossGroupProposalRef — `(proposal_id, group_id)` must match
//!      the local `(proposal_id, expected_group_id)` (binding from
//!      §12.1.1).
//!   5. StaleEpochProposalRef — `proposal_epoch` must equal
//!      `view.current_epoch` (no cross-epoch splice).
//!   6. ProposalRefReplay — the pair `(proposal_id, proposal_ref)`
//!      must not appear in `view.used_proposal_refs`.
//!   7. ZeroProposalRef — the all-zero `proposal_ref` is forbidden
//!      (a correctly evaluated HMAC never produces it).

#![forbid(unsafe_code)]

/// Canonical `ProposalRef` length (32 bytes — truncated HMAC-SHA256
/// per RFC 9420 §5.2 / §12.1.1 with the pinned ciphersuite set).
pub const PROPOSAL_REF_LEN: usize = 32;

/// Maximum `proposal_id` byte length we accept — generous upper bound
/// matching `EXTERNAL_PSK_ID_MAX_LEN` from CR-CHAT-03.
pub const PROPOSAL_ID_MAX_LEN: usize = 255;

/// Reference to a proposal carried inside a Commit per RFC 9420
/// §12.1.1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalReference {
    pub proposal_id: Vec<u8>,
    pub proposal_ref: Vec<u8>,
    pub group_id: Vec<u8>,
    pub proposal_epoch: u64,
}

/// Local view of known / used proposals at the current epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProposalRefView {
    pub expected_group_id: Vec<u8>,
    pub current_epoch: u64,
    /// `proposal_id`s the local store has actually observed.
    pub known_proposal_ids: Vec<Vec<u8>>,
    /// `(proposal_id, proposal_ref)` pairs already consumed by a
    /// prior Commit.
    pub used_proposal_refs: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Typed errors for `validate_proposal_ref`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProposalRefError {
    /// Rule 1 — `proposal_ref.len() != PROPOSAL_REF_LEN`.
    NonCanonicalProposalRefLength,
    /// Rule 2 — empty `proposal_id`.
    EmptyProposalId,
    /// Rule 3 — `proposal_id` not in `view.known_proposal_ids`.
    UnknownProposalRef,
    /// Rule 4 — cross-group splice.
    CrossGroupProposalRef,
    /// Rule 5 — cross-epoch splice.
    StaleEpochProposalRef,
    /// Rule 6 — `(proposal_id, proposal_ref)` already in
    ///          `view.used_proposal_refs`.
    ProposalRefReplay,
    /// Rule 7 — all-zero `proposal_ref`.
    ZeroProposalRef,
}

/// Constructive guard for a `ProposalRef`. Returns `Ok(())` iff
/// every rule (1)..(7) holds.
pub fn validate_proposal_ref(
    proposal: &ProposalReference,
    view: &ProposalRefView,
) -> Result<(), ProposalRefError> {
    if proposal.proposal_ref.len() != PROPOSAL_REF_LEN {
        return Err(ProposalRefError::NonCanonicalProposalRefLength);
    }
    if proposal.proposal_id.is_empty() {
        return Err(ProposalRefError::EmptyProposalId);
    }
    if !view.known_proposal_ids.contains(&proposal.proposal_id) {
        return Err(ProposalRefError::UnknownProposalRef);
    }
    if proposal.group_id != view.expected_group_id {
        return Err(ProposalRefError::CrossGroupProposalRef);
    }
    if proposal.proposal_epoch != view.current_epoch {
        return Err(ProposalRefError::StaleEpochProposalRef);
    }
    let pair = (
        proposal.proposal_id.clone(),
        proposal.proposal_ref.clone(),
    );
    if view.used_proposal_refs.contains(&pair) {
        return Err(ProposalRefError::ProposalRefReplay);
    }
    if proposal.proposal_ref.iter().all(|&b| b == 0) {
        return Err(ProposalRefError::ZeroProposalRef);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_ref() -> Vec<u8> {
        vec![0x33_u8; PROPOSAL_REF_LEN]
    }

    fn ok_view() -> ProposalRefView {
        ProposalRefView {
            expected_group_id: b"trinity-group-32".to_vec(),
            current_epoch: 11,
            known_proposal_ids: vec![b"prop-A".to_vec(), b"prop-B".to_vec()],
            used_proposal_refs: vec![],
        }
    }

    fn ok_proposal() -> ProposalReference {
        ProposalReference {
            proposal_id: b"prop-A".to_vec(),
            proposal_ref: canonical_ref(),
            group_id: b"trinity-group-32".to_vec(),
            proposal_epoch: 11,
        }
    }

    /// PREF-01 — 16-byte proposal_ref rejected.
    #[test]
    fn pref_01_short_proposal_ref_rejected() {
        let mut p = ok_proposal();
        p.proposal_ref = vec![0x33_u8; 16];
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::NonCanonicalProposalRefLength)
        );
    }

    /// PREF-02 — 64-byte proposal_ref rejected (over-long).
    #[test]
    fn pref_02_over_long_proposal_ref_rejected() {
        let mut p = ok_proposal();
        p.proposal_ref = vec![0x33_u8; 64];
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::NonCanonicalProposalRefLength)
        );
    }

    /// PREF-03 — empty proposal_id rejected.
    #[test]
    fn pref_03_empty_proposal_id_rejected() {
        let mut p = ok_proposal();
        p.proposal_id = vec![];
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::EmptyProposalId)
        );
    }

    /// PREF-04 — unknown proposal_id rejected.
    #[test]
    fn pref_04_unknown_proposal_id_rejected() {
        let mut p = ok_proposal();
        p.proposal_id = b"phantom-prop".to_vec();
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::UnknownProposalRef)
        );
    }

    /// PREF-05 — cross-group proposal_ref rejected.
    #[test]
    fn pref_05_cross_group_proposal_ref_rejected() {
        let mut p = ok_proposal();
        p.group_id = b"trinity-group-OTHER".to_vec();
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::CrossGroupProposalRef)
        );
    }

    /// PREF-06 — stale-epoch proposal_ref rejected.
    #[test]
    fn pref_06_stale_epoch_proposal_ref_rejected() {
        let mut p = ok_proposal();
        p.proposal_epoch = 10;
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::StaleEpochProposalRef)
        );
    }

    /// PREF-07 — replayed (proposal_id, proposal_ref) rejected.
    #[test]
    fn pref_07_replayed_proposal_ref_rejected() {
        let p = ok_proposal();
        let mut view = ok_view();
        view.used_proposal_refs.push((
            p.proposal_id.clone(),
            p.proposal_ref.clone(),
        ));
        assert_eq!(
            validate_proposal_ref(&p, &view),
            Err(ProposalRefError::ProposalRefReplay)
        );
    }

    /// PREF-08 — all-zero proposal_ref rejected.
    #[test]
    fn pref_08_zero_proposal_ref_rejected() {
        let mut p = ok_proposal();
        p.proposal_ref = vec![0u8; PROPOSAL_REF_LEN];
        assert_eq!(
            validate_proposal_ref(&p, &ok_view()),
            Err(ProposalRefError::ZeroProposalRef)
        );
    }

    /// PREF-09 — canonical proposal_ref accepted.
    #[test]
    fn pref_09_canonical_proposal_ref_accepted() {
        assert_eq!(
            validate_proposal_ref(&ok_proposal(), &ok_view()),
            Ok(())
        );
    }

    /// PREF-10 — same proposal_id with a distinct (fresh) proposal_ref accepted.
    #[test]
    fn pref_10_distinct_fresh_proposal_ref_accepted() {
        let p = ok_proposal();
        let mut view = ok_view();
        // A different proposal_ref under the same proposal_id was used
        // before — must not false-positive against rule (6).
        view.used_proposal_refs.push((
            p.proposal_id.clone(),
            vec![0xAA_u8; PROPOSAL_REF_LEN],
        ));
        assert_eq!(
            validate_proposal_ref(&p, &view),
            Ok(())
        );
    }
}
