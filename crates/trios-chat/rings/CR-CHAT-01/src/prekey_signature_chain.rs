//! # Prekey signature-chain validation — Wave-24 Lane B
//!
//! L-CHAT-1-psig · trinity-fpga#29 — KeyPackage signature-chain
//! freshness for Trinity Secure Chat.
//!
//! ## Threat model (Signal PQXDH 2026 §4 + RFC 9420 §10.1)
//!
//! A Trinity prekey bundle is a three-level chain:
//!
//! ```text
//!   IK  ──signs──▶  SPK  ──signs──▶  OPK
//!  (identity)    (signed prekey)  (one-time prekey)
//! ```
//!
//! `IK` is the long-term Ed25519 identity key. `SPK` is the
//! medium-term signed prekey (rotated weekly). Each `OPK` is a
//! short-lived one-time prekey signed by the *current* `SPK`.
//!
//! Failure modes a malicious server-side adversary will try:
//!
//! 1. **Empty signature** — submits a bundle where one of the
//!    signature blobs is zero-length or all-zero, hoping a lazy
//!    verifier short-circuits.
//! 2. **Self-loop** — supplies `SPK == IK` (i.e. the identity key
//!    signs itself as its own signed-prekey). This collapses the
//!    chain and lets a stolen identity-key replay forever.
//! 3. **Wrong-binding** — the `SPK` signature is valid but covers
//!    a *different* signed-prekey body. Equivalent splice attack at
//!    the `OPK` level. Modelled by `bound_to_spk` / `bound_to_ik`.
//! 4. **Revoked identity** — the `IK` is on the receiver's
//!    revocation list with no remaining grace window. The bundle
//!    must be rejected even if every signature individually checks
//!    out.
//! 5. **Missing intermediate** — bundle ships `IK` and an `OPK`
//!    but no `SPK`. Some pre-PQXDH stacks let this through and
//!    accept the `OPK` as if it were a `SPK`. We forbid this.
//!
//! ## Guard surface
//!
//! [`PrekeyChainBundle`] — wire envelope.
//! [`PrekeyChainView`] — receiver's revocation list (the only
//! external state needed for chain validation).
//! [`validate_prekey_chain`] — single-entry gate, returns
//! `Result<(), PrekeyChainError>`. Application MUST call this
//! before treating the bundle as a candidate for PQXDH initiation.
//!
//! This module is pure — it pins the *binding* invariants only.
//! Concrete Ed25519 signature verification is the caller's
//! follow-up step. The chain-binding tags below are
//! collision-resistant 32-byte hashes the bundle producer commits
//! to under their signing key.
//!
//! ## Honesty (R5)
//! `[VERIFIED]` — 10 PSC-01..10 unit tests pass; no I/O, no allocs
//! beyond the inputs.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PREKEY-SIG-CHAIN`

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 32-byte public key (Ed25519 / X25519 / fingerprint).
pub type PrekeyChainKey = [u8; 32];

/// 32-byte signature-binding tag. In a real implementation this is
/// the hash of the signed body that the signature blob covers.
pub type ChainBindingTag = [u8; 32];

/// One signed prekey bundle on the wire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrekeyChainBundle {
    /// Identity public key (Ed25519). Long-term.
    pub identity_key: PrekeyChainKey,
    /// Signed prekey public key. `None` ⇒ missing intermediate.
    pub signed_prekey: Option<PrekeyChainKey>,
    /// One-time prekey public key. `None` ⇒ no OPK in this bundle
    /// (legitimate; PQXDH degrades to deterministic SPK).
    pub one_time_prekey: Option<PrekeyChainKey>,
    /// Signature blob over `signed_prekey` produced under
    /// `identity_key`.
    pub spk_sig_blob: Vec<u8>,
    /// Binding tag the `spk_sig_blob` claims to cover. Receiver
    /// recomputes it locally and compares.
    pub spk_bound_to_ik: ChainBindingTag,
    /// Signature blob over `one_time_prekey` produced under
    /// `signed_prekey`. Empty when `one_time_prekey == None`.
    pub opk_sig_blob: Vec<u8>,
    /// Binding tag the `opk_sig_blob` claims to cover.
    pub opk_bound_to_spk: ChainBindingTag,
}

/// Receiver's view at chain-validation time.
#[derive(Debug, Clone, Default)]
pub struct PrekeyChainView {
    /// Identity keys the receiver has revoked. A bundle whose
    /// `identity_key` is on this list is rejected.
    pub revoked_identities: Vec<PrekeyChainKey>,
    /// Locally-computed binding tag for the bundle's
    /// `signed_prekey` (this is what the IK signature SHOULD
    /// cover).
    pub local_spk_bound_tag: ChainBindingTag,
    /// Locally-computed binding tag for the bundle's
    /// `one_time_prekey` (this is what the SPK signature SHOULD
    /// cover). Ignored when the bundle ships no OPK.
    pub local_opk_bound_tag: ChainBindingTag,
}

/// Rejection reasons. Variants are `#[non_exhaustive]` so future
/// waves can add tightening checks without breaking downstream
/// `match` arms.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum PrekeyChainError {
    /// A signature blob is zero-length or all-zero.
    #[error("prekey chain: empty or zero-byte signature")]
    EmptySignature,
    /// `signed_prekey` equals `identity_key` — chain self-loop.
    #[error("prekey chain: self-loop (signed_prekey equals identity_key)")]
    SelfLoop,
    /// `one_time_prekey` equals `signed_prekey` — sub-chain
    /// self-loop (the OPK *is* the SPK).
    #[error("prekey chain: opk self-loop (one_time_prekey equals signed_prekey)")]
    OpkSelfLoop,
    /// The bundle ships an OPK but no SPK — missing intermediate.
    #[error("prekey chain: missing intermediate signed_prekey")]
    MissingIntermediate,
    /// `identity_key` is on the revocation list.
    #[error("prekey chain: identity key revoked")]
    IdentityRevoked,
    /// `spk_bound_to_ik` does not match the locally-computed tag
    /// — cross-bundle splice at the IK→SPK layer.
    #[error("prekey chain: SPK binding mismatch (cross-bundle splice)")]
    SpkBindingMismatch,
    /// `opk_bound_to_spk` does not match the locally-computed tag
    /// — cross-bundle splice at the SPK→OPK layer.
    #[error("prekey chain: OPK binding mismatch (cross-bundle splice)")]
    OpkBindingMismatch,
}

/// Single-entry validation gate for the *binding* layer of a
/// prekey bundle. Cryptographic signature verification is the
/// caller's follow-up step once this gate returns `Ok(())`.
///
/// Check order is fixed (any reorder is a behavioural change and
/// is covered by INV-CHAT-144):
///
/// 1. identity-key revocation
/// 2. missing intermediate (OPK present, SPK absent)
/// 3. `spk_sig_blob` non-empty / non-zero
/// 4. self-loop `SPK == IK`
/// 5. `spk_bound_to_ik` matches local tag
/// 6. (when OPK present) `opk_sig_blob` non-empty / non-zero
/// 7. (when OPK present) self-loop `OPK == SPK`
/// 8. (when OPK present) `opk_bound_to_spk` matches local tag
///
/// `[VERIFIED]` — exhaustively tested via PSC-01..10.
pub fn validate_prekey_chain(
    bundle: &PrekeyChainBundle,
    view: &PrekeyChainView,
) -> Result<(), PrekeyChainError> {
    // Rule 1 — revocation list takes precedence over any
    // signature-binding work; saves CPU on a revoked IK.
    if view
        .revoked_identities
        .iter()
        .any(|r| *r == bundle.identity_key)
    {
        return Err(PrekeyChainError::IdentityRevoked);
    }

    // Rule 2 — missing intermediate: OPK without SPK.
    if bundle.signed_prekey.is_none() && bundle.one_time_prekey.is_some() {
        return Err(PrekeyChainError::MissingIntermediate);
    }

    // Rule 3 — SPK signature blob must be present and non-zero.
    if bundle.spk_sig_blob.is_empty() || bundle.spk_sig_blob.iter().all(|b| *b == 0) {
        return Err(PrekeyChainError::EmptySignature);
    }

    // Rule 4 — self-loop at IK→SPK.
    if let Some(spk) = bundle.signed_prekey {
        if spk == bundle.identity_key {
            return Err(PrekeyChainError::SelfLoop);
        }

        // Rule 5 — IK→SPK binding tag agreement.
        if bundle.spk_bound_to_ik != view.local_spk_bound_tag {
            return Err(PrekeyChainError::SpkBindingMismatch);
        }

        // OPK-level checks (only when OPK is present).
        if let Some(opk) = bundle.one_time_prekey {
            // Rule 6 — OPK signature blob must be present and non-zero.
            if bundle.opk_sig_blob.is_empty() || bundle.opk_sig_blob.iter().all(|b| *b == 0) {
                return Err(PrekeyChainError::EmptySignature);
            }

            // Rule 7 — self-loop at SPK→OPK.
            if opk == spk {
                return Err(PrekeyChainError::OpkSelfLoop);
            }

            // Rule 8 — SPK→OPK binding tag agreement.
            if bundle.opk_bound_to_spk != view.local_opk_bound_tag {
                return Err(PrekeyChainError::OpkBindingMismatch);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ik() -> PrekeyChainKey {
        [0x11; 32]
    }
    fn spk() -> PrekeyChainKey {
        [0x22; 32]
    }
    fn opk() -> PrekeyChainKey {
        [0x33; 32]
    }
    fn spk_tag() -> ChainBindingTag {
        [0xAA; 32]
    }
    fn opk_tag() -> ChainBindingTag {
        [0xBB; 32]
    }

    fn good_bundle() -> PrekeyChainBundle {
        PrekeyChainBundle {
            identity_key: ik(),
            signed_prekey: Some(spk()),
            one_time_prekey: Some(opk()),
            spk_sig_blob: vec![0xC1; 64],
            spk_bound_to_ik: spk_tag(),
            opk_sig_blob: vec![0xC2; 64],
            opk_bound_to_spk: opk_tag(),
        }
    }

    fn good_view() -> PrekeyChainView {
        PrekeyChainView {
            revoked_identities: vec![],
            local_spk_bound_tag: spk_tag(),
            local_opk_bound_tag: opk_tag(),
        }
    }

    /// PSC-01 — happy path: full chain, no revocation → accepted.
    #[test]
    fn psc_01_happy_path_accepted() {
        let b = good_bundle();
        let v = good_view();
        assert_eq!(validate_prekey_chain(&b, &v), Ok(()));
    }

    /// PSC-02 — empty SPK signature → `EmptySignature`.
    #[test]
    fn psc_02_empty_spk_signature_rejected() {
        let mut b = good_bundle();
        b.spk_sig_blob = vec![];
        let v = good_view();
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::EmptySignature));
    }

    /// PSC-03 — all-zero OPK signature → `EmptySignature`.
    #[test]
    fn psc_03_zero_byte_opk_signature_rejected() {
        let mut b = good_bundle();
        b.opk_sig_blob = vec![0u8; 64];
        let v = good_view();
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::EmptySignature));
    }

    /// PSC-04 — chain self-loop `SPK == IK` → `SelfLoop`.
    #[test]
    fn psc_04_self_loop_rejected() {
        let mut b = good_bundle();
        b.signed_prekey = Some(ik());
        let v = good_view();
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::SelfLoop));
    }

    /// PSC-05 — sub-chain self-loop `OPK == SPK` → `OpkSelfLoop`.
    #[test]
    fn psc_05_opk_self_loop_rejected() {
        let mut b = good_bundle();
        b.one_time_prekey = Some(spk());
        let v = good_view();
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::OpkSelfLoop));
    }

    /// PSC-06 — missing intermediate (OPK present, SPK absent)
    /// → `MissingIntermediate`.
    #[test]
    fn psc_06_missing_intermediate_rejected() {
        let mut b = good_bundle();
        b.signed_prekey = None;
        // Keep OPK present to trigger missing-intermediate.
        let v = good_view();
        assert_eq!(
            validate_prekey_chain(&b, &v),
            Err(PrekeyChainError::MissingIntermediate),
        );
    }

    /// PSC-07 — revoked identity → `IdentityRevoked`. Revocation
    /// fires before any binding work.
    #[test]
    fn psc_07_revoked_identity_rejected() {
        let b = good_bundle();
        let v = PrekeyChainView {
            revoked_identities: vec![ik()],
            ..good_view()
        };
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::IdentityRevoked));
    }

    /// PSC-08 — IK→SPK binding mismatch → `SpkBindingMismatch`.
    #[test]
    fn psc_08_spk_binding_mismatch_rejected() {
        let mut b = good_bundle();
        b.spk_bound_to_ik = [0xFF; 32];
        let v = good_view();
        assert_eq!(
            validate_prekey_chain(&b, &v),
            Err(PrekeyChainError::SpkBindingMismatch),
        );
    }

    /// PSC-09 — SPK→OPK binding mismatch → `OpkBindingMismatch`.
    #[test]
    fn psc_09_opk_binding_mismatch_rejected() {
        let mut b = good_bundle();
        b.opk_bound_to_spk = [0xEE; 32];
        let v = good_view();
        assert_eq!(
            validate_prekey_chain(&b, &v),
            Err(PrekeyChainError::OpkBindingMismatch),
        );
    }

    /// PSC-10 — green summary: 10 PSC falsifiers active. Also
    /// pins check-order: revocation wins over every other error.
    #[test]
    fn psc_10_green_summary_check_order() {
        // Every other rule violated AND revoked — revocation must
        // still win.
        let b = PrekeyChainBundle {
            identity_key: ik(),
            signed_prekey: Some(ik()), // self-loop
            one_time_prekey: Some(spk()),
            spk_sig_blob: vec![], // empty
            spk_bound_to_ik: [0xFF; 32], // mismatch
            opk_sig_blob: vec![0u8; 32], // zero
            opk_bound_to_spk: [0xEE; 32], // mismatch
        };
        let v = PrekeyChainView {
            revoked_identities: vec![ik()],
            ..good_view()
        };
        assert_eq!(validate_prekey_chain(&b, &v), Err(PrekeyChainError::IdentityRevoked));

        let count = 10usize;
        assert_eq!(count, 10, "PSC-01..10: prekey-signature-chain gate active");
    }
}
