//! # CR-CHAT-03 · L-CHAT-3-psk — MLS PSK external/resumption injection defense
//!
//! `[VERIFIED]` Wave-26 lane A — Defends against a class of attacks
//! where an adversary smuggles a Pre-Shared Key reference into an MLS
//! commit / Welcome that the receiving group MUST reject:
//!
//! * **External PSK forge** — claims `PSKType::External` with a
//!   `psk_id` the receiver never provisioned.
//! * **Resumption PSK splice** — claims `PSKType::Resumption` with a
//!   `psk_group_id` that does not match the local group (cross-group
//!   resumption-chain splice).
//! * **Resumption epoch rollback** — claims a resumption PSK from an
//!   epoch strictly greater than the local current epoch (forward-
//!   referenced resumption is a state-rollback primitive).
//! * **PSK nonce reuse** — claims a `psk_nonce` already consumed for
//!   the same `(psk_id, psk_group_id, psk_epoch)` triple.
//! * **PSK nonce off-canonical length** — `psk_nonce` not exactly
//!   `PSK_NONCE_LEN` bytes.
//!
//! See RFC 9420 §5.3 (Pre-Shared Keys) and §12.4.4 (PreSharedKey
//! Proposal). The five rules below are enforced in fixed order; any
//! attempt to weaken or skip them produces a `PskInjectionError`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-PSK-INJECTION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical length of a PSK nonce, in bytes (RFC 9420 §5.3 — KDF
/// hash output size for default ciphersuite is 32 bytes).
pub const PSK_NONCE_LEN: usize = 32;

/// PSK kind — RFC 9420 §5.3 enumerates two kinds of PSK references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PskType {
    /// Out-of-band provisioned PSK.
    External,
    /// Resumption PSK chained from an earlier epoch of the same (or
    /// related) MLS group.
    Resumption,
}

/// Reference to a single PSK as it would appear in a `PreSharedKey`
/// proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PskRef {
    /// `PSKType::External` or `PSKType::Resumption`.
    pub psk_type: PskType,
    /// Opaque identifier for the PSK (out-of-band agreed for
    /// `External`, MLS `group_id` for `Resumption`).
    pub psk_id: Vec<u8>,
    /// For `Resumption`: the MLS `group_id` the PSK is chained from.
    /// For `External`: empty.
    pub psk_group_id: Vec<u8>,
    /// For `Resumption`: the epoch the PSK is chained from. For
    /// `External`: zero.
    pub psk_epoch: u64,
    /// Per-use nonce, must be exactly `PSK_NONCE_LEN` bytes.
    pub psk_nonce: Vec<u8>,
}

/// Per-group view used to validate a `PskRef`. The receiving group
/// trusts only the PSKs it explicitly provisioned (External) or that
/// chain from a previously-accepted epoch of itself (Resumption).
#[derive(Debug, Clone)]
pub struct PskInjectionView {
    /// PSK ids the group provisioned as `External`.
    pub provisioned_external_ids: BTreeSet<Vec<u8>>,
    /// `group_id` of the local group (32 bytes by RFC 9420 default).
    pub local_group_id: Vec<u8>,
    /// Current epoch of the local group.
    pub current_epoch: u64,
    /// Set of `(psk_id, psk_group_id, psk_epoch, psk_nonce)` quads
    /// already consumed — replay ledger.
    pub used_nonces: BTreeSet<(Vec<u8>, Vec<u8>, u64, Vec<u8>)>,
}

/// All ways a `PskRef` can be rejected. Adding variants stays
/// non-breaking via `#[non_exhaustive]`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PskInjectionError {
    /// `psk_nonce.len() != PSK_NONCE_LEN`.
    NonCanonicalNonceLength,
    /// `PskType::External` with `psk_id` not in
    /// `provisioned_external_ids`.
    UnprovisionedExternalId,
    /// `PskType::Resumption` with `psk_group_id != local_group_id`.
    ResumptionGroupSplice,
    /// `PskType::Resumption` with `psk_epoch >= current_epoch`
    /// (a resumption PSK can only chain from a strictly earlier
    /// accepted epoch).
    ResumptionEpochRollback,
    /// `(psk_id, psk_group_id, psk_epoch, psk_nonce)` already in
    /// `used_nonces` — replay.
    NonceReplay,
}

/// `[VERIFIED]` Validate a `PskRef` against the receiving group's
/// view. Returns `Ok(())` iff the PSK is acceptable.
///
/// The five rules are evaluated in fixed order so the error variant
/// is deterministic for a given input.
pub fn validate_psk_ref(
    psk: &PskRef,
    view: &PskInjectionView,
) -> Result<(), PskInjectionError> {
    // Rule 1 — nonce length must match canonical KDF output size.
    if psk.psk_nonce.len() != PSK_NONCE_LEN {
        return Err(PskInjectionError::NonCanonicalNonceLength);
    }

    // Rule 2 — External PSKs must have been out-of-band provisioned.
    if psk.psk_type == PskType::External {
        if !view.provisioned_external_ids.contains(&psk.psk_id) {
            return Err(PskInjectionError::UnprovisionedExternalId);
        }
    }

    // Rule 3 — Resumption PSKs must chain from the local group.
    if psk.psk_type == PskType::Resumption {
        if psk.psk_group_id != view.local_group_id {
            return Err(PskInjectionError::ResumptionGroupSplice);
        }

        // Rule 4 — Resumption PSKs must chain from a strictly earlier
        // accepted epoch (forward references are rollback primitives).
        if psk.psk_epoch >= view.current_epoch {
            return Err(PskInjectionError::ResumptionEpochRollback);
        }
    }

    // Rule 5 — replay check on the (id, gid, epoch, nonce) quad.
    let key = (
        psk.psk_id.clone(),
        psk.psk_group_id.clone(),
        psk.psk_epoch,
        psk.psk_nonce.clone(),
    );
    if view.used_nonces.contains(&key) {
        return Err(PskInjectionError::NonceReplay);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCAL_GID: &[u8; 32] = &[0xAA; 32];

    fn nonce(byte: u8) -> Vec<u8> {
        vec![byte; PSK_NONCE_LEN]
    }

    fn view_with(provisioned: &[&[u8]], current_epoch: u64) -> PskInjectionView {
        PskInjectionView {
            provisioned_external_ids: provisioned
                .iter()
                .map(|s| s.to_vec())
                .collect(),
            local_group_id: LOCAL_GID.to_vec(),
            current_epoch,
            used_nonces: BTreeSet::new(),
        }
    }

    // PSK-01 — valid external PSK with provisioned id accepted.
    #[test]
    fn psk_01_valid_external_accepted() {
        let view = view_with(&[b"team-alpha".as_slice()], 5);
        let psk = PskRef {
            psk_type: PskType::External,
            psk_id: b"team-alpha".to_vec(),
            psk_group_id: vec![],
            psk_epoch: 0,
            psk_nonce: nonce(0x11),
        };
        assert_eq!(validate_psk_ref(&psk, &view), Ok(()));
    }

    // PSK-02 — valid resumption PSK chaining from earlier epoch.
    #[test]
    fn psk_02_valid_resumption_accepted() {
        let view = view_with(&[], 10);
        let psk = PskRef {
            psk_type: PskType::Resumption,
            psk_id: b"resume-7".to_vec(),
            psk_group_id: LOCAL_GID.to_vec(),
            psk_epoch: 7,
            psk_nonce: nonce(0x22),
        };
        assert_eq!(validate_psk_ref(&psk, &view), Ok(()));
    }

    // PSK-03 — non-canonical nonce length rejected.
    #[test]
    fn psk_03_short_nonce_rejected() {
        let view = view_with(&[b"team".as_slice()], 5);
        let psk = PskRef {
            psk_type: PskType::External,
            psk_id: b"team".to_vec(),
            psk_group_id: vec![],
            psk_epoch: 0,
            psk_nonce: vec![0x33; PSK_NONCE_LEN - 1],
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::NonCanonicalNonceLength)
        );
    }

    // PSK-04 — empty nonce rejected before any other check.
    #[test]
    fn psk_04_empty_nonce_rejected() {
        let view = view_with(&[b"team".as_slice()], 5);
        let psk = PskRef {
            psk_type: PskType::External,
            psk_id: b"team".to_vec(),
            psk_group_id: vec![],
            psk_epoch: 0,
            psk_nonce: vec![],
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::NonCanonicalNonceLength)
        );
    }

    // PSK-05 — unprovisioned external id rejected.
    #[test]
    fn psk_05_unprovisioned_external_rejected() {
        let view = view_with(&[b"team-alpha".as_slice()], 5);
        let psk = PskRef {
            psk_type: PskType::External,
            psk_id: b"team-evil".to_vec(),
            psk_group_id: vec![],
            psk_epoch: 0,
            psk_nonce: nonce(0x44),
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::UnprovisionedExternalId)
        );
    }

    // PSK-06 — resumption group splice rejected.
    #[test]
    fn psk_06_resumption_group_splice_rejected() {
        let view = view_with(&[], 10);
        let psk = PskRef {
            psk_type: PskType::Resumption,
            psk_id: b"resume-7".to_vec(),
            psk_group_id: vec![0xBB; 32], // other group
            psk_epoch: 7,
            psk_nonce: nonce(0x55),
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::ResumptionGroupSplice)
        );
    }

    // PSK-07 — resumption epoch rollback (forward reference) rejected.
    #[test]
    fn psk_07_resumption_forward_epoch_rejected() {
        let view = view_with(&[], 10);
        let psk = PskRef {
            psk_type: PskType::Resumption,
            psk_id: b"resume-future".to_vec(),
            psk_group_id: LOCAL_GID.to_vec(),
            psk_epoch: 10, // == current, not strictly earlier
            psk_nonce: nonce(0x66),
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::ResumptionEpochRollback)
        );
    }

    // PSK-08 — resumption with epoch greater than current rejected.
    #[test]
    fn psk_08_resumption_epoch_overflow_rejected() {
        let view = view_with(&[], 10);
        let psk = PskRef {
            psk_type: PskType::Resumption,
            psk_id: b"resume-future".to_vec(),
            psk_group_id: LOCAL_GID.to_vec(),
            psk_epoch: 11,
            psk_nonce: nonce(0x66),
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::ResumptionEpochRollback)
        );
    }

    // PSK-09 — replayed nonce rejected.
    #[test]
    fn psk_09_replayed_nonce_rejected() {
        let mut view = view_with(&[b"team".as_slice()], 5);
        let n = nonce(0x77);
        view.used_nonces.insert((
            b"team".to_vec(),
            vec![],
            0,
            n.clone(),
        ));
        let psk = PskRef {
            psk_type: PskType::External,
            psk_id: b"team".to_vec(),
            psk_group_id: vec![],
            psk_epoch: 0,
            psk_nonce: n,
        };
        assert_eq!(
            validate_psk_ref(&psk, &view),
            Err(PskInjectionError::NonceReplay)
        );
    }

    // PSK-10 — same nonce on different epoch is fresh (not replay).
    #[test]
    fn psk_10_same_nonce_different_epoch_accepted() {
        let mut view = view_with(&[], 10);
        let n = nonce(0x88);
        view.used_nonces.insert((
            b"resume".to_vec(),
            LOCAL_GID.to_vec(),
            7,
            n.clone(),
        ));
        let psk = PskRef {
            psk_type: PskType::Resumption,
            psk_id: b"resume".to_vec(),
            psk_group_id: LOCAL_GID.to_vec(),
            psk_epoch: 8, // different epoch
            psk_nonce: n,
        };
        assert_eq!(validate_psk_ref(&psk, &view), Ok(()));
    }
}
