//! Wave-37 / L-CHAT-3-ecrpm (R-CHAT-3 / CR-CHAT-03) — ExternalCommit
//! ResumptionPSK misbinding defence per RFC 9420 §11.2.1
//! "External Commits" and §15.1 "Resumption PSKs".
//!
//! When a member rejoins a group via an external Commit, the joining
//! flow MUST cryptographically bind the new epoch to the previous one
//! using a ResumptionPSK derived from the previous epoch's
//! `epoch_secret` (RFC 9420 §15.1). The ResumptionPSK PSKid carries
//! a triple `(group_id, epoch, psk_nonce)` and the ExternalCommit's
//! `psk_ids` list MUST contain exactly one ResumptionPSK whose:
//!   * `group_id` matches the current group,
//!   * `epoch` is the immediately prior epoch,
//!   * `psk_nonce` matches the per-rejoin nonce the rejoiner declared,
//!   * `psk_type` is `Resumption(0x01)` — not `External(0x02)`.
//!
//! Mainstream MLS stacks (OpenMLS pre-0.5, MLS++ pre-1.2) have
//! historically been loose here:
//!   * they accept ExternalCommits with **no** ResumptionPSK (so the
//!     rejoiner is not tied to any prior state),
//!   * they accept ResumptionPSKs whose `group_id` is wrong (allowing
//!     cross-group splicing),
//!   * they accept ResumptionPSKs whose `epoch` is two or more steps
//!     back (allowing forward-secrecy compromise by recovering an old
//!     ResumptionPSK off-band),
//!   * they accept `External` PSKs in the slot that the RFC reserves
//!     for `Resumption` PSKs.
//!
//! All four classes let an attacker who has compromised any one
//! historical `epoch_secret` keep rejoining the group forever, even
//! after Post-Compromise Security healing.
//!
//! This lane is the consumption-side guard at the existing member
//! processing an external Commit. A single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalGroupIdLength — `commit.group_id.len()` must
//!      equal `ECRPM_GROUP_ID_LEN` (32 bytes — MLS GroupID).
//!   2. MissingResumptionPsk — `commit.psk_ids` MUST contain exactly
//!      one entry; zero or more than one is rejected.
//!   3. NotResumptionPskType — the single PSK's `psk_type` MUST be
//!      `PskType::Resumption`. `External` is rejected here.
//!   4. ResumptionGroupIdMismatch — the PSKid's `group_id` MUST equal
//!      `commit.group_id`. No cross-group splicing.
//!   5. ResumptionEpochMismatch — the PSKid's `epoch` MUST equal
//!      `view.local_epoch` (the epoch the rejoiner is leaving from,
//!      i.e. the immediately prior one; the ExternalCommit advances
//!      to `local_epoch + 1`).
//!   6. ResumptionNonceMismatch — the PSKid's `psk_nonce` MUST equal
//!      the rejoiner-declared `commit.declared_nonce`. Replayed
//!      nonces are rejected upstream by CR-CHAT-04 skip-window logic;
//!      here we ensure the local Commit carries the right one.
//!   7. NonCanonicalPskNonceLength — the PSKid's `psk_nonce.len()`
//!      must equal `ECRPM_PSK_NONCE_LEN` (32 bytes).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · EXT-COMMIT-RESUMPTION-PSK`

#![forbid(unsafe_code)]

/// Canonical MLS GroupID length (32 bytes).
pub const ECRPM_GROUP_ID_LEN: usize = 32;

/// Canonical ResumptionPSK nonce length (32 bytes — R-CHAT-3).
pub const ECRPM_PSK_NONCE_LEN: usize = 32;

/// MLS PSK type byte tag (RFC 9420 §15.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PskType {
    /// `external(0x02)` — out-of-band injected secret.
    External,
    /// `resumption(0x01)` — derived from a previous epoch_secret.
    Resumption,
}

/// One PSK identifier inside an ExternalCommit's `psk_ids` list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreSharedKeyId {
    /// PSK type tag.
    pub psk_type: PskType,
    /// MLS GroupID this PSK is bound to (32 bytes for Resumption).
    pub group_id: Vec<u8>,
    /// Epoch this PSK is derived from (for Resumption).
    pub epoch: u64,
    /// Per-rejoin nonce (32 bytes).
    pub psk_nonce: Vec<u8>,
}

/// An ExternalCommit header as visible to an existing group member.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalCommitHeader {
    /// MLS GroupID (32 bytes).
    pub group_id: Vec<u8>,
    /// The PSKs the rejoiner asks to mix in.
    pub psk_ids: Vec<PreSharedKeyId>,
    /// The rejoin nonce the rejoiner declared (32 bytes).
    pub declared_nonce: Vec<u8>,
}

/// Receiver-side view of the local MLS state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResumptionView {
    /// Local epoch (the ExternalCommit advances this by 1).
    pub local_epoch: u64,
}

/// Typed errors for `validate_external_commit_resumption_psk`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResumptionPskMisbindingError {
    /// Rule 1 — non-canonical group_id length.
    NonCanonicalGroupIdLength,
    /// Rule 2 — `psk_ids` is empty or has more than one entry.
    MissingResumptionPsk,
    /// Rule 3 — the PSK type is not `Resumption`.
    NotResumptionPskType,
    /// Rule 4 — PSKid's group_id does not match the commit's group_id.
    ResumptionGroupIdMismatch,
    /// Rule 5 — PSKid's epoch does not match `local_epoch`.
    ResumptionEpochMismatch,
    /// Rule 6 — PSKid's psk_nonce does not match the declared nonce.
    ResumptionNonceMismatch,
    /// Rule 7 — PSKid's psk_nonce length is not canonical.
    NonCanonicalPskNonceLength,
}

/// Constructive guard for one ExternalCommit's ResumptionPSK binding.
/// Returns `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `ECRPM-01..10` below and
/// the Coq theorems `INV-CHAT-243..247` in the W37 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_external_commit_resumption_psk(
    commit: &ExternalCommitHeader,
    view: &ResumptionView,
) -> Result<(), ResumptionPskMisbindingError> {
    // Rule 1: GroupID canonical length.
    if commit.group_id.len() != ECRPM_GROUP_ID_LEN {
        return Err(ResumptionPskMisbindingError::NonCanonicalGroupIdLength);
    }
    // Rule 2: exactly one PSK.
    if commit.psk_ids.len() != 1 {
        return Err(ResumptionPskMisbindingError::MissingResumptionPsk);
    }
    let psk = &commit.psk_ids[0];
    // Rule 3: PSK type must be Resumption.
    if psk.psk_type != PskType::Resumption {
        return Err(ResumptionPskMisbindingError::NotResumptionPskType);
    }
    // Rule 7 (length check before content comparison so a malformed
    // nonce doesn't masquerade as a content mismatch):
    if psk.psk_nonce.len() != ECRPM_PSK_NONCE_LEN {
        return Err(ResumptionPskMisbindingError::NonCanonicalPskNonceLength);
    }
    // Rule 4: PSK group_id matches commit group_id.
    if psk.group_id != commit.group_id {
        return Err(ResumptionPskMisbindingError::ResumptionGroupIdMismatch);
    }
    // Rule 5: PSK epoch matches local epoch.
    if psk.epoch != view.local_epoch {
        return Err(ResumptionPskMisbindingError::ResumptionEpochMismatch);
    }
    // Rule 6: PSK nonce matches the rejoiner-declared nonce.
    if psk.psk_nonce != commit.declared_nonce {
        return Err(ResumptionPskMisbindingError::ResumptionNonceMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_group_id() -> Vec<u8> {
        vec![0x77_u8; ECRPM_GROUP_ID_LEN]
    }

    fn ok_other_group_id() -> Vec<u8> {
        vec![0x88_u8; ECRPM_GROUP_ID_LEN]
    }

    fn ok_nonce() -> Vec<u8> {
        vec![0x99_u8; ECRPM_PSK_NONCE_LEN]
    }

    fn ok_other_nonce() -> Vec<u8> {
        vec![0xAA_u8; ECRPM_PSK_NONCE_LEN]
    }

    fn ok_view() -> ResumptionView {
        ResumptionView { local_epoch: 100 }
    }

    fn ok_psk() -> PreSharedKeyId {
        PreSharedKeyId {
            psk_type: PskType::Resumption,
            group_id: ok_group_id(),
            epoch: 100,
            psk_nonce: ok_nonce(),
        }
    }

    fn ok_commit() -> ExternalCommitHeader {
        ExternalCommitHeader {
            group_id: ok_group_id(),
            psk_ids: vec![ok_psk()],
            declared_nonce: ok_nonce(),
        }
    }

    /// ECRPM-01 — short group_id (16 bytes) rejected — Rule 1.
    #[test]
    fn ecrpm_01_short_group_id_rejected() {
        let mut c = ok_commit();
        c.group_id = vec![0x77_u8; 16];
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::NonCanonicalGroupIdLength)
        );
    }

    /// ECRPM-02 — empty psk_ids rejected — Rule 2.
    #[test]
    fn ecrpm_02_empty_psk_ids_rejected() {
        let mut c = ok_commit();
        c.psk_ids.clear();
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::MissingResumptionPsk)
        );
    }

    /// ECRPM-03 — two psk_ids rejected — Rule 2.
    #[test]
    fn ecrpm_03_two_psk_ids_rejected() {
        let mut c = ok_commit();
        c.psk_ids.push(ok_psk());
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::MissingResumptionPsk)
        );
    }

    /// ECRPM-04 — External PSK in resumption slot rejected — Rule 3.
    #[test]
    fn ecrpm_04_external_psk_type_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].psk_type = PskType::External;
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::NotResumptionPskType)
        );
    }

    /// ECRPM-05 — PSK group_id mismatch rejected — Rule 4.
    #[test]
    fn ecrpm_05_group_id_mismatch_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].group_id = ok_other_group_id();
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::ResumptionGroupIdMismatch)
        );
    }

    /// ECRPM-06 — PSK epoch two steps back rejected — Rule 5.
    #[test]
    fn ecrpm_06_epoch_mismatch_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].epoch = 98; // local is 100
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::ResumptionEpochMismatch)
        );
    }

    /// ECRPM-07 — PSK nonce mismatch rejected — Rule 6.
    #[test]
    fn ecrpm_07_nonce_mismatch_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].psk_nonce = ok_other_nonce();
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::ResumptionNonceMismatch)
        );
    }

    /// ECRPM-08 — short psk_nonce (16 bytes) rejected — Rule 7.
    #[test]
    fn ecrpm_08_short_psk_nonce_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].psk_nonce = vec![0x99_u8; 16];
        c.declared_nonce = vec![0x99_u8; 16];
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::NonCanonicalPskNonceLength)
        );
    }

    /// ECRPM-09 — future epoch rejected — Rule 5.
    #[test]
    fn ecrpm_09_future_epoch_rejected() {
        let mut c = ok_commit();
        c.psk_ids[0].epoch = 101;
        assert_eq!(
            validate_external_commit_resumption_psk(&c, &ok_view()),
            Err(ResumptionPskMisbindingError::ResumptionEpochMismatch)
        );
    }

    /// ECRPM-10 — canonical external Commit accepted.
    #[test]
    fn ecrpm_10_canonical_external_commit_accepted() {
        assert_eq!(
            validate_external_commit_resumption_psk(&ok_commit(), &ok_view()),
            Ok(())
        );
    }
}
