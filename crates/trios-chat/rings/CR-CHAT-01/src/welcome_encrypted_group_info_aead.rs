//! Wave-32 / L-CHAT-1-wegi (R-CHAT-1 / CR-CHAT-01) — Welcome
//! `encrypted_group_info` AEAD integrity per RFC 9420 §12.4.3.
//!
//! A hostile distributor of a Welcome message may try to substitute,
//! splice, or truncate the AEAD-protected `encrypted_group_info`
//! envelope that carries the GroupInfo of the freshly created (or
//! joined) MLS group. This module is the constructive guard that
//! refuses every such substitution before the joiner ever attempts
//! AEAD.Open.
//!
//! Six rules enforced in fixed order (a single deny wins):
//!   1. NonCanonicalAeadNonceLength — `aead_nonce` must be exactly
//!      WELCOME_GROUP_INFO_AEAD_NONCE_LEN bytes (canonical 12-byte
//!      AEAD nonce for ciphersuites pinned at W11).
//!   2. ShortAeadCiphertext — `ciphertext` must carry at least the
//!      16-byte authentication tag (an envelope shorter than that is
//!      structurally impossible).
//!   3. CrossGroupAeadEnvelope — the envelope's `group_id` must
//!      equal the joiner's expected `group_id`. Splicing the AEAD
//!      ciphertext into a different group destroys the §12.4.3 key
//!      binding.
//!   4. StaleEpochAeadEnvelope — the envelope's `epoch` must equal
//!      the joiner's expected `epoch` (Welcome installs exactly one
//!      epoch — anything else means a stale or future Welcome was
//!      injected).
//!   5. ReusedAeadNonce — the (group_id, epoch, aead_nonce) triple
//!      must not appear in `view.used_welcome_aead_nonces` (any
//!      reuse breaks AEAD non-misuse and is a confidentiality
//!      hazard).
//!   6. ZeroAeadNonce — the all-zero nonce is forbidden (a correctly
//!      derived `welcome_nonce` from `welcome_secret` is never zero
//!      under a non-degenerate KDF).

#![forbid(unsafe_code)]

/// Canonical AEAD nonce length for the Welcome `encrypted_group_info`
/// envelope (12 bytes — AES-128-GCM / ChaCha20-Poly1305 / AES-256-GCM
/// all agree on a 12-byte nonce per RFC 9420 §5.2).
pub const WELCOME_GROUP_INFO_AEAD_NONCE_LEN: usize = 12;

/// Minimum ciphertext length — every AEAD output carries at least a
/// 16-byte authentication tag.
pub const WELCOME_GROUP_INFO_MIN_CT_LEN: usize = 16;

/// Welcome's AEAD-protected `encrypted_group_info` envelope. Field
/// shape mirrors `EncryptedGroupInfo` in RFC 9420 §12.4.3.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeAeadEnvelope {
    pub group_id: Vec<u8>,
    pub epoch: u64,
    pub aead_nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// Joiner's local view of the expected Welcome envelope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeAeadView {
    pub expected_group_id: Vec<u8>,
    pub expected_epoch: u64,
    /// Set of `(group_id, epoch, aead_nonce)` triples already
    /// consumed by this joiner — Welcomes are one-shot per epoch and
    /// per `welcome_secret`, so any reuse is a misuse.
    pub used_welcome_aead_nonces: Vec<(Vec<u8>, u64, Vec<u8>)>,
}

/// Typed errors for `validate_welcome_aead_envelope`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WelcomeAeadError {
    /// Rule 1 — `aead_nonce.len() != WELCOME_GROUP_INFO_AEAD_NONCE_LEN`.
    NonCanonicalAeadNonceLength,
    /// Rule 2 — `ciphertext.len() < WELCOME_GROUP_INFO_MIN_CT_LEN`.
    ShortAeadCiphertext,
    /// Rule 3 — envelope's `group_id` does not match the joiner.
    CrossGroupAeadEnvelope,
    /// Rule 4 — envelope's `epoch` does not match the joiner.
    StaleEpochAeadEnvelope,
    /// Rule 5 — `(group_id, epoch, aead_nonce)` already used.
    ReusedAeadNonce,
    /// Rule 6 — all-zero `aead_nonce`.
    ZeroAeadNonce,
}

/// Constructive guard for the Welcome `encrypted_group_info` AEAD
/// envelope. Returns `Ok(())` iff every rule (1)..(6) holds.
pub fn validate_welcome_aead_envelope(
    envelope: &WelcomeAeadEnvelope,
    view: &WelcomeAeadView,
) -> Result<(), WelcomeAeadError> {
    if envelope.aead_nonce.len() != WELCOME_GROUP_INFO_AEAD_NONCE_LEN {
        return Err(WelcomeAeadError::NonCanonicalAeadNonceLength);
    }
    if envelope.ciphertext.len() < WELCOME_GROUP_INFO_MIN_CT_LEN {
        return Err(WelcomeAeadError::ShortAeadCiphertext);
    }
    if envelope.group_id != view.expected_group_id {
        return Err(WelcomeAeadError::CrossGroupAeadEnvelope);
    }
    if envelope.epoch != view.expected_epoch {
        return Err(WelcomeAeadError::StaleEpochAeadEnvelope);
    }
    let triple = (
        envelope.group_id.clone(),
        envelope.epoch,
        envelope.aead_nonce.clone(),
    );
    if view.used_welcome_aead_nonces.contains(&triple) {
        return Err(WelcomeAeadError::ReusedAeadNonce);
    }
    if envelope.aead_nonce.iter().all(|&b| b == 0) {
        return Err(WelcomeAeadError::ZeroAeadNonce);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_nonce() -> Vec<u8> {
        vec![1u8; WELCOME_GROUP_INFO_AEAD_NONCE_LEN]
    }

    fn canonical_ct() -> Vec<u8> {
        // 32 bytes — payload (16) + tag (16).
        let mut v = vec![0xAA_u8; 16];
        v.extend_from_slice(&[0xBB_u8; 16]);
        v
    }

    fn ok_view() -> WelcomeAeadView {
        WelcomeAeadView {
            expected_group_id: b"trinity-group-32".to_vec(),
            expected_epoch: 7,
            used_welcome_aead_nonces: vec![],
        }
    }

    fn ok_envelope() -> WelcomeAeadEnvelope {
        WelcomeAeadEnvelope {
            group_id: b"trinity-group-32".to_vec(),
            epoch: 7,
            aead_nonce: canonical_nonce(),
            ciphertext: canonical_ct(),
        }
    }

    /// WEGI-01 — 8-byte aead_nonce rejected.
    #[test]
    fn wegi_01_short_aead_nonce_rejected() {
        let mut env = ok_envelope();
        env.aead_nonce = vec![1u8; 8];
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::NonCanonicalAeadNonceLength)
        );
    }

    /// WEGI-02 — 32-byte aead_nonce rejected (over-long).
    #[test]
    fn wegi_02_over_long_aead_nonce_rejected() {
        let mut env = ok_envelope();
        env.aead_nonce = vec![1u8; 32];
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::NonCanonicalAeadNonceLength)
        );
    }

    /// WEGI-03 — ciphertext shorter than 16 bytes rejected.
    #[test]
    fn wegi_03_short_ciphertext_rejected() {
        let mut env = ok_envelope();
        env.ciphertext = vec![0xAB_u8; 15];
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::ShortAeadCiphertext)
        );
    }

    /// WEGI-04 — cross-group AEAD envelope rejected.
    #[test]
    fn wegi_04_cross_group_envelope_rejected() {
        let mut env = ok_envelope();
        env.group_id = b"trinity-group-OTHER".to_vec();
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::CrossGroupAeadEnvelope)
        );
    }

    /// WEGI-05 — stale-epoch envelope rejected (past epoch).
    #[test]
    fn wegi_05_stale_epoch_envelope_rejected() {
        let mut env = ok_envelope();
        env.epoch = 6;
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::StaleEpochAeadEnvelope)
        );
    }

    /// WEGI-06 — future-epoch envelope rejected.
    #[test]
    fn wegi_06_future_epoch_envelope_rejected() {
        let mut env = ok_envelope();
        env.epoch = 8;
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::StaleEpochAeadEnvelope)
        );
    }

    /// WEGI-07 — reused (group_id, epoch, aead_nonce) triple rejected.
    #[test]
    fn wegi_07_reused_aead_nonce_rejected() {
        let env = ok_envelope();
        let mut view = ok_view();
        view.used_welcome_aead_nonces.push((
            env.group_id.clone(),
            env.epoch,
            env.aead_nonce.clone(),
        ));
        assert_eq!(
            validate_welcome_aead_envelope(&env, &view),
            Err(WelcomeAeadError::ReusedAeadNonce)
        );
    }

    /// WEGI-08 — all-zero aead_nonce rejected.
    #[test]
    fn wegi_08_zero_aead_nonce_rejected() {
        let mut env = ok_envelope();
        env.aead_nonce = vec![0u8; WELCOME_GROUP_INFO_AEAD_NONCE_LEN];
        assert_eq!(
            validate_welcome_aead_envelope(&env, &ok_view()),
            Err(WelcomeAeadError::ZeroAeadNonce)
        );
    }

    /// WEGI-09 — canonical fresh Welcome envelope accepted.
    #[test]
    fn wegi_09_canonical_envelope_accepted() {
        assert_eq!(
            validate_welcome_aead_envelope(&ok_envelope(), &ok_view()),
            Ok(())
        );
    }

    /// WEGI-10 — distinct aead_nonce reusing same (group_id, epoch) accepted.
    #[test]
    fn wegi_10_distinct_aead_nonce_same_group_epoch_accepted() {
        let env = ok_envelope();
        let mut view = ok_view();
        // Different nonce (same group_id+epoch) already used — must not
        // false-positive against rule (5).
        view.used_welcome_aead_nonces.push((
            env.group_id.clone(),
            env.epoch,
            vec![9u8; WELCOME_GROUP_INFO_AEAD_NONCE_LEN],
        ));
        assert_eq!(
            validate_welcome_aead_envelope(&env, &view),
            Ok(())
        );
    }
}
