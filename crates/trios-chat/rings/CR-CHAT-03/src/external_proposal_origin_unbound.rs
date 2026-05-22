//! Wave-33 / L-CHAT-3-epou (R-CHAT-11 / CR-CHAT-03) — External
//! proposal origin unbound per RFC 9420 §12.1.8.2 (`new_member_*`
//! external senders) and §6.2 (`sender.type = external`).
//!
//! An MLS external proposal is a proposal NOT authored by a current
//! member of the group — it is authored either by an external sender
//! key in the group's `external_senders` extension, or by a
//! prospective new member via the `new_member_proposal` path. Every
//! such proposal carries an `origin` (sender index into
//! `external_senders` or `NewMemberCommit`/`NewMemberProposal`
//! discriminant). A hostile party may try to:
//!   - forge an `origin` discriminant that is not in the group's
//!     declared `external_senders` set (UnknownExternalOrigin),
//!   - submit an external proposal kind that is NOT permitted for the
//!     declared origin (e.g. a `Remove` from a `new_member_proposal`
//!     sender — RFC 9420 §12.1.8.2 only allows Add and
//!     `external_init` from new members) (UnpermittedExternalKind),
//!   - cross-group splice an external proposal
//!     (CrossGroupExternalProposal),
//!   - cross-epoch splice (StaleEpochExternalProposal),
//!   - replay an `(origin, proposal_id)` pair already consumed
//!     (ExternalProposalReplay),
//!   - submit the all-zero `origin_signature` (degenerate Ed25519
//!     output / sentinel) (ZeroOriginSignature),
//!   - submit a non-canonical `origin_signature` length
//!     (NonCanonicalOriginSignatureLength).
//!
//! Seven rules enforced in fixed order (single deny wins):
//!   1. NonCanonicalOriginSignatureLength — `origin_signature.len()`
//!      must equal `ORIGIN_SIGNATURE_LEN` (64 bytes — Ed25519 output,
//!      RFC 8032 §5.1.6).
//!   2. UnknownExternalOrigin — `origin` must be present in
//!      `view.declared_external_origins` (no phantom external sender).
//!   3. UnpermittedExternalKind — `(origin, kind)` must be present in
//!      `view.permitted_kinds_for_origin` — RFC 9420 §12.1.8.2 says
//!      `new_member_*` origins may submit only the kinds in that set
//!      (canonically: Add and external_init for new members).
//!   4. CrossGroupExternalProposal — `group_id` must match
//!      `view.expected_group_id`.
//!   5. StaleEpochExternalProposal — `proposal_epoch` must equal
//!      `view.current_epoch`.
//!   6. ExternalProposalReplay — the `(origin, proposal_id)` pair
//!      must not appear in `view.used_external_proposals`.
//!   7. ZeroOriginSignature — the all-zero `origin_signature` is
//!      forbidden (a correctly evaluated Ed25519 sign never produces
//!      it).

#![forbid(unsafe_code)]

/// Canonical `origin_signature` length (64 bytes — Ed25519 per RFC
/// 8032 §5.1.6, matched by W14 `LEAF_NODE_SIGNATURE_LEN`).
pub const ORIGIN_SIGNATURE_LEN: usize = 64;

/// Maximum byte length of a `proposal_id` payload we accept — matches
/// the `PROPOSAL_ID_MAX_LEN` constant from W32 `proposal_ref_collision`.
pub const EXTERNAL_PROPOSAL_ID_MAX_LEN: usize = 255;

/// Discriminant identifying the origin of an external MLS proposal
/// per RFC 9420 §6.2 / §12.1.8.2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalOrigin {
    /// Index into `group.external_senders` extension list.
    ExternalSender(u32),
    /// `new_member_proposal` from a prospective member (joining via
    /// an `external_init` flow).
    NewMemberProposal,
    /// `new_member_commit` from a prospective member.
    NewMemberCommit,
}

/// Kind of MLS proposal an external sender claims to be submitting
/// (subset of the §12.1 ProposalType registry — only those an
/// external party may even theoretically attempt).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternalProposalKind {
    /// Add a new member (Add).
    Add,
    /// External-init (`external_init` from a NewMember commit path).
    ExternalInit,
    /// Remove a member (Remove). NOT permitted for `new_member_*`
    /// origins — only for declared `ExternalSender(_)` origins whose
    /// `external_senders` entry grants the privilege.
    Remove,
    /// Update — RFC 9420 forbids this from any external origin.
    Update,
}

/// External proposal as carried on the wire (RFC 9420 §12.1.8.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalProposal {
    pub origin: ExternalOrigin,
    pub kind: ExternalProposalKind,
    pub proposal_id: Vec<u8>,
    pub origin_signature: Vec<u8>,
    pub group_id: Vec<u8>,
    pub proposal_epoch: u64,
}

/// Local view of declared external senders + permitted kinds + used
/// proposals at the current epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalProposalView {
    pub expected_group_id: Vec<u8>,
    pub current_epoch: u64,
    /// Origins the group's GroupContextExtensions actually declare.
    pub declared_external_origins: Vec<ExternalOrigin>,
    /// `(origin, kind)` pairs the group's policy permits.
    pub permitted_kinds_for_origin:
        Vec<(ExternalOrigin, ExternalProposalKind)>,
    /// `(origin, proposal_id)` pairs already consumed by a prior
    /// Commit.
    pub used_external_proposals: Vec<(ExternalOrigin, Vec<u8>)>,
}

/// Typed errors for `validate_external_proposal_origin`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExternalProposalError {
    /// Rule 1 — `origin_signature.len() != ORIGIN_SIGNATURE_LEN`.
    NonCanonicalOriginSignatureLength,
    /// Rule 2 — `origin` not in `view.declared_external_origins`.
    UnknownExternalOrigin,
    /// Rule 3 — `(origin, kind)` not in
    /// `view.permitted_kinds_for_origin`.
    UnpermittedExternalKind,
    /// Rule 4 — cross-group splice.
    CrossGroupExternalProposal,
    /// Rule 5 — cross-epoch splice.
    StaleEpochExternalProposal,
    /// Rule 6 — `(origin, proposal_id)` already in
    ///          `view.used_external_proposals`.
    ExternalProposalReplay,
    /// Rule 7 — all-zero `origin_signature`.
    ZeroOriginSignature,
}

/// Constructive guard for an external proposal. Returns `Ok(())` iff
/// every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `EPOU-01..10` below and the
/// Coq theorems `INV-CHAT-201..207` in the W33 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_external_proposal_origin(
    proposal: &ExternalProposal,
    view: &ExternalProposalView,
) -> Result<(), ExternalProposalError> {
    if proposal.origin_signature.len() != ORIGIN_SIGNATURE_LEN {
        return Err(
            ExternalProposalError::NonCanonicalOriginSignatureLength,
        );
    }
    if !view
        .declared_external_origins
        .contains(&proposal.origin)
    {
        return Err(ExternalProposalError::UnknownExternalOrigin);
    }
    let pk_pair = (proposal.origin.clone(), proposal.kind.clone());
    if !view.permitted_kinds_for_origin.contains(&pk_pair) {
        return Err(ExternalProposalError::UnpermittedExternalKind);
    }
    if proposal.group_id != view.expected_group_id {
        return Err(ExternalProposalError::CrossGroupExternalProposal);
    }
    if proposal.proposal_epoch != view.current_epoch {
        return Err(ExternalProposalError::StaleEpochExternalProposal);
    }
    let used_pair = (
        proposal.origin.clone(),
        proposal.proposal_id.clone(),
    );
    if view.used_external_proposals.contains(&used_pair) {
        return Err(ExternalProposalError::ExternalProposalReplay);
    }
    if proposal.origin_signature.iter().all(|&b| b == 0) {
        return Err(ExternalProposalError::ZeroOriginSignature);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_sig() -> Vec<u8> {
        vec![0x55_u8; ORIGIN_SIGNATURE_LEN]
    }

    fn ok_view() -> ExternalProposalView {
        ExternalProposalView {
            expected_group_id: b"trinity-group-33".to_vec(),
            current_epoch: 12,
            declared_external_origins: vec![
                ExternalOrigin::ExternalSender(0),
                ExternalOrigin::NewMemberProposal,
            ],
            permitted_kinds_for_origin: vec![
                (
                    ExternalOrigin::ExternalSender(0),
                    ExternalProposalKind::Remove,
                ),
                (
                    ExternalOrigin::NewMemberProposal,
                    ExternalProposalKind::Add,
                ),
                (
                    ExternalOrigin::NewMemberProposal,
                    ExternalProposalKind::ExternalInit,
                ),
            ],
            used_external_proposals: vec![],
        }
    }

    fn ok_proposal() -> ExternalProposal {
        ExternalProposal {
            origin: ExternalOrigin::NewMemberProposal,
            kind: ExternalProposalKind::Add,
            proposal_id: b"ext-prop-A".to_vec(),
            origin_signature: canonical_sig(),
            group_id: b"trinity-group-33".to_vec(),
            proposal_epoch: 12,
        }
    }

    /// EPOU-01 — 32-byte origin_signature rejected.
    #[test]
    fn epou_01_short_signature_rejected() {
        let mut p = ok_proposal();
        p.origin_signature = vec![0x55_u8; 32];
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::NonCanonicalOriginSignatureLength)
        );
    }

    /// EPOU-02 — 128-byte origin_signature rejected (over-long).
    #[test]
    fn epou_02_over_long_signature_rejected() {
        let mut p = ok_proposal();
        p.origin_signature = vec![0x55_u8; 128];
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::NonCanonicalOriginSignatureLength)
        );
    }

    /// EPOU-03 — unknown ExternalSender origin rejected.
    #[test]
    fn epou_03_unknown_external_sender_rejected() {
        let mut p = ok_proposal();
        p.origin = ExternalOrigin::ExternalSender(7);
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::UnknownExternalOrigin)
        );
    }

    /// EPOU-04 — NewMemberProposal submitting Remove rejected
    /// (UnpermittedExternalKind — RFC 9420 §12.1.8.2 forbids).
    #[test]
    fn epou_04_new_member_remove_rejected() {
        let mut p = ok_proposal();
        p.kind = ExternalProposalKind::Remove;
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::UnpermittedExternalKind)
        );
    }

    /// EPOU-05 — NewMemberProposal submitting Update rejected
    /// (Update is never permitted from any external origin).
    #[test]
    fn epou_05_new_member_update_rejected() {
        let mut p = ok_proposal();
        p.kind = ExternalProposalKind::Update;
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::UnpermittedExternalKind)
        );
    }

    /// EPOU-06 — cross-group external proposal rejected.
    #[test]
    fn epou_06_cross_group_rejected() {
        let mut p = ok_proposal();
        p.group_id = b"trinity-group-OTHER".to_vec();
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::CrossGroupExternalProposal)
        );
    }

    /// EPOU-07 — stale-epoch external proposal rejected.
    #[test]
    fn epou_07_stale_epoch_rejected() {
        let mut p = ok_proposal();
        p.proposal_epoch = 11;
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::StaleEpochExternalProposal)
        );
    }

    /// EPOU-08 — replayed (origin, proposal_id) rejected.
    #[test]
    fn epou_08_replay_rejected() {
        let p = ok_proposal();
        let mut view = ok_view();
        view.used_external_proposals.push((
            p.origin.clone(),
            p.proposal_id.clone(),
        ));
        assert_eq!(
            validate_external_proposal_origin(&p, &view),
            Err(ExternalProposalError::ExternalProposalReplay)
        );
    }

    /// EPOU-09 — all-zero origin_signature rejected.
    #[test]
    fn epou_09_zero_signature_rejected() {
        let mut p = ok_proposal();
        p.origin_signature = vec![0u8; ORIGIN_SIGNATURE_LEN];
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Err(ExternalProposalError::ZeroOriginSignature)
        );
    }

    /// EPOU-10 — canonical ExternalSender Remove accepted (the only
    /// declared external sender path that permits Remove).
    #[test]
    fn epou_10_canonical_external_sender_remove_accepted() {
        let mut p = ok_proposal();
        p.origin = ExternalOrigin::ExternalSender(0);
        p.kind = ExternalProposalKind::Remove;
        assert_eq!(
            validate_external_proposal_origin(&p, &ok_view()),
            Ok(())
        );
    }
}
