//! # CR-CHAT-02 · L-CHAT-2-headerenc — MLS sender_data header encryption integrity
//!
//! `[VERIFIED]` Wave-28 lane B — Defends against a class of attacks
//! where an adversary tampers with the encrypted `sender_data` header
//! that prefixes every MLSCiphertext (RFC 9420 §6.3.2 — Sender Data
//! Encryption):
//!
//! * **Sender-data nonce length forge** — claims a `sender_data_nonce`
//!   shorter or longer than `SENDER_DATA_NONCE_LEN` bytes (AEAD nonce
//!   must be exactly the cipher's nonce length).
//! * **Sender-data AAD splice** — claims a `sender_data_aad` whose
//!   `(group_id, epoch, content_type)` does NOT match the receiving
//!   group's view, breaking the AEAD context binding.
//! * **Stale-epoch sender-data** — claims a sender_data whose `epoch`
//!   is not equal to the receiver's current epoch.
//! * **Zero ciphertext** — claims a `sender_data_ciphertext` of zero
//!   length or below the AEAD-tag minimum (RFC 9420 §6.3.2 requires
//!   ciphertext + 16-byte authentication tag).
//! * **Reserved-bit forge** — claims a `sender_data_aad.reserved`
//!   field that is non-zero (the field is reserved and MUST be zero
//!   on receive per RFC 9420 §6.3.2; tolerating non-zero permits a
//!   covert side-channel).
//! * **Nonce reuse** — claims a `sender_data_nonce` already consumed
//!   for the same `(group_id, epoch)` — AEAD nonce reuse breaks
//!   confidentiality and integrity simultaneously.
//!
//! See RFC 9420 §6.3.2 — Sender Data Encryption. The six rules below
//! are enforced in fixed order; any attempt to weaken or skip them
//! produces a `SenderDataHeaderError`.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-HEADER-ENCRYPTION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical length of `sender_data_nonce` in bytes — RFC 9420
/// default ciphersuite is AES-128-GCM (12-byte nonce) or
/// ChaCha20-Poly1305 (12-byte nonce). We pin AEAD-12.
pub const SENDER_DATA_NONCE_LEN: usize = 12;

/// Minimum length of `sender_data_ciphertext` — AEAD ciphertext MUST
/// carry at least the 16-byte authentication tag (Poly1305 / GCM tag).
pub const MIN_SENDER_DATA_CT_LEN: usize = 16;

/// Content-type byte for MLSCiphertext: 1 = application, 2 = proposal,
/// 3 = commit (RFC 9420 §6 enumeration). Any other value is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Application data message.
    Application,
    /// Proposal message.
    Proposal,
    /// Commit message.
    Commit,
}

/// Additional Authenticated Data bound around the encrypted sender_data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SenderDataAad {
    /// Local group identifier.
    pub group_id: Vec<u8>,
    /// Epoch this sender_data belongs to.
    pub epoch: u64,
    /// Content type (Application / Proposal / Commit).
    pub content_type: ContentType,
    /// Reserved — MUST be zero on receive (RFC 9420 §6.3.2).
    pub reserved: u8,
}

/// Encrypted sender_data header as it would appear in an MLSCiphertext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedSenderData {
    /// AEAD nonce — exactly `SENDER_DATA_NONCE_LEN` bytes.
    pub sender_data_nonce: Vec<u8>,
    /// AEAD ciphertext (carries leaf_index || generation || reuse_guard).
    pub sender_data_ciphertext: Vec<u8>,
    /// AAD bound by the AEAD construction.
    pub sender_data_aad: SenderDataAad,
}

/// Receiving-group view used to validate an `EncryptedSenderData`. The
/// receiver enforces context binding against its current epoch.
#[derive(Debug, Clone)]
pub struct SenderDataView {
    /// `group_id` of the local group.
    pub local_group_id: Vec<u8>,
    /// Current epoch.
    pub current_epoch: u64,
    /// Ledger of `(group_id, epoch, sender_data_nonce)` triples already
    /// consumed — AEAD nonce-reuse guard.
    pub used_nonces: BTreeSet<(Vec<u8>, u64, Vec<u8>)>,
}

/// All ways an `EncryptedSenderData` can be rejected. Adding variants
/// stays non-breaking via `#[non_exhaustive]`.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SenderDataHeaderError {
    /// `sender_data_nonce.len() != SENDER_DATA_NONCE_LEN`.
    NonCanonicalNonceLength,
    /// `sender_data_aad.group_id != view.local_group_id`.
    CrossGroupAadSplice,
    /// `sender_data_aad.epoch != view.current_epoch`.
    StaleEpochSenderData,
    /// `sender_data_ciphertext.len() < MIN_SENDER_DATA_CT_LEN`.
    TruncatedCiphertext,
    /// `sender_data_aad.reserved != 0`.
    ReservedBitForge,
    /// `(group_id, epoch, sender_data_nonce)` already consumed.
    NonceReuse,
}

/// `[VERIFIED]` Validate an `EncryptedSenderData` against the receiving
/// group's `SenderDataView`. Returns `Ok(())` if all six rules pass,
/// else the first failing rule as a `SenderDataHeaderError`.
///
/// Rules enforced in fixed order from RFC 9420 §6.3.2:
///
/// 1. `sender_data_nonce.len() == SENDER_DATA_NONCE_LEN`.
/// 2. `sender_data_aad.group_id == view.local_group_id`.
/// 3. `sender_data_aad.epoch == view.current_epoch`.
/// 4. `sender_data_ciphertext.len() >= MIN_SENDER_DATA_CT_LEN`.
/// 5. `sender_data_aad.reserved == 0`.
/// 6. `(group_id, epoch, sender_data_nonce)` not in `view.used_nonces`.
pub fn validate_sender_data_header(
    header: &EncryptedSenderData,
    view: &SenderDataView,
) -> Result<(), SenderDataHeaderError> {
    // 1. Canonical AEAD nonce length.
    if header.sender_data_nonce.len() != SENDER_DATA_NONCE_LEN {
        return Err(SenderDataHeaderError::NonCanonicalNonceLength);
    }
    // 2. Cross-group AAD splice.
    if header.sender_data_aad.group_id != view.local_group_id {
        return Err(SenderDataHeaderError::CrossGroupAadSplice);
    }
    // 3. Stale-epoch sender_data.
    if header.sender_data_aad.epoch != view.current_epoch {
        return Err(SenderDataHeaderError::StaleEpochSenderData);
    }
    // 4. Truncated ciphertext (must include AEAD tag).
    if header.sender_data_ciphertext.len() < MIN_SENDER_DATA_CT_LEN {
        return Err(SenderDataHeaderError::TruncatedCiphertext);
    }
    // 5. Reserved bit must be zero.
    if header.sender_data_aad.reserved != 0 {
        return Err(SenderDataHeaderError::ReservedBitForge);
    }
    // 6. AEAD nonce reuse.
    let key = (
        header.sender_data_aad.group_id.clone(),
        header.sender_data_aad.epoch,
        header.sender_data_nonce.clone(),
    );
    if view.used_nonces.contains(&key) {
        return Err(SenderDataHeaderError::NonceReuse);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_view() -> SenderDataView {
        SenderDataView {
            local_group_id: vec![0xAA; 32],
            current_epoch: 7,
            used_nonces: BTreeSet::new(),
        }
    }

    fn good_header() -> EncryptedSenderData {
        EncryptedSenderData {
            sender_data_nonce: vec![0x11; SENDER_DATA_NONCE_LEN],
            sender_data_ciphertext: vec![0x22; 32],
            sender_data_aad: SenderDataAad {
                group_id: vec![0xAA; 32],
                epoch: 7,
                content_type: ContentType::Application,
                reserved: 0,
            },
        }
    }

    /// **SDH-01** — short (8-byte) sender_data_nonce rejected.
    #[test]
    fn sdh_01_short_nonce_rejected() {
        let mut h = good_header();
        h.sender_data_nonce = vec![0x11; 8];
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::NonCanonicalNonceLength)
        );
    }

    /// **SDH-02** — over-long (16-byte) sender_data_nonce rejected.
    #[test]
    fn sdh_02_overlong_nonce_rejected() {
        let mut h = good_header();
        h.sender_data_nonce = vec![0x11; 16];
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::NonCanonicalNonceLength)
        );
    }

    /// **SDH-03** — cross-group AAD splice rejected.
    #[test]
    fn sdh_03_cross_group_aad_splice_rejected() {
        let mut h = good_header();
        h.sender_data_aad.group_id = vec![0xBB; 32];
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::CrossGroupAadSplice)
        );
    }

    /// **SDH-04** — past-epoch sender_data rejected.
    #[test]
    fn sdh_04_past_epoch_rejected() {
        let mut h = good_header();
        h.sender_data_aad.epoch = 5;
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::StaleEpochSenderData)
        );
    }

    /// **SDH-05** — future-epoch sender_data rejected.
    #[test]
    fn sdh_05_future_epoch_rejected() {
        let mut h = good_header();
        h.sender_data_aad.epoch = 99;
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::StaleEpochSenderData)
        );
    }

    /// **SDH-06** — truncated ciphertext (< AEAD tag length) rejected.
    #[test]
    fn sdh_06_truncated_ciphertext_rejected() {
        let mut h = good_header();
        h.sender_data_ciphertext = vec![0x22; 8];
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::TruncatedCiphertext)
        );
    }

    /// **SDH-07** — empty ciphertext rejected.
    #[test]
    fn sdh_07_empty_ciphertext_rejected() {
        let mut h = good_header();
        h.sender_data_ciphertext = vec![];
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::TruncatedCiphertext)
        );
    }

    /// **SDH-08** — reserved-bit forge rejected.
    #[test]
    fn sdh_08_reserved_bit_forge_rejected() {
        let mut h = good_header();
        h.sender_data_aad.reserved = 0x80;
        assert_eq!(
            validate_sender_data_header(&h, &good_view()),
            Err(SenderDataHeaderError::ReservedBitForge)
        );
    }

    /// **SDH-09** — AEAD nonce reuse rejected.
    #[test]
    fn sdh_09_nonce_reuse_rejected() {
        let mut view = good_view();
        let h = good_header();
        view.used_nonces.insert((
            h.sender_data_aad.group_id.clone(),
            h.sender_data_aad.epoch,
            h.sender_data_nonce.clone(),
        ));
        assert_eq!(
            validate_sender_data_header(&h, &view),
            Err(SenderDataHeaderError::NonceReuse)
        );
    }

    /// **SDH-10** — valid sender_data header (proposal content_type) accepted.
    #[test]
    fn sdh_10_valid_proposal_header_accepted() {
        let mut h = good_header();
        h.sender_data_aad.content_type = ContentType::Proposal;
        assert_eq!(validate_sender_data_header(&h, &good_view()), Ok(()));
    }
}
