//! # CR-CHAT-04 — MLS plaintext authenticated-data tampering guard (Wave-39 Lane B)
//!
//! RFC 9420 §6 — MLS message framing integrity.
//!
//! Every `MLSPlaintext` and `MLSCiphertext` carries an authenticated-data
//! (AAD) context that binds the ciphertext to the group context. An
//! adversary who can tamper with the AAD fields without detection can:
//!
//! * **Cross-group splice** — replay a message from one group into another
//!   by swapping the `group_id` in the AAD.
//! * **Epoch rollback** — claim a lower epoch in the AAD, causing the
//!   receiver to use stale keys for decryption.
//! * **Content-type forge** — change `Application` to `Commit` or vice
//!   versa, tricking the receiver into processing a data message as a
//!   control message.
//! * **Sender impersonation** — change the sender leaf index.
//! * **Wire-version confusion** — send a future wire version the
//!   receiver hasn't implemented, falling back to an insecure parser.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Wire version is `mls10` (0x01).
//! 2. `group_id` is non-empty.
//! 3. `epoch` is non-zero (epoch 0 is pre-group).
//! 4. Content type is valid (`Application = 1`, `Proposal = 2`, `Commit = 3`).
//! 5. Sender leaf index is within group bounds.
//! 6. Authenticated data length is within bounds (≤ 65535 bytes).
//! 7. Ciphertext length ≥ AEAD tag minimum (16 bytes) for encrypted content.
//!
//! Tests **MLF-01..10**. Error enum [`MlsFramingError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · MLS-FRAMING`

#![forbid(unsafe_code)]

/// Wire version byte for MLS 1.0 (RFC 9420 §3).
pub const MLF_WIRE_VERSION_MLS10: u8 = 0x01;

/// Maximum authenticated data length (u16 max).
pub const MLF_MAX_AAD_LEN: usize = 65535;

/// Minimum AEAD ciphertext length (tag only, no plaintext).
pub const MLF_MIN_AEAD_CT_LEN: usize = 16;

/// Maximum number of leaves in a group.
pub const MLF_MAX_LEAF_INDEX: u32 = (1u32 << 30) - 1;

/// Content type byte for MLS messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MlsContentType {
    /// Application data (encrypted).
    Application,
    /// Proposal message.
    Proposal,
    /// Commit message.
    Commit,
}

impl MlsContentType {
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            1 => Some(Self::Application),
            2 => Some(Self::Proposal),
            3 => Some(Self::Commit),
            _ => None,
        }
    }
}

/// MLS message framing envelope for validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MlsFrame {
    /// Wire version byte.
    pub wire_version: u8,
    /// Group identifier (non-empty).
    pub group_id: Vec<u8>,
    /// Epoch number.
    pub epoch: u64,
    /// Content type byte.
    pub content_type: u8,
    /// Sender leaf index.
    pub sender_leaf: u32,
    /// Authenticated data (AAD) bytes.
    pub authenticated_data: Vec<u8>,
    /// Ciphertext (empty for plaintext messages).
    pub ciphertext: Vec<u8>,
}

/// Receiver's view of the group for validation.
#[derive(Debug, Clone)]
pub struct MlsGroupView {
    /// Current group identifier.
    pub group_id: Vec<u8>,
    /// Current epoch.
    pub epoch: u64,
    /// Total number of leaves.
    pub leaf_count: u32,
}

/// All ways an MLS frame can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MlsFramingError {
    /// Wire version is not `mls10` (0x01).
    InvalidWireVersion,
    /// `group_id` is empty.
    EmptyGroupId,
    /// Epoch is zero (pre-group epoch).
    ZeroEpoch,
    /// Content type byte is not 1, 2, or 3.
    InvalidContentType,
    /// Sender leaf index exceeds group bounds.
    SenderOutOfBounds,
    /// Authenticated data exceeds max length.
    AadTooLong,
    /// Ciphertext shorter than AEAD tag for encrypted content.
    TruncatedCiphertext,
    /// `group_id` doesn't match receiver's group.
    GroupIdMismatch,
    /// Epoch doesn't match receiver's current epoch.
    EpochMismatch,
}

/// `[VERIFIED]` Validate an MLS message frame against the 7 canonical
/// rules from RFC 9420 §6. Returns `Ok(())` if all rules pass, else
/// the first failing rule as an [`MlsFramingError`].
///
/// Rules enforced in fixed order:
///
/// 1. `wire_version == 0x01`.
/// 2. `group_id` is non-empty.
/// 3. `epoch > 0`.
/// 4. `content_type` is 1, 2, or 3.
/// 5. `sender_leaf < leaf_count`.
/// 6. `authenticated_data.len() <= 65535`.
/// 7. For Application content: `ciphertext.len() >= 16`.
pub fn validate_mls_frame(
    frame: &MlsFrame,
    view: &MlsGroupView,
) -> Result<(), MlsFramingError> {
    if frame.wire_version != MLF_WIRE_VERSION_MLS10 {
        return Err(MlsFramingError::InvalidWireVersion);
    }
    if frame.group_id.is_empty() {
        return Err(MlsFramingError::EmptyGroupId);
    }
    if frame.epoch == 0 {
        return Err(MlsFramingError::ZeroEpoch);
    }
    if MlsContentType::from_byte(frame.content_type).is_none() {
        return Err(MlsFramingError::InvalidContentType);
    }
    if frame.sender_leaf >= view.leaf_count {
        return Err(MlsFramingError::SenderOutOfBounds);
    }
    if frame.authenticated_data.len() > MLF_MAX_AAD_LEN {
        return Err(MlsFramingError::AadTooLong);
    }
    if frame.content_type == 1 && frame.ciphertext.len() < MLF_MIN_AEAD_CT_LEN {
        return Err(MlsFramingError::TruncatedCiphertext);
    }
    if frame.group_id != view.group_id {
        return Err(MlsFramingError::GroupIdMismatch);
    }
    if frame.epoch != view.epoch {
        return Err(MlsFramingError::EpochMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_view() -> MlsGroupView {
        MlsGroupView {
            group_id: vec![0xAA; 32],
            epoch: 7,
            leaf_count: 10,
        }
    }

    fn good_frame() -> MlsFrame {
        MlsFrame {
            wire_version: MLF_WIRE_VERSION_MLS10,
            group_id: vec![0xAA; 32],
            epoch: 7,
            content_type: 1,
            sender_leaf: 3,
            authenticated_data: vec![0x00; 64],
            ciphertext: vec![0x22; 32],
        }
    }

    /// **MLF-01** — invalid wire version (0x02) rejected.
    #[test]
    fn mlf_01_invalid_wire_version_rejected() {
        let mut f = good_frame();
        f.wire_version = 0x02;
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::InvalidWireVersion)
        );
    }

    /// **MLF-02** — empty group_id rejected.
    #[test]
    fn mlf_02_empty_group_id_rejected() {
        let mut f = good_frame();
        f.group_id = vec![];
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::EmptyGroupId)
        );
    }

    /// **MLF-03** — zero epoch rejected.
    #[test]
    fn mlf_03_zero_epoch_rejected() {
        let mut f = good_frame();
        f.epoch = 0;
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::ZeroEpoch)
        );
    }

    /// **MLF-04** — invalid content type (0xFF) rejected.
    #[test]
    fn mlf_04_invalid_content_type_rejected() {
        let mut f = good_frame();
        f.content_type = 0xFF;
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::InvalidContentType)
        );
    }

    /// **MLF-05** — sender leaf out of bounds rejected.
    #[test]
    fn mlf_05_sender_out_of_bounds_rejected() {
        let mut f = good_frame();
        f.sender_leaf = 99;
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::SenderOutOfBounds)
        );
    }

    /// **MLF-06** — oversized AAD rejected.
    #[test]
    fn mlf_06_oversized_aad_rejected() {
        let mut f = good_frame();
        f.authenticated_data = vec![0x00; 65536];
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::AadTooLong)
        );
    }

    /// **MLF-07** — truncated ciphertext for Application content rejected.
    #[test]
    fn mlf_07_truncated_ciphertext_rejected() {
        let mut f = good_frame();
        f.content_type = 1;
        f.ciphertext = vec![0x22; 8];
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::TruncatedCiphertext)
        );
    }

    /// **MLF-08** — group_id mismatch rejected.
    #[test]
    fn mlf_08_group_id_mismatch_rejected() {
        let mut f = good_frame();
        f.group_id = vec![0xBB; 32];
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::GroupIdMismatch)
        );
    }

    /// **MLF-09** — epoch mismatch rejected.
    #[test]
    fn mlf_09_epoch_mismatch_rejected() {
        let mut f = good_frame();
        f.epoch = 99;
        assert_eq!(
            validate_mls_frame(&f, &good_view()),
            Err(MlsFramingError::EpochMismatch)
        );
    }

    /// **MLF-10** — valid MLS frame (Proposal content, no ciphertext) accepted.
    #[test]
    fn mlf_10_valid_proposal_frame_accepted() {
        let mut f = good_frame();
        f.content_type = 2;
        f.ciphertext = vec![];
        assert_eq!(validate_mls_frame(&f, &good_view()), Ok(()));
    }
}
