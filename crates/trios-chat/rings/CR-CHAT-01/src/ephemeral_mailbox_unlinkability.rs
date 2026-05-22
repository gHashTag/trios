//! Wave-34 / L-CHAT-4-emu (R-CHAT-3 / CR-CHAT-01) — Ephemeral mailbox
//! unlinkability per NDSS 2021 "Improving Signal's Sealed Sender" §IV
//! (Statistical Disclosure Attack mitigation).
//!
//! Signal's sealed sender hides the sender identity from the relay, but
//! the relay still observes the *receiver*'s long-term mailbox (account
//! ID). The NDSS 2021 paper (Martiny et al.) showed that an honest-but-
//! curious relay can de-anonymise the sender via a Statistical
//! Disclosure Attack after observing as few as ~5 messages, because the
//! receiver's mailbox is reused across all envelopes.
//!
//! The fix proposed in §IV is to route every sealed-sender envelope
//! through a *one-shot ephemeral mailbox*: the receiver pre-publishes a
//! pool of fresh mailbox tokens, each token is bound to (a) a single
//! receiver, (b) a freshness window, (c) a per-token blind-signature
//! capability (the W34 sister lane `blind_signature_sender_token`), and
//! must be (d) used at most once. The relay sees only
//! `(mailbox_token[32], padded_envelope)` — the long-term receiver
//! identifier never appears.
//!
//! This lane enforces the consumption-side invariants for a single
//! envelope. The Rust `validate_ephemeral_mailbox_envelope` is the
//! constructive guard called by the receiver-side router before it
//! decrypts the sealed envelope. A single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalMailboxTokenLength — `mailbox_token.len()` must
//!      equal `EPHEMERAL_MAILBOX_TOKEN_LEN` (32 bytes — output of the
//!      receiver's KDF derivation per §IV-B).
//!   2. UnknownMailboxToken — `mailbox_token` must be present in
//!      `view.published_tokens` (no phantom mailbox — the receiver
//!      must have pre-published it).
//!   3. MailboxTokenWrongReceiver — `(mailbox_token, claimed_receiver)`
//!      must match the receiver the token was actually published for
//!      (`view.token_owner[token] == claimed_receiver`).
//!   4. StaleMailboxToken — `view.current_epoch > token_expiry_epoch`
//!      is rejected (mailbox lifetime per §IV-C is bounded; expired
//!      tokens cannot be replayed in a later epoch to splice in
//!      old observations).
//!   5. MailboxTokenReuse — `mailbox_token` must NOT be present in
//!      `view.consumed_tokens` (one-shot — the SDA defence collapses
//!      the moment a token is reused, see §V-A).
//!   6. ZeroMailboxToken — the all-zero `mailbox_token` is forbidden
//!      (a correctly evaluated KDF never produces it — sentinel that
//!      must be denied at the boundary).
//!   7. EnvelopeBindingMismatch — `envelope_binding_tag` (HKDF-Expand
//!      of `mailbox_token` || `padded_envelope_hash` per §IV-B Eq. 3)
//!      must equal `view.expected_binding(mailbox_token, envelope_hash)`
//!      — prevents an attacker who steals a single mailbox token from
//!      pairing it with a different envelope.

#![forbid(unsafe_code)]

/// Canonical mailbox-token length (32 bytes — output of the receiver's
/// HKDF derivation per NDSS 2021 §IV-B; matched by W6
/// `dest_hash` half-rate).
pub const EPHEMERAL_MAILBOX_TOKEN_LEN: usize = 32;

/// Canonical envelope-binding tag length (32 bytes — HMAC-SHA-256
/// output per NDSS 2021 §IV-B Eq. 3).
pub const ENVELOPE_BINDING_TAG_LEN: usize = 32;

/// A single sealed-sender envelope arriving through an ephemeral
/// mailbox per NDSS 2021 §IV.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EphemeralMailboxEnvelope {
    /// The one-shot mailbox token (32 bytes).
    pub mailbox_token: Vec<u8>,
    /// The receiver this envelope claims to be destined for.
    pub claimed_receiver: Vec<u8>,
    /// HMAC binding (`mailbox_token`, `padded_envelope_hash`).
    pub envelope_binding_tag: Vec<u8>,
    /// SHA-256 of the padded ciphertext (used to recompute the
    /// expected binding).
    pub envelope_hash: Vec<u8>,
}

/// Receiver-side view of mailbox state at the current epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EphemeralMailboxView {
    /// The relay's local epoch.
    pub current_epoch: u64,
    /// Mailbox tokens the receiver has published. Each entry is
    /// `(token, owner_receiver_id, expiry_epoch)`.
    pub published_tokens: Vec<(Vec<u8>, Vec<u8>, u64)>,
    /// Mailbox tokens already consumed in a previous envelope. The
    /// router MUST refuse to consume the same token twice — this is
    /// the SDA defence.
    pub consumed_tokens: Vec<Vec<u8>>,
    /// Expected binding tag for `(mailbox_token, envelope_hash)`
    /// pairs the receiver has pre-computed (or that the receiver's
    /// router recomputes on the fly — both shapes verify against this
    /// table for the pure constructive guard).
    pub expected_binding: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>,
}

/// Typed errors for `validate_ephemeral_mailbox_envelope`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EphemeralMailboxError {
    /// Rule 1 — `mailbox_token.len() != EPHEMERAL_MAILBOX_TOKEN_LEN`.
    NonCanonicalMailboxTokenLength,
    /// Rule 2 — token not in `view.published_tokens`.
    UnknownMailboxToken,
    /// Rule 3 — token published for a different receiver.
    MailboxTokenWrongReceiver,
    /// Rule 4 — `view.current_epoch > expiry_epoch`.
    StaleMailboxToken,
    /// Rule 5 — token already in `view.consumed_tokens`.
    MailboxTokenReuse,
    /// Rule 6 — all-zero mailbox token.
    ZeroMailboxToken,
    /// Rule 7 — binding tag mismatch.
    EnvelopeBindingMismatch,
}

/// Constructive guard for a single ephemeral-mailbox envelope. Returns
/// `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `EMU-01..10` below and the
/// Coq theorems `INV-CHAT-208..212` in the W34 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_ephemeral_mailbox_envelope(
    envelope: &EphemeralMailboxEnvelope,
    view: &EphemeralMailboxView,
) -> Result<(), EphemeralMailboxError> {
    if envelope.mailbox_token.len() != EPHEMERAL_MAILBOX_TOKEN_LEN {
        return Err(EphemeralMailboxError::NonCanonicalMailboxTokenLength);
    }
    let published = view
        .published_tokens
        .iter()
        .find(|(t, _, _)| t == &envelope.mailbox_token);
    let (_, owner, expiry) = match published {
        Some(entry) => entry,
        None => {
            return Err(EphemeralMailboxError::UnknownMailboxToken);
        }
    };
    if owner != &envelope.claimed_receiver {
        return Err(EphemeralMailboxError::MailboxTokenWrongReceiver);
    }
    if view.current_epoch > *expiry {
        return Err(EphemeralMailboxError::StaleMailboxToken);
    }
    if view.consumed_tokens.contains(&envelope.mailbox_token) {
        return Err(EphemeralMailboxError::MailboxTokenReuse);
    }
    if envelope.mailbox_token.iter().all(|&b| b == 0) {
        return Err(EphemeralMailboxError::ZeroMailboxToken);
    }
    let expected = view.expected_binding.iter().find(|(tok, hash, _)| {
        tok == &envelope.mailbox_token && hash == &envelope.envelope_hash
    });
    match expected {
        Some((_, _, tag)) if tag == &envelope.envelope_binding_tag => Ok(()),
        _ => Err(EphemeralMailboxError::EnvelopeBindingMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token_a() -> Vec<u8> {
        vec![0xA1_u8; EPHEMERAL_MAILBOX_TOKEN_LEN]
    }

    fn token_b() -> Vec<u8> {
        vec![0xB2_u8; EPHEMERAL_MAILBOX_TOKEN_LEN]
    }

    fn binding_a() -> Vec<u8> {
        vec![0x77_u8; ENVELOPE_BINDING_TAG_LEN]
    }

    fn envelope_hash_a() -> Vec<u8> {
        vec![0x33_u8; 32]
    }

    fn ok_view() -> EphemeralMailboxView {
        EphemeralMailboxView {
            current_epoch: 10,
            published_tokens: vec![(token_a(), b"alice".to_vec(), 20)],
            consumed_tokens: vec![],
            expected_binding: vec![(token_a(), envelope_hash_a(), binding_a())],
        }
    }

    fn ok_envelope() -> EphemeralMailboxEnvelope {
        EphemeralMailboxEnvelope {
            mailbox_token: token_a(),
            claimed_receiver: b"alice".to_vec(),
            envelope_binding_tag: binding_a(),
            envelope_hash: envelope_hash_a(),
        }
    }

    /// EMU-01 — 16-byte mailbox_token rejected.
    #[test]
    fn emu_01_short_token_rejected() {
        let mut e = ok_envelope();
        e.mailbox_token = vec![0xA1_u8; 16];
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::NonCanonicalMailboxTokenLength)
        );
    }

    /// EMU-02 — 64-byte mailbox_token rejected (over-long).
    #[test]
    fn emu_02_over_long_token_rejected() {
        let mut e = ok_envelope();
        e.mailbox_token = vec![0xA1_u8; 64];
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::NonCanonicalMailboxTokenLength)
        );
    }

    /// EMU-03 — unknown mailbox_token rejected.
    #[test]
    fn emu_03_unknown_token_rejected() {
        let mut e = ok_envelope();
        e.mailbox_token = token_b();
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::UnknownMailboxToken)
        );
    }

    /// EMU-04 — token published for different receiver rejected.
    #[test]
    fn emu_04_wrong_receiver_rejected() {
        let mut e = ok_envelope();
        e.claimed_receiver = b"bob".to_vec();
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::MailboxTokenWrongReceiver)
        );
    }

    /// EMU-05 — expired mailbox token rejected.
    #[test]
    fn emu_05_stale_token_rejected() {
        let e = ok_envelope();
        let mut view = ok_view();
        view.current_epoch = 21; // past expiry 20
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &view),
            Err(EphemeralMailboxError::StaleMailboxToken)
        );
    }

    /// EMU-06 — replayed (consumed) mailbox token rejected — SDA
    /// defence core invariant.
    #[test]
    fn emu_06_token_reuse_rejected() {
        let e = ok_envelope();
        let mut view = ok_view();
        view.consumed_tokens.push(token_a());
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &view),
            Err(EphemeralMailboxError::MailboxTokenReuse)
        );
    }

    /// EMU-07 — all-zero mailbox token rejected (must be pre-
    /// published via UnknownMailboxToken; we also assert at the
    /// boundary that even if a degenerate KDF output were ever
    /// published, the all-zero sentinel is forbidden).
    #[test]
    fn emu_07_zero_token_rejected() {
        let mut e = ok_envelope();
        let zero = vec![0u8; EPHEMERAL_MAILBOX_TOKEN_LEN];
        e.mailbox_token = zero.clone();
        let mut view = ok_view();
        // Make zero appear in published + binding, isolating Rule 6.
        view.published_tokens
            .push((zero.clone(), b"alice".to_vec(), 20));
        view.expected_binding
            .push((zero.clone(), envelope_hash_a(), binding_a()));
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &view),
            Err(EphemeralMailboxError::ZeroMailboxToken)
        );
    }

    /// EMU-08 — binding tag mismatch rejected (token stolen and
    /// paired with a different envelope).
    #[test]
    fn emu_08_binding_mismatch_rejected() {
        let mut e = ok_envelope();
        e.envelope_binding_tag = vec![0xFF_u8; ENVELOPE_BINDING_TAG_LEN];
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::EnvelopeBindingMismatch)
        );
    }

    /// EMU-09 — binding tag absent (no expected binding for
    /// this `(token, envelope_hash)` pair) rejected.
    #[test]
    fn emu_09_missing_binding_rejected() {
        let mut e = ok_envelope();
        e.envelope_hash = vec![0x99_u8; 32];
        assert_eq!(
            validate_ephemeral_mailbox_envelope(&e, &ok_view()),
            Err(EphemeralMailboxError::EnvelopeBindingMismatch)
        );
    }

    /// EMU-10 — canonical envelope accepted.
    #[test]
    fn emu_10_canonical_envelope_accepted() {
        assert_eq!(
            validate_ephemeral_mailbox_envelope(
                &ok_envelope(),
                &ok_view()
            ),
            Ok(())
        );
    }
}
