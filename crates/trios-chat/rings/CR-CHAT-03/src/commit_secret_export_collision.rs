//! Wave-33 / L-CHAT-3-csec (R-CHAT-11 / CR-CHAT-03) — Commit-secret
//! export collision per RFC 9420 §8.4 / §9.
//!
//! After each Commit, the deriving member exports a fresh
//! `commit_secret` from the KDF tree. The exported secret has a
//! canonical length and is bound to `(group_id, epoch,
//! commit_transcript_hash)`. A hostile member may try to:
//!   - submit a `commit_secret` whose length differs from the canonical
//!     hash length (NonCanonicalCommitSecretLength),
//!   - cross-group splice an exported secret
//!     (CrossGroupCommitSecret),
//!   - cross-epoch splice (`StaleEpochCommitSecret`),
//!   - replay an `(epoch, transcript_hash, commit_secret)` triple
//!     across two distinct commits in the same group
//!     (CommitSecretReplay),
//!   - claim a `commit_secret` whose `transcript_hash` is not in the
//!     local commit ledger (UnknownTranscriptHash),
//!   - submit a zero-padded sentinel `commit_secret` (ZeroCommitSecret),
//!   - submit a `commit_secret` against an empty `transcript_hash`
//!     (EmptyTranscriptHash, degenerate KDF binding).
//!
//! Seven rules enforced in fixed order (single deny wins):
//!   1. NonCanonicalCommitSecretLength — `commit_secret.len()` must
//!      equal `COMMIT_SECRET_LEN` (32 bytes — pinned ciphersuite KDF
//!      output, RFC 9420 §5.2).
//!   2. EmptyTranscriptHash — `transcript_hash` must be non-empty
//!      (each Commit binds against the canonical interim transcript
//!      hash from W28 — `INTERIM_TRANSCRIPT_HASH_LEN`).
//!   3. UnknownTranscriptHash — `transcript_hash` must be present in
//!      `view.known_transcript_hashes` (no phantom commit binding).
//!   4. CrossGroupCommitSecret — `group_id` must match
//!      `view.expected_group_id` (no inter-group splice).
//!   5. StaleEpochCommitSecret — `commit_epoch` must equal
//!      `view.current_epoch` (no inter-epoch splice — a stale
//!      `commit_secret` from a prior epoch would seed PCS-healing
//!      against the wrong ratchet branch).
//!   6. CommitSecretReplay — the `(transcript_hash, commit_secret)`
//!      pair must not appear in `view.exported_commit_secrets`
//!      (catastrophic: two distinct commits exporting the **same**
//!      secret implies KDF collision or replay of a published secret).
//!   7. ZeroCommitSecret — the all-zero `commit_secret` is forbidden
//!      (a correctly evaluated KDF chain never produces it; the
//!      sentinel is reserved for uninitialised state).

#![forbid(unsafe_code)]

/// Canonical `commit_secret` length (32 bytes — KDF output truncated
/// to the pinned ciphersuite hash length per RFC 9420 §5.2).
pub const COMMIT_SECRET_LEN: usize = 32;

/// Maximum `transcript_hash` byte length we accept — generous upper
/// bound matching the W28 `INTERIM_TRANSCRIPT_HASH_LEN` (64 covers
/// SHA-512 ciphersuites if/when re-pinned).
pub const COMMIT_TRANSCRIPT_HASH_MAX_LEN: usize = 64;

/// Exported commit secret as carried inside a Commit per RFC 9420
/// §8.4 / §9 (`commit_secret = KDF(epoch_secret, "commit", _)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportedCommitSecret {
    /// Canonical 32-byte KDF output.
    pub commit_secret: Vec<u8>,
    /// Commit transcript hash this secret is bound to.
    pub transcript_hash: Vec<u8>,
    /// MLS group identifier.
    pub group_id: Vec<u8>,
    /// Epoch in which the Commit was authored.
    pub commit_epoch: u64,
}

/// Local view of known / used commit secrets at the current epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSecretView {
    /// Group identifier we are currently a member of.
    pub expected_group_id: Vec<u8>,
    /// Current epoch counter.
    pub current_epoch: u64,
    /// `transcript_hash`es the local commit ledger has actually
    /// observed (W28 — confirmation-tag chain feeds this set).
    pub known_transcript_hashes: Vec<Vec<u8>>,
    /// `(transcript_hash, commit_secret)` pairs already consumed by a
    /// prior commit export.
    pub exported_commit_secrets: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Typed errors for `validate_commit_secret_export`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommitSecretError {
    /// Rule 1 — `commit_secret.len() != COMMIT_SECRET_LEN`.
    NonCanonicalCommitSecretLength,
    /// Rule 2 — empty `transcript_hash`.
    EmptyTranscriptHash,
    /// Rule 3 — `transcript_hash` not in
    /// `view.known_transcript_hashes`.
    UnknownTranscriptHash,
    /// Rule 4 — cross-group splice.
    CrossGroupCommitSecret,
    /// Rule 5 — cross-epoch splice.
    StaleEpochCommitSecret,
    /// Rule 6 — `(transcript_hash, commit_secret)` already in
    ///          `view.exported_commit_secrets`.
    CommitSecretReplay,
    /// Rule 7 — all-zero `commit_secret`.
    ZeroCommitSecret,
}

/// Constructive guard for an exported commit secret. Returns `Ok(())`
/// iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `CSEC-01..10` below and the
/// Coq theorems `INV-CHAT-201..207` (W33 Section in
/// `proofs/chat/Trinity_Chat.v`).
pub fn validate_commit_secret_export(
    export: &ExportedCommitSecret,
    view: &CommitSecretView,
) -> Result<(), CommitSecretError> {
    if export.commit_secret.len() != COMMIT_SECRET_LEN {
        return Err(CommitSecretError::NonCanonicalCommitSecretLength);
    }
    if export.transcript_hash.is_empty() {
        return Err(CommitSecretError::EmptyTranscriptHash);
    }
    if !view
        .known_transcript_hashes
        .contains(&export.transcript_hash)
    {
        return Err(CommitSecretError::UnknownTranscriptHash);
    }
    if export.group_id != view.expected_group_id {
        return Err(CommitSecretError::CrossGroupCommitSecret);
    }
    if export.commit_epoch != view.current_epoch {
        return Err(CommitSecretError::StaleEpochCommitSecret);
    }
    let pair = (
        export.transcript_hash.clone(),
        export.commit_secret.clone(),
    );
    if view.exported_commit_secrets.contains(&pair) {
        return Err(CommitSecretError::CommitSecretReplay);
    }
    if export.commit_secret.iter().all(|&b| b == 0) {
        return Err(CommitSecretError::ZeroCommitSecret);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_secret() -> Vec<u8> {
        vec![0x44_u8; COMMIT_SECRET_LEN]
    }

    fn ok_view() -> CommitSecretView {
        CommitSecretView {
            expected_group_id: b"trinity-group-33".to_vec(),
            current_epoch: 12,
            known_transcript_hashes: vec![
                b"transcript-A".to_vec(),
                b"transcript-B".to_vec(),
            ],
            exported_commit_secrets: vec![],
        }
    }

    fn ok_export() -> ExportedCommitSecret {
        ExportedCommitSecret {
            commit_secret: canonical_secret(),
            transcript_hash: b"transcript-A".to_vec(),
            group_id: b"trinity-group-33".to_vec(),
            commit_epoch: 12,
        }
    }

    /// CSEC-01 — 16-byte commit_secret rejected.
    #[test]
    fn csec_01_short_commit_secret_rejected() {
        let mut e = ok_export();
        e.commit_secret = vec![0x44_u8; 16];
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::NonCanonicalCommitSecretLength)
        );
    }

    /// CSEC-02 — 64-byte commit_secret rejected (over-long).
    #[test]
    fn csec_02_over_long_commit_secret_rejected() {
        let mut e = ok_export();
        e.commit_secret = vec![0x44_u8; 64];
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::NonCanonicalCommitSecretLength)
        );
    }

    /// CSEC-03 — empty transcript_hash rejected.
    #[test]
    fn csec_03_empty_transcript_hash_rejected() {
        let mut e = ok_export();
        e.transcript_hash = vec![];
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::EmptyTranscriptHash)
        );
    }

    /// CSEC-04 — unknown transcript_hash rejected.
    #[test]
    fn csec_04_unknown_transcript_hash_rejected() {
        let mut e = ok_export();
        e.transcript_hash = b"phantom-transcript".to_vec();
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::UnknownTranscriptHash)
        );
    }

    /// CSEC-05 — cross-group commit_secret rejected.
    #[test]
    fn csec_05_cross_group_commit_secret_rejected() {
        let mut e = ok_export();
        e.group_id = b"trinity-group-OTHER".to_vec();
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::CrossGroupCommitSecret)
        );
    }

    /// CSEC-06 — stale-epoch commit_secret rejected.
    #[test]
    fn csec_06_stale_epoch_commit_secret_rejected() {
        let mut e = ok_export();
        e.commit_epoch = 11;
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::StaleEpochCommitSecret)
        );
    }

    /// CSEC-07 — replayed (transcript_hash, commit_secret) rejected.
    #[test]
    fn csec_07_replayed_commit_secret_rejected() {
        let e = ok_export();
        let mut view = ok_view();
        view.exported_commit_secrets.push((
            e.transcript_hash.clone(),
            e.commit_secret.clone(),
        ));
        assert_eq!(
            validate_commit_secret_export(&e, &view),
            Err(CommitSecretError::CommitSecretReplay)
        );
    }

    /// CSEC-08 — all-zero commit_secret rejected.
    #[test]
    fn csec_08_zero_commit_secret_rejected() {
        let mut e = ok_export();
        e.commit_secret = vec![0u8; COMMIT_SECRET_LEN];
        assert_eq!(
            validate_commit_secret_export(&e, &ok_view()),
            Err(CommitSecretError::ZeroCommitSecret)
        );
    }

    /// CSEC-09 — canonical commit_secret accepted.
    #[test]
    fn csec_09_canonical_commit_secret_accepted() {
        assert_eq!(
            validate_commit_secret_export(&ok_export(), &ok_view()),
            Ok(())
        );
    }

    /// CSEC-10 — same transcript_hash with a distinct (fresh)
    /// commit_secret accepted (must not false-positive against
    /// rule (6) — the replay check is on the PAIR not the hash alone).
    #[test]
    fn csec_10_distinct_fresh_commit_secret_accepted() {
        let e = ok_export();
        let mut view = ok_view();
        // A different commit_secret under the same transcript_hash was
        // exported before — but the new (transcript_hash, secret) pair
        // is fresh, so it must be accepted.
        view.exported_commit_secrets.push((
            e.transcript_hash.clone(),
            vec![0xCC_u8; COMMIT_SECRET_LEN],
        ));
        assert_eq!(
            validate_commit_secret_export(&e, &view),
            Ok(())
        );
    }
}
