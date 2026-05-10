//! L-CHAT-1-revoke · Wave-15 — Identity-key revocation with grace window.
//!
//! When a user's long-term Ed25519 identity key is compromised, lost, or
//! voluntarily rotated, the protocol MUST allow them to publish a
//! *revocation certificate* that pins the moment after which all messages
//! signed by the old key are rejected. Per **R-CHAT-1** the revocation
//! cert is itself signed by the very key it revokes (so the "self-issued
//! at compromise time" semantics is preserved) and carries:
//!
//! 1. The 32-byte Ed25519 public key being revoked.
//! 2. A monotonically-increasing `revoked_at` timestamp (seconds since
//!    Unix epoch).
//! 3. A 1-byte reason code (compromise / rotate / lost).
//! 4. An Ed25519 signature over the (key‖ts‖reason) tuple.
//!
//! The verifier maintains a [`RevocationLedger`] (an in-memory monotone
//! map `IdKey → revoked_at`). [`verify_identity_with_grace`] takes a
//! claimed identity key + the signed-at-time of the message and the
//! current clock, and:
//!
//! * If no revocation cert is on file → accept.
//! * If `signed_at` strictly precedes `revoked_at` → accept (the message
//!   was created *before* the key was revoked).
//! * If `signed_at >= revoked_at` AND `now <= revoked_at + grace_secs` →
//!   accept (the per-key grace window).
//! * Otherwise → reject with [`Error::Invariant("identity_revoked")`].
//!
//! `[VERIFIED]` — round-trip + 6 falsifier-style REV-01..06 tests below.
//! `[ASPIRATIONAL]` — distribution / ledger replication is out of scope
//!   for this Silver-tier ring (lives in CR-CHAT-05 at-rest store).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ZERO-METADATA · IDENTITY-REVOKE`

use std::collections::BTreeMap;

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use trios_chat_cr_chat_00::{Error, Result};

/// Reason code stamped into the revocation certificate. Single byte to
/// keep the signed body fixed-length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RevocationReason {
    /// The long-term key was compromised (default — strictest).
    Compromise = 0,
    /// Voluntary rotation — old key is retired but not believed leaked.
    Rotate = 1,
    /// The user lost access to the secret material.
    Lost = 2,
}

impl RevocationReason {
    fn to_byte(self) -> u8 {
        self as u8
    }
}

/// Self-signed revocation certificate. The signing key MUST equal the
/// public key being revoked — there is no notion of an external
/// revocation authority in Trinity Chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationCert {
    /// The 32-byte Ed25519 public key being revoked.
    pub revoked_key: [u8; 32],
    /// Seconds since Unix epoch at which the key becomes invalid.
    pub revoked_at: u64,
    /// Reason code (1 byte).
    pub reason: RevocationReason,
    /// Ed25519 signature over the canonical body
    /// `revoked_key ‖ revoked_at_le ‖ reason_byte` (32 + 8 + 1 = 41 bytes).
    pub signature: [u8; 64],
}

impl RevocationCert {
    /// Canonical signing input — concatenation of the three fields in
    /// little-endian, fixed length (41 bytes).
    pub fn signed_body(revoked_key: &[u8; 32], revoked_at: u64, reason: RevocationReason) -> [u8; 41] {
        let mut buf = [0u8; 41];
        buf[..32].copy_from_slice(revoked_key);
        buf[32..40].copy_from_slice(&revoked_at.to_le_bytes());
        buf[40] = reason.to_byte();
        buf
    }

    /// Self-issue a revocation certificate using the *very* signing key
    /// the cert revokes — mirrors R-CHAT-1 semantics.
    pub fn issue_self(signer: &SigningKey, revoked_at: u64, reason: RevocationReason) -> Self {
        let revoked_key = signer.verifying_key().to_bytes();
        let body = Self::signed_body(&revoked_key, revoked_at, reason);
        let sig: Signature = signer.sign(&body);
        Self {
            revoked_key,
            revoked_at,
            reason,
            signature: sig.to_bytes(),
        }
    }

    /// Verify the certificate is well-formed: signature must validate
    /// under the very key the cert revokes.
    ///
    /// Errors:
    /// - `Error::Invariant("revocation_invalid_pubkey")` if the
    ///   `revoked_key` bytes are not a valid Ed25519 point.
    /// - `Error::Invariant("revocation_invalid_signature")` on signature
    ///   verification failure.
    pub fn verify_self_signed(&self) -> Result<()> {
        let vk = VerifyingKey::from_bytes(&self.revoked_key)
            .map_err(|_| Error::Invariant("revocation_invalid_pubkey"))?;
        let body = Self::signed_body(&self.revoked_key, self.revoked_at, self.reason);
        let sig = Signature::from_bytes(&self.signature);
        vk.verify(&body, &sig)
            .map_err(|_| Error::Invariant("revocation_invalid_signature"))?;
        Ok(())
    }
}

/// Monotone ledger of accepted revocations. Once a key is revoked the
/// `revoked_at` timestamp can only move *earlier* (a sender realising
/// they were compromised earlier than they thought) — never later, so
/// a compromised-then-replayed cert with a later timestamp is rejected.
#[derive(Debug, Default)]
pub struct RevocationLedger {
    inner: BTreeMap<[u8; 32], u64>,
}

impl RevocationLedger {
    /// Empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of revocations on file.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the ledger is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Lookup a key's revoked-at timestamp.
    pub fn revoked_at(&self, key: &[u8; 32]) -> Option<u64> {
        self.inner.get(key).copied()
    }

    /// Submit a self-signed cert. The cert MUST verify; the new entry
    /// is accepted only if (a) no entry exists, or (b) the new
    /// `revoked_at` is *strictly earlier* than the stored one (monotone
    /// rule — earlier compromise wins).
    ///
    /// Errors:
    /// - any error from [`RevocationCert::verify_self_signed`].
    /// - `Error::Invariant("revocation_replay_rejected")` if the same or a
    ///   later timestamp is submitted for a key already on file.
    pub fn submit(&mut self, cert: &RevocationCert) -> Result<()> {
        cert.verify_self_signed()?;
        match self.inner.get(&cert.revoked_key) {
            Some(&existing) if cert.revoked_at >= existing => {
                Err(Error::Invariant("revocation_replay_rejected"))
            }
            _ => {
                self.inner.insert(cert.revoked_key, cert.revoked_at);
                Ok(())
            }
        }
    }
}

/// Verify an identity-signed event under the revocation ledger plus a
/// per-key grace window.
///
/// Parameters:
/// - `ledger`        — current revocation state.
/// - `claimed_key`   — public key that signed the event.
/// - `signed_at`     — seconds since epoch when the event was signed.
/// - `now`           — current verifier clock (seconds since epoch).
/// - `grace_secs`    — max delay after `revoked_at` during which a
///   pre-revocation message is still accepted (to allow in-flight
///   envelopes to settle).
///
/// Returns:
/// - `Ok(())` on accept.
/// - `Err(Error::Invariant("identity_revoked"))` on reject.
/// - `Err(Error::Invariant("clock_skew_future"))` if `signed_at > now`
///   (no future-dated messages — closes a trivial replay/skew bypass).
pub fn verify_identity_with_grace(
    ledger: &RevocationLedger,
    claimed_key: &[u8; 32],
    signed_at: u64,
    now: u64,
    grace_secs: u64,
) -> Result<()> {
    if signed_at > now {
        return Err(Error::Invariant("clock_skew_future"));
    }
    match ledger.revoked_at(claimed_key) {
        None => Ok(()),
        Some(revoked_at) => {
            // Pre-revocation message: accept.
            if signed_at < revoked_at {
                return Ok(());
            }
            // signed_at >= revoked_at — only the per-key grace window
            // covers this case, and only against the verifier's clock.
            let grace_end = revoked_at.saturating_add(grace_secs);
            if now <= grace_end {
                Ok(())
            } else {
                Err(Error::Invariant("identity_revoked"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_core::OsRng;

    fn fresh_signer() -> SigningKey {
        SigningKey::generate(&mut OsRng)
    }

    #[test]
    fn rev_01_well_formed_cert_self_verifies() {
        // REV-01: a self-signed cert verifies under the revoked key.
        let sk = fresh_signer();
        let cert = RevocationCert::issue_self(&sk, 1_000_000, RevocationReason::Compromise);
        cert.verify_self_signed().unwrap();
    }

    #[test]
    fn rev_02_signature_mismatch_rejected() {
        // REV-02: tampering any byte of the signed body invalidates the cert.
        let sk = fresh_signer();
        let mut cert = RevocationCert::issue_self(&sk, 1_000_000, RevocationReason::Compromise);
        cert.revoked_at = cert.revoked_at.wrapping_add(1); // tamper after signing
        let r = cert.verify_self_signed();
        assert!(matches!(r, Err(Error::Invariant("revocation_invalid_signature"))));
    }

    #[test]
    fn rev_03_post_revocation_message_rejected_outside_grace() {
        // REV-03: a message signed *after* revocation, beyond the grace
        // window, is rejected.
        let sk = fresh_signer();
        let key = sk.verifying_key().to_bytes();
        let cert = RevocationCert::issue_self(&sk, 1_000, RevocationReason::Compromise);
        let mut ledger = RevocationLedger::new();
        ledger.submit(&cert).unwrap();

        // Message signed at 1_500, now = 5_000, grace = 1_000 → grace_end = 2_000.
        // 5_000 > 2_000 → reject.
        let r = verify_identity_with_grace(&ledger, &key, 1_500, 5_000, 1_000);
        assert!(matches!(r, Err(Error::Invariant("identity_revoked"))));
    }

    #[test]
    fn rev_04_pre_revocation_message_accepted() {
        // REV-04: a message signed *before* revocation is always accepted,
        // regardless of how late the verifier sees it.
        let sk = fresh_signer();
        let key = sk.verifying_key().to_bytes();
        let cert = RevocationCert::issue_self(&sk, 5_000, RevocationReason::Rotate);
        let mut ledger = RevocationLedger::new();
        ledger.submit(&cert).unwrap();

        // signed_at = 4_000 < revoked_at = 5_000 → accept even at now = 9_999_999.
        verify_identity_with_grace(&ledger, &key, 4_000, 9_999_999, 0).unwrap();
    }

    #[test]
    fn rev_05_grace_window_edge_accepts_then_rejects() {
        // REV-05: at now == revoked_at + grace_secs the message is still
        // accepted; at now == revoked_at + grace_secs + 1 it is rejected.
        let sk = fresh_signer();
        let key = sk.verifying_key().to_bytes();
        let cert = RevocationCert::issue_self(&sk, 1_000, RevocationReason::Compromise);
        let mut ledger = RevocationLedger::new();
        ledger.submit(&cert).unwrap();

        // signed_at = revoked_at = 1_000, grace = 100 → grace_end = 1_100.
        verify_identity_with_grace(&ledger, &key, 1_000, 1_100, 100).unwrap();
        let r = verify_identity_with_grace(&ledger, &key, 1_000, 1_101, 100);
        assert!(matches!(r, Err(Error::Invariant("identity_revoked"))));
    }

    #[test]
    fn rev_06_replayed_later_cert_rejected_and_no_clock_skew() {
        // REV-06: combination falsifier — submitting a *later* revocation
        // cert for an already-revoked key is rejected (replay-with-newer);
        // and a future-dated signed_at is rejected for clock-skew.
        let sk = fresh_signer();
        let key = sk.verifying_key().to_bytes();
        let early = RevocationCert::issue_self(&sk, 1_000, RevocationReason::Compromise);
        let later = RevocationCert::issue_self(&sk, 2_000, RevocationReason::Compromise);
        let mut ledger = RevocationLedger::new();
        ledger.submit(&early).unwrap();
        let r_replay = ledger.submit(&later);
        assert!(matches!(r_replay, Err(Error::Invariant("revocation_replay_rejected"))));

        // signed_at strictly in the future of `now` → reject regardless of revocation state.
        let r_skew = verify_identity_with_grace(&ledger, &key, 9_999, 100, 0);
        assert!(matches!(r_skew, Err(Error::Invariant("clock_skew_future"))));

        // Wrong key (not on file) under future-dated signed_at also rejected for skew.
        let other = fresh_signer().verifying_key().to_bytes();
        let r_skew2 = verify_identity_with_grace(&ledger, &other, 9_999, 100, 0);
        assert!(matches!(r_skew2, Err(Error::Invariant("clock_skew_future"))));
    }
}
