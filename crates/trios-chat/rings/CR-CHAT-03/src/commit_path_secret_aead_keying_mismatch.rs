//! Wave-36 / L-CHAT-3-cpakm (R-CHAT-3 / CR-CHAT-03) — Commit path-secret
//! AEAD keying mismatch defence per RFC 9420 §7.7 "Updating Tree State"
//! and §8 "Encrypting and Decrypting to/from Tree Nodes" (UPDATE-PATH
//! HPKE encryptions).
//!
//! When a Commit ships an UPDATE-PATH, every non-blank parent node on
//! the sender's direct path carries one HPKE ciphertext per node in
//! its **resolution** (the set of leaves that should receive the new
//! path secret). Each ciphertext is bound to that specific recipient
//! via:
//!   * the recipient leaf's HPKE `init_key`,
//!   * an AEAD nonce derived from the sender's leaf index and the
//!     node level (RFC 9420 §8.2),
//!   * an AAD that pins `(group_id, epoch, sender_leaf, node_index)`.
//!
//! A faulty/malicious sender can produce an UPDATE-PATH whose
//! per-resolution ciphertexts are mis-keyed: the same path secret
//! sealed under the **wrong** leaf's init_key, or with an AAD that
//! lies about which node is being updated. RFC 9420 §12.4.3.2 calls
//! these out as a hard reject: the receiver MUST verify that for
//! every node `n` on its own direct path, the ciphertext slot it
//! decrypts was sealed to *its* leaf (its index in the resolution
//! of `n`) and that the AAD matches the local
//! `(group_id, epoch, sender_leaf, n)` tuple. A mismatch on any
//! single slot poisons the whole epoch.
//!
//! This lane is the consumption-side guard at the receiver. A single
//! deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalGroupIdLength — `commit.group_id.len()` must
//!      equal `CPAKM_GROUP_ID_LEN` (32 bytes — MLS GroupID).
//!   2. EpochMismatch — `commit.epoch != view.local_epoch + 1`
//!      (Commits advance the epoch by exactly 1 per RFC 9420 §7.5).
//!   3. SenderLeafOutOfRange — `commit.sender_leaf >= view.tree_size`
//!      (no phantom senders).
//!   4. ResolutionSlotOutOfRange — for the receiver's own slot index
//!      `view.local_resolution_index`, the slot must exist in
//!      `commit.update_path_slots`.
//!   5. RecipientInitKeyMismatch — the slot's `recipient_init_key`
//!      MUST equal `view.local_init_key` (the receiver's own HPKE
//!      init key). Anything else means the slot was sealed to the
//!      wrong leaf.
//!   6. AadContextMismatch — the slot's `aad_context` MUST equal the
//!      canonical encoding of
//!      `(group_id ‖ epoch ‖ sender_leaf ‖ node_index)`. AAD drift
//!      is the loudest signal.
//!   7. NonCanonicalCiphertextLength — `slot.ciphertext.len()` must
//!      equal `CPAKM_PATH_SECRET_CIPHERTEXT_LEN` (48 bytes — 32-byte
//!      sealed path secret + 16-byte Poly1305 tag per R-CHAT-3).
//!      Wrong-length ciphertext is rejected even before decap.

#![forbid(unsafe_code)]

/// Canonical MLS GroupID length (32 bytes).
pub const CPAKM_GROUP_ID_LEN: usize = 32;

/// Canonical HPKE init_key length (32 bytes — X25519 / ML-KEM-768 wrap
/// per R-CHAT-3).
pub const CPAKM_INIT_KEY_LEN: usize = 32;

/// Canonical AAD-context length (32 + 8 + 8 + 8 = 56 bytes —
/// `group_id ‖ epoch ‖ sender_leaf ‖ node_index`).
pub const CPAKM_AAD_CONTEXT_LEN: usize = CPAKM_GROUP_ID_LEN + 8 + 8 + 8;

/// Canonical path-secret ciphertext length (32 + 16 = 48 bytes — sealed
/// path secret + Poly1305 tag).
pub const CPAKM_PATH_SECRET_CIPHERTEXT_LEN: usize = 48;

/// One HPKE encryption inside a Commit's UPDATE-PATH, addressed to a
/// single leaf in a node's resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePathSlot {
    /// Index of the node on the sender's direct path this slot
    /// belongs to.
    pub node_index: u64,
    /// HPKE init_key of the recipient leaf (32 bytes).
    pub recipient_init_key: Vec<u8>,
    /// AAD context bound by the sender at seal time (56 bytes).
    pub aad_context: Vec<u8>,
    /// HPKE-sealed path secret + AEAD tag (48 bytes).
    pub ciphertext: Vec<u8>,
}

/// A Commit message's UPDATE-PATH header (the receiver-visible
/// metadata only — full body decryption is downstream).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitUpdatePath {
    /// MLS GroupID (32 bytes).
    pub group_id: Vec<u8>,
    /// New epoch the Commit advances to.
    pub epoch: u64,
    /// Sender's leaf index in the ratchet tree.
    pub sender_leaf: u64,
    /// Per-resolution HPKE slots. Indexed by the receiver via
    /// `local_resolution_index`.
    pub update_path_slots: Vec<UpdatePathSlot>,
}

/// Receiver-side view of the local MLS state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatePathView {
    /// Local epoch (the Commit must move this to `local_epoch + 1`).
    pub local_epoch: u64,
    /// Ratchet tree size (leaves).
    pub tree_size: u64,
    /// Receiver's own HPKE init_key (32 bytes).
    pub local_init_key: Vec<u8>,
    /// Receiver's index into the resolution-ordered
    /// `update_path_slots` vector.
    pub local_resolution_index: usize,
    /// The node index the receiver's slot belongs to (the deepest
    /// ancestor on its direct path that the sender is updating).
    pub local_node_index: u64,
}

/// Typed errors for `validate_commit_path_secret`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSecretAeadKeyingError {
    /// Rule 1 — non-canonical group_id length.
    NonCanonicalGroupIdLength,
    /// Rule 2 — epoch does not advance by exactly 1.
    EpochMismatch,
    /// Rule 3 — sender_leaf >= tree_size.
    SenderLeafOutOfRange,
    /// Rule 4 — local_resolution_index out of range in
    /// `update_path_slots`.
    ResolutionSlotOutOfRange,
    /// Rule 5 — slot's recipient_init_key != local_init_key.
    RecipientInitKeyMismatch,
    /// Rule 6 — slot's aad_context disagrees with the canonical
    /// `(group_id ‖ epoch ‖ sender_leaf ‖ node_index)` encoding.
    AadContextMismatch,
    /// Rule 7 — slot's ciphertext length is not canonical.
    NonCanonicalCiphertextLength,
}

/// Build the canonical AAD-context encoding
/// `group_id ‖ epoch_u64_be ‖ sender_leaf_u64_be ‖ node_index_u64_be`.
fn canonical_aad_context(
    group_id: &[u8],
    epoch: u64,
    sender_leaf: u64,
    node_index: u64,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(CPAKM_AAD_CONTEXT_LEN);
    out.extend_from_slice(group_id);
    out.extend_from_slice(&epoch.to_be_bytes());
    out.extend_from_slice(&sender_leaf.to_be_bytes());
    out.extend_from_slice(&node_index.to_be_bytes());
    out
}

/// Constructive guard for a single Commit's UPDATE-PATH slot
/// targeted at the local receiver. Returns `Ok(())` iff every rule
/// (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `CPAKM-01..10` below and
/// the Coq theorems `INV-CHAT-228..232` in the W36 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_commit_path_secret(
    commit: &CommitUpdatePath,
    view: &UpdatePathView,
) -> Result<(), PathSecretAeadKeyingError> {
    // Rule 1: GroupID canonical length.
    if commit.group_id.len() != CPAKM_GROUP_ID_LEN {
        return Err(PathSecretAeadKeyingError::NonCanonicalGroupIdLength);
    }
    // Rule 2: epoch advances by exactly 1.
    if commit.epoch != view.local_epoch.saturating_add(1) {
        return Err(PathSecretAeadKeyingError::EpochMismatch);
    }
    // Rule 3: sender_leaf in range.
    if commit.sender_leaf >= view.tree_size {
        return Err(PathSecretAeadKeyingError::SenderLeafOutOfRange);
    }
    // Rule 4: local slot index in range.
    if view.local_resolution_index >= commit.update_path_slots.len() {
        return Err(PathSecretAeadKeyingError::ResolutionSlotOutOfRange);
    }
    let slot = &commit.update_path_slots[view.local_resolution_index];
    // Rule 7 (checked before crypto-bound rules 5/6 — wrong-length
    // ciphertext is rejected pre-decap):
    if slot.ciphertext.len() != CPAKM_PATH_SECRET_CIPHERTEXT_LEN {
        return Err(PathSecretAeadKeyingError::NonCanonicalCiphertextLength);
    }
    // Rule 5: recipient init_key matches local.
    if slot.recipient_init_key != view.local_init_key {
        return Err(PathSecretAeadKeyingError::RecipientInitKeyMismatch);
    }
    // Rule 6: AAD-context matches the canonical encoding.
    let expected = canonical_aad_context(
        &commit.group_id,
        commit.epoch,
        commit.sender_leaf,
        view.local_node_index,
    );
    if slot.aad_context != expected || slot.node_index != view.local_node_index {
        return Err(PathSecretAeadKeyingError::AadContextMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_group_id() -> Vec<u8> {
        vec![0x11_u8; CPAKM_GROUP_ID_LEN]
    }

    fn ok_init_key() -> Vec<u8> {
        vec![0x22_u8; CPAKM_INIT_KEY_LEN]
    }

    fn ok_other_init_key() -> Vec<u8> {
        vec![0x33_u8; CPAKM_INIT_KEY_LEN]
    }

    fn ok_view() -> UpdatePathView {
        UpdatePathView {
            local_epoch: 41,
            tree_size: 16,
            local_init_key: ok_init_key(),
            local_resolution_index: 0,
            local_node_index: 7,
        }
    }

    fn ok_slot() -> UpdatePathSlot {
        UpdatePathSlot {
            node_index: 7,
            recipient_init_key: ok_init_key(),
            aad_context: canonical_aad_context(&ok_group_id(), 42, 3, 7),
            ciphertext: vec![0x44_u8; CPAKM_PATH_SECRET_CIPHERTEXT_LEN],
        }
    }

    fn ok_commit() -> CommitUpdatePath {
        CommitUpdatePath {
            group_id: ok_group_id(),
            epoch: 42,
            sender_leaf: 3,
            update_path_slots: vec![ok_slot()],
        }
    }

    /// CPAKM-01 — short group_id (16 bytes) rejected — Rule 1.
    #[test]
    fn cpakm_01_short_group_id_rejected() {
        let mut c = ok_commit();
        c.group_id = vec![0x11_u8; 16];
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::NonCanonicalGroupIdLength)
        );
    }

    /// CPAKM-02 — epoch not advancing by 1 rejected — Rule 2.
    #[test]
    fn cpakm_02_epoch_skip_rejected() {
        let mut c = ok_commit();
        c.epoch = 44; // local 41 + 1 should be 42
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::EpochMismatch)
        );
    }

    /// CPAKM-03 — sender_leaf out of range rejected — Rule 3.
    #[test]
    fn cpakm_03_sender_leaf_out_of_range_rejected() {
        let mut c = ok_commit();
        c.sender_leaf = 16; // tree_size = 16
        // Adjust AAD to the bad sender so we reach Rule 3 first.
        c.update_path_slots[0].aad_context =
            canonical_aad_context(&ok_group_id(), 42, 16, 7);
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::SenderLeafOutOfRange)
        );
    }

    /// CPAKM-04 — local slot index out of range rejected — Rule 4.
    #[test]
    fn cpakm_04_resolution_slot_out_of_range_rejected() {
        let c = ok_commit();
        let mut v = ok_view();
        v.local_resolution_index = 1; // only one slot in commit
        assert_eq!(
            validate_commit_path_secret(&c, &v),
            Err(PathSecretAeadKeyingError::ResolutionSlotOutOfRange)
        );
    }

    /// CPAKM-05 — recipient_init_key mismatch rejected — Rule 5.
    #[test]
    fn cpakm_05_recipient_init_key_mismatch_rejected() {
        let mut c = ok_commit();
        c.update_path_slots[0].recipient_init_key = ok_other_init_key();
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::RecipientInitKeyMismatch)
        );
    }

    /// CPAKM-06 — AAD-context mismatch (wrong epoch in AAD) rejected
    /// — Rule 6.
    #[test]
    fn cpakm_06_aad_context_epoch_mismatch_rejected() {
        let mut c = ok_commit();
        c.update_path_slots[0].aad_context =
            canonical_aad_context(&ok_group_id(), 999, 3, 7);
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::AadContextMismatch)
        );
    }

    /// CPAKM-07 — AAD-context disagrees on node_index — Rule 6.
    #[test]
    fn cpakm_07_aad_context_node_index_mismatch_rejected() {
        let mut c = ok_commit();
        c.update_path_slots[0].node_index = 9; // view says 7
        c.update_path_slots[0].aad_context =
            canonical_aad_context(&ok_group_id(), 42, 3, 9);
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::AadContextMismatch)
        );
    }

    /// CPAKM-08 — short ciphertext (32 bytes — missing tag) rejected
    /// — Rule 7.
    #[test]
    fn cpakm_08_short_ciphertext_rejected() {
        let mut c = ok_commit();
        c.update_path_slots[0].ciphertext = vec![0x44_u8; 32];
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::NonCanonicalCiphertextLength)
        );
    }

    /// CPAKM-09 — long ciphertext (64 bytes) rejected — Rule 7.
    #[test]
    fn cpakm_09_long_ciphertext_rejected() {
        let mut c = ok_commit();
        c.update_path_slots[0].ciphertext = vec![0x44_u8; 64];
        assert_eq!(
            validate_commit_path_secret(&c, &ok_view()),
            Err(PathSecretAeadKeyingError::NonCanonicalCiphertextLength)
        );
    }

    /// CPAKM-10 — canonical update-path slot accepted.
    #[test]
    fn cpakm_10_canonical_commit_accepted() {
        assert_eq!(
            validate_commit_path_secret(&ok_commit(), &ok_view()),
            Ok(())
        );
    }
}
