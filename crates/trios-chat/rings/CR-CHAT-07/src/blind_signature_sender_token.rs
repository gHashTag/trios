//! Wave-34 / L-CHAT-7-bsst (R-CHAT-10 / CR-CHAT-07) — Blind-signature
//! sender token per NDSS 2021 "Improving Signal's Sealed Sender" §IV-D
//! (anonymous token authorisation, Chaum-style blind signatures).
//!
//! The sister lane `ephemeral_mailbox_unlinkability` (CR-CHAT-01)
//! ensures that the relay sees only `(mailbox_token, padded_envelope)`.
//! But that alone does not stop the relay from rate-limiting or
//! denying service: every envelope still needs *some* authorisation
//! token. NDSS 2021 §IV-D proposes Chaum-style blind signatures: the
//! receiver issues a blind signature over a one-shot token nonce, the
//! sender unblinds it, and the relay then verifies the signature *over
//! the unblinded nonce* — meaning the relay can confirm the token was
//! issued by an authorised receiver without learning *which* issuance
//! request the token corresponds to. This breaks the relay's ability
//! to correlate `(receiver, sender)` pairs even when it sees all
//! traffic.
//!
//! This lane enforces the verification-side invariants for a single
//! sender token. The Rust `validate_blind_signature_sender_token` is
//! the constructive guard called by the relay before it accepts a
//! sender-side envelope into the queue. A single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalTokenNonceLength — `token_nonce.len()` must equal
//!      `BLIND_TOKEN_NONCE_LEN` (32 bytes — output of the sender's
//!      blinding factor unblinding per §IV-D Eq. 5).
//!   2. NonCanonicalSignatureLength — `signature.len()` must equal
//!      `BLIND_SIGNATURE_LEN` (256 bytes — RSA-2048 / RSA-FDH per
//!      RFC 8017 §8.2, the standard FDH instantiation referenced by
//!      NDSS 2021 §IV-D).
//!   3. UnknownIssuerPublicKey — `issuer_pubkey_id` must be present
//!      in `view.trusted_issuers` (the relay must recognise the
//!      receiver-side issuer public key whose private half issued the
//!      blind signature).
//!   4. ExpiredIssuerEpoch — `view.current_epoch > issuer_expiry`
//!      is rejected (issuer keys are rotated per §IV-E so revoked
//!      issuers cannot keep signing).
//!   5. TokenNonceReuse — `token_nonce` must NOT be present in
//!      `view.spent_nonces` (one-shot — the same blind-signed token
//!      cannot be spent twice; this is the anti-double-spend rail).
//!   6. ZeroTokenNonce — the all-zero `token_nonce` is forbidden
//!      (a correctly evaluated unblinding never produces it).
//!   7. SignatureVerificationFailed — the signature must verify
//!      against the issuer's public key over `token_nonce`. We model
//!      verification as a table-driven check `view.valid_signatures`
//!      containing pre-computed `(issuer_pubkey_id, token_nonce,
//!      signature)` triples that pass RSA-FDH; absence ⇒ deny.

#![forbid(unsafe_code)]

/// Canonical token-nonce length (32 bytes — receiver-side
/// unblinded nonce per NDSS 2021 §IV-D Eq. 5).
pub const BLIND_TOKEN_NONCE_LEN: usize = 32;

/// Canonical RSA-FDH blind signature length (256 bytes — RSA-2048
/// per RFC 8017 §8.2).
pub const BLIND_SIGNATURE_LEN: usize = 256;

/// A single sender token presented to the relay together with an
/// ephemeral-mailbox envelope per NDSS 2021 §IV-D.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlindSenderToken {
    /// Identifier of the receiver-side issuer key that signed this
    /// token (e.g. a 16-byte digest of the issuer's RSA public key).
    pub issuer_pubkey_id: Vec<u8>,
    /// Unblinded one-shot nonce (32 bytes).
    pub token_nonce: Vec<u8>,
    /// RSA-FDH signature over `token_nonce` (256 bytes).
    pub signature: Vec<u8>,
}

/// Relay-side view of issuer + double-spend state at the current
/// epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlindTokenView {
    /// The relay's local epoch.
    pub current_epoch: u64,
    /// Issuer keys the relay currently trusts. Each entry is
    /// `(issuer_pubkey_id, expiry_epoch)`.
    pub trusted_issuers: Vec<(Vec<u8>, u64)>,
    /// Token nonces already spent (anti-double-spend ledger).
    pub spent_nonces: Vec<Vec<u8>>,
    /// Pre-computed valid signature triples
    /// `(issuer_pubkey_id, token_nonce, signature)` — surrogate for
    /// the RSA-FDH verifier in a pure constructive guard.
    pub valid_signatures: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>,
}

/// Typed errors for `validate_blind_signature_sender_token`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlindTokenError {
    /// Rule 1 — `token_nonce.len() != BLIND_TOKEN_NONCE_LEN`.
    NonCanonicalTokenNonceLength,
    /// Rule 2 — `signature.len() != BLIND_SIGNATURE_LEN`.
    NonCanonicalSignatureLength,
    /// Rule 3 — issuer key not in `view.trusted_issuers`.
    UnknownIssuerPublicKey,
    /// Rule 4 — `current_epoch > issuer_expiry`.
    ExpiredIssuerEpoch,
    /// Rule 5 — nonce already in `view.spent_nonces`.
    TokenNonceReuse,
    /// Rule 6 — all-zero token nonce.
    ZeroTokenNonce,
    /// Rule 7 — RSA-FDH signature does not verify.
    SignatureVerificationFailed,
}

/// Constructive guard for a single blind-signature sender token.
/// Returns `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `BSST-01..10` below and the
/// Coq theorems `INV-CHAT-213..217` in the W34 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_blind_signature_sender_token(
    token: &BlindSenderToken,
    view: &BlindTokenView,
) -> Result<(), BlindTokenError> {
    if token.token_nonce.len() != BLIND_TOKEN_NONCE_LEN {
        return Err(BlindTokenError::NonCanonicalTokenNonceLength);
    }
    if token.signature.len() != BLIND_SIGNATURE_LEN {
        return Err(BlindTokenError::NonCanonicalSignatureLength);
    }
    let issuer = view
        .trusted_issuers
        .iter()
        .find(|(id, _)| id == &token.issuer_pubkey_id);
    let (_, expiry) = match issuer {
        Some(entry) => entry,
        None => return Err(BlindTokenError::UnknownIssuerPublicKey),
    };
    if view.current_epoch > *expiry {
        return Err(BlindTokenError::ExpiredIssuerEpoch);
    }
    if view.spent_nonces.contains(&token.token_nonce) {
        return Err(BlindTokenError::TokenNonceReuse);
    }
    if token.token_nonce.iter().all(|&b| b == 0) {
        return Err(BlindTokenError::ZeroTokenNonce);
    }
    let sig_ok = view.valid_signatures.iter().any(|(id, nonce, sig)| {
        id == &token.issuer_pubkey_id
            && nonce == &token.token_nonce
            && sig == &token.signature
    });
    if !sig_ok {
        return Err(BlindTokenError::SignatureVerificationFailed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issuer_id_a() -> Vec<u8> {
        b"issuer-A-pubkey-id-16".to_vec()
    }

    fn nonce_a() -> Vec<u8> {
        vec![0xC3_u8; BLIND_TOKEN_NONCE_LEN]
    }

    fn sig_a() -> Vec<u8> {
        vec![0x55_u8; BLIND_SIGNATURE_LEN]
    }

    fn ok_view() -> BlindTokenView {
        BlindTokenView {
            current_epoch: 10,
            trusted_issuers: vec![(issuer_id_a(), 20)],
            spent_nonces: vec![],
            valid_signatures: vec![(issuer_id_a(), nonce_a(), sig_a())],
        }
    }

    fn ok_token() -> BlindSenderToken {
        BlindSenderToken {
            issuer_pubkey_id: issuer_id_a(),
            token_nonce: nonce_a(),
            signature: sig_a(),
        }
    }

    /// BSST-01 — 16-byte token_nonce rejected.
    #[test]
    fn bsst_01_short_nonce_rejected() {
        let mut t = ok_token();
        t.token_nonce = vec![0xC3_u8; 16];
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::NonCanonicalTokenNonceLength)
        );
    }

    /// BSST-02 — 64-byte token_nonce rejected.
    #[test]
    fn bsst_02_over_long_nonce_rejected() {
        let mut t = ok_token();
        t.token_nonce = vec![0xC3_u8; 64];
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::NonCanonicalTokenNonceLength)
        );
    }

    /// BSST-03 — 128-byte signature rejected (under-long).
    #[test]
    fn bsst_03_short_signature_rejected() {
        let mut t = ok_token();
        t.signature = vec![0x55_u8; 128];
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::NonCanonicalSignatureLength)
        );
    }

    /// BSST-04 — 512-byte signature rejected (over-long).
    #[test]
    fn bsst_04_over_long_signature_rejected() {
        let mut t = ok_token();
        t.signature = vec![0x55_u8; 512];
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::NonCanonicalSignatureLength)
        );
    }

    /// BSST-05 — unknown issuer rejected.
    #[test]
    fn bsst_05_unknown_issuer_rejected() {
        let mut t = ok_token();
        t.issuer_pubkey_id = b"phantom-issuer-Z-1234".to_vec();
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::UnknownIssuerPublicKey)
        );
    }

    /// BSST-06 — expired issuer epoch rejected.
    #[test]
    fn bsst_06_expired_issuer_rejected() {
        let t = ok_token();
        let mut view = ok_view();
        view.current_epoch = 21; // past expiry 20
        assert_eq!(
            validate_blind_signature_sender_token(&t, &view),
            Err(BlindTokenError::ExpiredIssuerEpoch)
        );
    }

    /// BSST-07 — replayed token nonce (double-spend) rejected — SDA
    /// defence anti-correlation core invariant.
    #[test]
    fn bsst_07_nonce_reuse_rejected() {
        let t = ok_token();
        let mut view = ok_view();
        view.spent_nonces.push(nonce_a());
        assert_eq!(
            validate_blind_signature_sender_token(&t, &view),
            Err(BlindTokenError::TokenNonceReuse)
        );
    }

    /// BSST-08 — all-zero token_nonce rejected (sentinel).
    #[test]
    fn bsst_08_zero_nonce_rejected() {
        let mut t = ok_token();
        let zero = vec![0u8; BLIND_TOKEN_NONCE_LEN];
        t.token_nonce = zero.clone();
        let mut view = ok_view();
        view.valid_signatures
            .push((issuer_id_a(), zero, sig_a()));
        assert_eq!(
            validate_blind_signature_sender_token(&t, &view),
            Err(BlindTokenError::ZeroTokenNonce)
        );
    }

    /// BSST-09 — signature does not verify under issuer pubkey
    /// rejected.
    #[test]
    fn bsst_09_bad_signature_rejected() {
        let mut t = ok_token();
        t.signature = vec![0xFF_u8; BLIND_SIGNATURE_LEN];
        assert_eq!(
            validate_blind_signature_sender_token(&t, &ok_view()),
            Err(BlindTokenError::SignatureVerificationFailed)
        );
    }

    /// BSST-10 — canonical token accepted.
    #[test]
    fn bsst_10_canonical_token_accepted() {
        assert_eq!(
            validate_blind_signature_sender_token(&ok_token(), &ok_view()),
            Ok(())
        );
    }
}
