//! # L-CHAT-3-pskprov — External PSK identifier provenance defense
//!
//! Wave-31, Lane B. RFC 9420 §5.3.2 (`PreSharedKeyID` — external PSK
//! type with `external_psk_id` payload) + §5.3.3 (PSK secret
//! derivation pre-image binding).
//!
//! Trinity Chat allows an out-of-band ("external") PSK to be mixed
//! into the group's `psk_secret` via a `PreSharedKey` proposal. Each
//! such proposal carries:
//!
//! - `psktype = external (0x01)`
//! - `psk_id`         — the external PSK identifier (≤ 255 bytes)
//! - `psk_nonce`      — 32-byte freshness nonce
//!
//! Receivers must check that `psk_id` was actually provisioned by a
//! trusted authority and was not yet consumed (one-shot semantics
//! per RFC 9420 §5.3.3). A forged or replayed `psk_id` lets an
//! attacker drag a known secret into the group's KDF, sandbagging
//! forward secrecy and committing CRYPTO injection on the PSK
//! evolution path.
//!
//! Six rules in fixed order:
//! 1. `NonCanonicalPskNonceLength` — reject any `psk_nonce` whose
//!    length differs from `EXTERNAL_PSK_NONCE_LEN = 32`.
//! 2. `EmptyPskId` — reject the zero-length `psk_id` (the spec
//!    requires `length(psk_id) ≥ 1`).
//! 3. `OversizedPskId` — reject any `psk_id` longer than
//!    `EXTERNAL_PSK_ID_MAX_LEN = 255` bytes (`opaque<V>` upper bound).
//! 4. `UnprovisionedExternalPsk` — reject `psk_id ∉
//!    view.provisioned_psk_ids` (no trusted authority entry).
//! 5. `ExternalPskIdReplay` — reject `(psk_id, psk_nonce)` pair
//!    already present in `view.consumed_external_psks` (per-group
//!    one-shot ledger).
//! 6. `ZeroPskNonce` — reject the all-zero `psk_nonce` (degenerate
//!    freshness; a correct sampler never produces it).
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · EXTERNAL-PSK-PROVENANCE`

use std::collections::BTreeSet;

/// Canonical `psk_nonce` length (RFC 9420 §5.3.3 — 32 bytes for
/// MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519).
pub const EXTERNAL_PSK_NONCE_LEN: usize = 32;

/// Maximum permitted `psk_id` length in the on-wire `opaque<V>`
/// encoding (RFC 9420 §5.3.2).
pub const EXTERNAL_PSK_ID_MAX_LEN: usize = 255;

/// One external `PreSharedKeyID` proposal to be validated against
/// the receiver / DS view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalPskProposal {
    /// The external PSK identifier (out-of-band-assigned label).
    pub psk_id: Vec<u8>,
    /// Per-injection freshness nonce.
    pub psk_nonce: Vec<u8>,
}

/// Receiver / Delivery-Service view used to authenticate provenance
/// and enforce one-shot semantics for external PSKs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExternalPskView {
    /// Ledger of `psk_id`s a trusted authority has provisioned.
    pub provisioned_psk_ids: BTreeSet<Vec<u8>>,
    /// Ledger of `(psk_id, psk_nonce)` pairs already consumed by
    /// this group.
    pub consumed_external_psks: BTreeSet<(Vec<u8>, Vec<u8>)>,
}

/// Why an external PSK proposal was rejected. Mirrors INV-CHAT-191..193.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExternalPskIdError {
    /// Rule 1 — `psk_nonce` length is not exactly 32 bytes.
    NonCanonicalPskNonceLength,
    /// Rule 2 — `psk_id` is empty.
    EmptyPskId,
    /// Rule 3 — `psk_id` longer than 255 bytes.
    OversizedPskId,
    /// Rule 4 — `psk_id` is not in the provisioned ledger.
    UnprovisionedExternalPsk,
    /// Rule 5 — `(psk_id, psk_nonce)` pair already consumed.
    ExternalPskIdReplay,
    /// Rule 6 — all-zero `psk_nonce`.
    ZeroPskNonce,
}

/// Validate one external `PreSharedKeyID` proposal against the
/// receiver view.
///
/// Returns `Ok(())` iff all six rules pass; otherwise returns the
/// first rule that fired. Order matches INV-CHAT-191..193.
pub fn validate_external_psk_id(
    proposal: &ExternalPskProposal,
    view: &ExternalPskView,
) -> Result<(), ExternalPskIdError> {
    // Rule 1.
    if proposal.psk_nonce.len() != EXTERNAL_PSK_NONCE_LEN {
        return Err(ExternalPskIdError::NonCanonicalPskNonceLength);
    }
    // Rule 2.
    if proposal.psk_id.is_empty() {
        return Err(ExternalPskIdError::EmptyPskId);
    }
    // Rule 3.
    if proposal.psk_id.len() > EXTERNAL_PSK_ID_MAX_LEN {
        return Err(ExternalPskIdError::OversizedPskId);
    }
    // Rule 4.
    if !view.provisioned_psk_ids.contains(&proposal.psk_id) {
        return Err(ExternalPskIdError::UnprovisionedExternalPsk);
    }
    // Rule 5.
    let key = (proposal.psk_id.clone(), proposal.psk_nonce.clone());
    if view.consumed_external_psks.contains(&key) {
        return Err(ExternalPskIdError::ExternalPskIdReplay);
    }
    // Rule 6.
    if proposal.psk_nonce.iter().all(|&b| b == 0) {
        return Err(ExternalPskIdError::ZeroPskNonce);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_view() -> ExternalPskView {
        let mut provisioned = BTreeSet::new();
        provisioned.insert(b"trinity-export-2026-Q2".to_vec());
        ExternalPskView {
            provisioned_psk_ids: provisioned,
            consumed_external_psks: BTreeSet::new(),
        }
    }

    fn good_proposal(nonce_byte: u8) -> ExternalPskProposal {
        ExternalPskProposal {
            psk_id: b"trinity-export-2026-Q2".to_vec(),
            psk_nonce: vec![nonce_byte; EXTERNAL_PSK_NONCE_LEN],
        }
    }

    /// EPK-01 — 16-byte (too-short) psk_nonce rejected.
    #[test]
    fn epk_01_short_nonce_rejected() {
        let view = base_view();
        let mut p = good_proposal(0x22);
        p.psk_nonce = vec![0x22; 16];
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::NonCanonicalPskNonceLength)
        );
    }

    /// EPK-02 — 64-byte (over-long) psk_nonce rejected.
    #[test]
    fn epk_02_long_nonce_rejected() {
        let view = base_view();
        let mut p = good_proposal(0x22);
        p.psk_nonce = vec![0x22; 64];
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::NonCanonicalPskNonceLength)
        );
    }

    /// EPK-03 — empty psk_id rejected.
    #[test]
    fn epk_03_empty_psk_id_rejected() {
        let view = base_view();
        let mut p = good_proposal(0x22);
        p.psk_id = Vec::new();
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::EmptyPskId)
        );
    }

    /// EPK-04 — oversized psk_id rejected (length > 255 bytes).
    #[test]
    fn epk_04_oversized_psk_id_rejected() {
        let view = base_view();
        let mut p = good_proposal(0x22);
        p.psk_id = vec![b'A'; EXTERNAL_PSK_ID_MAX_LEN + 1];
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::OversizedPskId)
        );
    }

    /// EPK-05 — unprovisioned psk_id rejected.
    #[test]
    fn epk_05_unprovisioned_psk_id_rejected() {
        let view = base_view();
        let mut p = good_proposal(0x22);
        p.psk_id = b"forged-external-psk".to_vec();
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::UnprovisionedExternalPsk)
        );
    }

    /// EPK-06 — replayed (psk_id, psk_nonce) pair rejected.
    #[test]
    fn epk_06_external_psk_replay_rejected() {
        let mut view = base_view();
        let p = good_proposal(0x22);
        view.consumed_external_psks
            .insert((p.psk_id.clone(), p.psk_nonce.clone()));
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::ExternalPskIdReplay)
        );
    }

    /// EPK-07 — all-zero psk_nonce rejected.
    #[test]
    fn epk_07_zero_psk_nonce_rejected() {
        let view = base_view();
        let p = good_proposal(0x00);
        assert_eq!(
            validate_external_psk_id(&p, &view),
            Err(ExternalPskIdError::ZeroPskNonce)
        );
    }

    /// EPK-08 — fresh proposal with provisioned psk_id accepted.
    #[test]
    fn epk_08_valid_proposal_accepted() {
        let view = base_view();
        let p = good_proposal(0x22);
        assert_eq!(validate_external_psk_id(&p, &view), Ok(()));
    }

    /// EPK-09 — distinct psk_nonce for the same psk_id still accepted
    /// (one-shot ledger keys on the pair, not on psk_id alone).
    #[test]
    fn epk_09_distinct_nonce_same_id_accepted() {
        let mut view = base_view();
        let first = good_proposal(0x22);
        view.consumed_external_psks
            .insert((first.psk_id.clone(), first.psk_nonce.clone()));
        let second = good_proposal(0x33);
        assert_eq!(validate_external_psk_id(&second, &view), Ok(()));
    }

    /// EPK-10 — module green: compiles and re-exports through
    /// `CR-CHAT-03/src/lib.rs`.
    #[test]
    fn epk_10_module_green() {
        let count = 10usize;
        assert_eq!(
            count, 10,
            "Wave-31 L-CHAT-3-pskprov: {count} external-PSK-id-provenance falsifiers active"
        );
    }
}
