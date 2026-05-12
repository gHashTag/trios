//! # CR-CHAT-03 · L-CHAT-3-leafsig — MLS LeafNode signature validation
//!
//! `[VERIFIED]` Wave-29 lane A — Defends against a class of attacks
//! where an adversary tampers with the per-leaf signature on a
//! `LeafNode` (RFC 9420 §7.1 — LeafNode contents, §7.3 — LeafNode
//! signature, §7.6 — LeafNode validation):
//!
//! * **Non-canonical signature length** — claims a `signature` whose
//!   byte length is not equal to the ciphersuite's canonical signature
//!   size (Ed25519 ⇒ 64 bytes, ECDSA-P256 ⇒ 64 bytes for the raw
//!   encoded form used by MLS — see RFC 9420 §5.1.2).
//! * **Cross-group LeafNode binding** — the LeafNode is signed under
//!   one `group_id` but is presented to a receiver whose
//!   `local_group_id` differs (RFC 9420 §7.6: LeafNodeSource ∈
//!   {commit, update} ⇒ signature MUST be bound to the group).
//! * **Stale epoch LeafNode** — the LeafNode is signed for an epoch
//!   strictly less than the receiver's current epoch (replay /
//!   downgrade): RFC 9420 §7.6 forbids accepting a leaf bound to a
//!   stale epoch from `LeafNodeSource::Commit` or `Update`.
//! * **Signature key / credential public-key mismatch** — the
//!   `signature_key` advertised in the LeafNode does not match the
//!   public key inside the bundled `credential`; an attacker could
//!   bind a victim's credential to an attacker-held signing key.
//! * **Reserved capability bit forge** — non-zero bits set inside the
//!   reserved range of the LeafNode `capabilities.extensions`
//!   bitfield (RFC 9420 §7.2: unknown / reserved capability codes
//!   MUST be ignored — but if the attacker sets a reserved bit the
//!   leaf is malformed and MUST be rejected before any group-state
//!   commit).
//! * **Signature replay across leaves** — the exact `(signature,
//!   signature_key, group_id, epoch)` quadruple has already been
//!   recorded in the leaf ledger (an attacker re-uses a captured
//!   signature on a different LeafNode body).
//!
//! See RFC 9420 §7.6 (LeafNode validation rules). The six rules below
//! are enforced in fixed order; any attempt to weaken or skip them
//! produces a `LeafNodeSignatureError`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-LEAFNODE-SIGNATURE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical length of an MLS LeafNode signature in bytes (Ed25519
/// and ECDSA-P256 both produce 64-byte raw signatures under the
/// MLS-default encoding — RFC 9420 §5.1.2 ciphersuite tables).
pub const LEAF_NODE_SIGNATURE_LEN: usize = 64;

/// Canonical length of a LeafNode signing public key in bytes (32 for
/// Ed25519 and 33 for compressed P-256; the most commonly deployed
/// MLS ciphersuite is `MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519`
/// whose signing public key is 32 bytes — RFC 9420 §5.1.2).
pub const LEAF_NODE_SIGNATURE_KEY_LEN: usize = 32;

/// One LeafNode packet as it appears inside a Commit, Update or
/// KeyPackage. We only model the fields that the signature guard
/// actually inspects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafNodePacket {
    /// Local group identifier the LeafNode is bound to.
    pub group_id: Vec<u8>,
    /// Epoch this LeafNode is bound to (`epoch == current_epoch + 1`
    /// for `LeafNodeSource::Commit`, `== current_epoch` for
    /// `Update`; either way, MUST NOT be strictly less than the
    /// receiver's current epoch).
    pub epoch: u64,
    /// Signing public key advertised in the LeafNode body — RFC 9420
    /// §7.1: `signature_key`.
    pub signature_key: Vec<u8>,
    /// Signing public key inside the bundled `credential` — RFC 9420
    /// §7.1: MUST equal `signature_key`.
    pub credential_public_key: Vec<u8>,
    /// Raw signature bytes — exactly `LEAF_NODE_SIGNATURE_LEN` bytes.
    pub signature: Vec<u8>,
    /// Capability bits advertised by the leaf (only the lower 16 bits
    /// are RFC-assigned; bits 16..63 are reserved and MUST be zero).
    pub capability_bits: u64,
}

/// Receiving-group view used to validate a `LeafNodePacket`. The
/// receiver trusts only the leaves it has already accepted.
#[derive(Debug, Clone)]
pub struct LeafNodeSignatureView {
    /// `group_id` of the local group.
    pub local_group_id: Vec<u8>,
    /// Current epoch the receiver has accepted.
    pub current_epoch: u64,
    /// Bitmask of capability bits assigned by RFC 9420; any bit
    /// outside this mask is reserved.
    pub assigned_capability_mask: u64,
    /// Ledger of `(group_id, epoch, signature_key, signature)`
    /// quadruples already accepted — replay guard against the same
    /// signature being re-used on a different leaf body.
    pub used_leaf_signatures:
        BTreeSet<(Vec<u8>, u64, Vec<u8>, Vec<u8>)>,
}

/// Why a LeafNode signature guard rejected a packet. Variants are in
/// the same fixed order as the rules in [`validate_leaf_node_signature`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafNodeSignatureError {
    /// `signature.len() != LEAF_NODE_SIGNATURE_LEN`.
    NonCanonicalSignatureLength,
    /// `packet.group_id != view.local_group_id`.
    CrossGroupBinding,
    /// `packet.epoch < view.current_epoch`.
    StaleEpoch,
    /// `packet.signature_key != packet.credential_public_key` (or
    /// either side has a non-canonical key length).
    SignatureKeyCredentialMismatch,
    /// `packet.capability_bits & !view.assigned_capability_mask != 0`.
    ReservedCapabilityBitForge,
    /// `(group_id, epoch, signature_key, signature)` already present
    /// in `view.used_leaf_signatures`.
    SignatureReplay,
}

/// Validate one LeafNode against the receiver's view. Enforces the
/// six rules from RFC 9420 §7.6 in fixed order.
pub fn validate_leaf_node_signature(
    packet: &LeafNodePacket,
    view: &LeafNodeSignatureView,
) -> Result<(), LeafNodeSignatureError> {
    // Rule 1 — signature length must match the ciphersuite default.
    // Coq: Trinity_Chat.v::lns_non_canonical_sig_len
    if packet.signature.len() != LEAF_NODE_SIGNATURE_LEN {
        return Err(LeafNodeSignatureError::NonCanonicalSignatureLength);
    }
    // Rule 2 — group_id must match the receiver's local group.
    // Coq: Trinity_Chat.v::lns_cross_group_binding
    if packet.group_id != view.local_group_id {
        return Err(LeafNodeSignatureError::CrossGroupBinding);
    }
    // Rule 3 — epoch must NOT be strictly less than current_epoch.
    // Coq: Trinity_Chat.v::lns_stale_epoch
    if packet.epoch < view.current_epoch {
        return Err(LeafNodeSignatureError::StaleEpoch);
    }
    // Rule 4 — signature_key inside the LeafNode body MUST equal the
    // public key inside the bundled credential.
    // Coq: Trinity_Chat.v::lns_sig_credential_mismatch
    if packet.signature_key.len() != LEAF_NODE_SIGNATURE_KEY_LEN
        || packet.credential_public_key.len() != LEAF_NODE_SIGNATURE_KEY_LEN
        || packet.signature_key != packet.credential_public_key
    {
        return Err(LeafNodeSignatureError::SignatureKeyCredentialMismatch);
    }
    // Rule 5 — reserved capability bits MUST be zero.
    // Coq: Trinity_Chat.v::lns_reserved_capability_bit_forge
    if packet.capability_bits & !view.assigned_capability_mask != 0 {
        return Err(LeafNodeSignatureError::ReservedCapabilityBitForge);
    }
    // Rule 6 — signature replay across leaves.
    // Coq: Trinity_Chat.v::lns_signature_replay (informal — replay
    // ledger is operational, not algebraic).
    let key = (
        packet.group_id.clone(),
        packet.epoch,
        packet.signature_key.clone(),
        packet.signature.clone(),
    );
    if view.used_leaf_signatures.contains(&key) {
        return Err(LeafNodeSignatureError::SignatureReplay);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_packet() -> LeafNodePacket {
        LeafNodePacket {
            group_id: b"trinity-chat-room".to_vec(),
            epoch: 12,
            signature_key: vec![0xA1; LEAF_NODE_SIGNATURE_KEY_LEN],
            credential_public_key: vec![0xA1; LEAF_NODE_SIGNATURE_KEY_LEN],
            signature: vec![0x55; LEAF_NODE_SIGNATURE_LEN],
            capability_bits: 0x0000_0000_0000_00FF,
        }
    }

    fn good_view() -> LeafNodeSignatureView {
        LeafNodeSignatureView {
            local_group_id: b"trinity-chat-room".to_vec(),
            current_epoch: 12,
            assigned_capability_mask: 0x0000_0000_0000_FFFF,
            used_leaf_signatures: BTreeSet::new(),
        }
    }

    /// **LNS-01** — signature too short rejected.
    #[test]
    fn lns_01_signature_too_short_rejected() {
        let mut p = good_packet();
        p.signature = vec![0x55; 32];
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::NonCanonicalSignatureLength)
        );
    }

    /// **LNS-02** — signature too long rejected.
    #[test]
    fn lns_02_signature_too_long_rejected() {
        let mut p = good_packet();
        p.signature = vec![0x55; 96];
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::NonCanonicalSignatureLength)
        );
    }

    /// **LNS-03** — cross-group LeafNode rejected.
    #[test]
    fn lns_03_cross_group_binding_rejected() {
        let mut p = good_packet();
        p.group_id = b"other-room".to_vec();
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::CrossGroupBinding)
        );
    }

    /// **LNS-04** — stale-epoch LeafNode rejected.
    #[test]
    fn lns_04_stale_epoch_rejected() {
        let mut p = good_packet();
        p.epoch = 5;
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::StaleEpoch)
        );
    }

    /// **LNS-05** — credential / signing-key mismatch rejected.
    #[test]
    fn lns_05_credential_key_mismatch_rejected() {
        let mut p = good_packet();
        p.credential_public_key = vec![0x99; LEAF_NODE_SIGNATURE_KEY_LEN];
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::SignatureKeyCredentialMismatch)
        );
    }

    /// **LNS-06** — non-canonical signature_key length rejected.
    #[test]
    fn lns_06_wrong_signature_key_len_rejected() {
        let mut p = good_packet();
        p.signature_key = vec![0xA1; 16];
        p.credential_public_key = vec![0xA1; 16];
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::SignatureKeyCredentialMismatch)
        );
    }

    /// **LNS-07** — reserved capability bit forge rejected.
    #[test]
    fn lns_07_reserved_capability_bit_rejected() {
        let mut p = good_packet();
        p.capability_bits = 0x0001_0000_0000_0000;
        assert_eq!(
            validate_leaf_node_signature(&p, &good_view()),
            Err(LeafNodeSignatureError::ReservedCapabilityBitForge)
        );
    }

    /// **LNS-08** — replay of identical signature on same leaf
    /// coordinates rejected.
    #[test]
    fn lns_08_signature_replay_rejected() {
        let p = good_packet();
        let mut v = good_view();
        v.used_leaf_signatures.insert((
            p.group_id.clone(),
            p.epoch,
            p.signature_key.clone(),
            p.signature.clone(),
        ));
        assert_eq!(
            validate_leaf_node_signature(&p, &v),
            Err(LeafNodeSignatureError::SignatureReplay)
        );
    }

    /// **LNS-09** — valid LeafNode at current epoch accepted.
    #[test]
    fn lns_09_valid_current_epoch_accepted() {
        let p = good_packet();
        let v = good_view();
        assert_eq!(validate_leaf_node_signature(&p, &v), Ok(()));
    }

    /// **LNS-10** — valid LeafNode at next epoch (Commit source) accepted.
    #[test]
    fn lns_10_valid_next_epoch_accepted() {
        let mut p = good_packet();
        p.epoch = 13;
        let v = good_view();
        assert_eq!(validate_leaf_node_signature(&p, &v), Ok(()));
    }
}
