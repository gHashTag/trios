//! # CR-CHAT-02 — Header key derivation epoch separation guard (Wave-150 Lane A)
//!
//! RATCHET — header key derivations must use unique epoch labels;
//! reuse enables cross-epoch attacks.
//!
//! In the Double Ratchet, each message header is encrypted with a
//! key derived from the current epoch. If the same epoch label is
//! reused across different header key derivations:
//!
//! * **Cross-epoch attack** — an attacker can replay headers from
//!   one epoch in another if they share derivation labels.
//! * **Key confusion** — same label + different inputs produces
//!   related keys, leaking information about inputs.
//! * **Header forgery** — if epoch labels aren't unique, an attacker
//!   can forge headers that appear valid for multiple epochs.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All epoch+label pairs must be unique.
//! 2. Epoch must be > 0.
//! 3. Label must not be zero.
//! 4. No duplicate (epoch, label) pairs.
//! 5. Header ID must not be zero.
//! 6. Batch size <= `HKDE_MAX_HEADERS`.
//!
//! Tests **HKDE-01..10**. Error enum [`EpochSeparationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-SEPARATE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum headers per batch.
pub const HKDE_MAX_HEADERS: usize = 512;

/// Header ID length.
pub const HKDE_HEADER_ID_LEN: usize = 16;

/// Label length.
pub const HKDE_LABEL_LEN: usize = 32;

/// A header key derivation record.
#[derive(Debug, Clone)]
pub struct HeaderDerivationRecord {
    /// Header identifier.
    pub header_id: [u8; HKDE_HEADER_ID_LEN],
    /// Epoch number.
    pub epoch: u64,
    /// Derivation label.
    pub label: [u8; HKDE_LABEL_LEN],
}

/// All ways epoch separation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochSeparationError {
    /// Duplicate epoch+label pair.
    DuplicateEpochLabel {
        /// Index.
        idx: usize,
    },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Zero label.
    ZeroLabel(usize),
    /// Zero header ID.
    ZeroHeaderId(usize),
    /// Duplicate header ID.
    DuplicateHeaderId {
        /// Index.
        idx: usize,
    },
    /// Too many headers.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate header key derivation epoch separation.
pub fn validate_epoch_separation(
    headers: &[HeaderDerivationRecord],
) -> Result<(), EpochSeparationError> {
    if headers.len() > HKDE_MAX_HEADERS {
        return Err(EpochSeparationError::TooMany {
            got: headers.len(),
            max: HKDE_MAX_HEADERS,
        });
    }
    let mut seen_ids: BTreeSet<[u8; HKDE_HEADER_ID_LEN]> = BTreeSet::new();
    let mut seen_pairs: BTreeSet<(u64, [u8; HKDE_LABEL_LEN])> = BTreeSet::new();
    for (i, h) in headers.iter().enumerate() {
        if h.header_id == [0u8; HKDE_HEADER_ID_LEN] {
            return Err(EpochSeparationError::ZeroHeaderId(i));
        }
        if !seen_ids.insert(h.header_id) {
            return Err(EpochSeparationError::DuplicateHeaderId { idx: i });
        }
        if h.epoch == 0 {
            return Err(EpochSeparationError::ZeroEpoch(i));
        }
        if h.label == [0u8; HKDE_LABEL_LEN] {
            return Err(EpochSeparationError::ZeroLabel(i));
        }
        if !seen_pairs.insert((h.epoch, h.label)) {
            return Err(EpochSeparationError::DuplicateEpochLabel { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hid(byte: u8) -> [u8; HKDE_HEADER_ID_LEN] {
        [byte; HKDE_HEADER_ID_LEN]
    }

    fn lbl(byte: u8) -> [u8; HKDE_LABEL_LEN] {
        [byte; HKDE_LABEL_LEN]
    }

    fn hdr(id: u8, epoch: u64, label: u8) -> HeaderDerivationRecord {
        HeaderDerivationRecord { header_id: hid(id), epoch, label: lbl(label) }
    }

    fn valid_headers() -> Vec<HeaderDerivationRecord> {
        vec![
            hdr(0x01, 1, 0xA1),
            hdr(0x02, 1, 0xA2),
            hdr(0x03, 2, 0xA1),
        ]
    }

    /// **HKDE-01** — duplicate epoch+label rejected.
    #[test]
    fn hkde_01_duplicate_pair_rejected() {
        let hs = vec![
            hdr(0x01, 1, 0xA1),
            hdr(0x02, 1, 0xA1),
        ];
        assert_eq!(
            validate_epoch_separation(&hs),
            Err(EpochSeparationError::DuplicateEpochLabel { idx: 1 })
        );
    }

    /// **HKDE-02** — zero epoch rejected.
    #[test]
    fn hkde_02_zero_epoch_rejected() {
        let h = HeaderDerivationRecord { header_id: hid(0x01), epoch: 0, label: lbl(0xA1) };
        assert_eq!(
            validate_epoch_separation(&[h]),
            Err(EpochSeparationError::ZeroEpoch(0))
        );
    }

    /// **HKDE-03** — zero label rejected.
    #[test]
    fn hkde_03_zero_label_rejected() {
        let h = HeaderDerivationRecord { header_id: hid(0x01), epoch: 1, label: [0u8; HKDE_LABEL_LEN] };
        assert_eq!(
            validate_epoch_separation(&[h]),
            Err(EpochSeparationError::ZeroLabel(0))
        );
    }

    /// **HKDE-04** — zero header ID rejected.
    #[test]
    fn hkde_04_zero_header_rejected() {
        let h = HeaderDerivationRecord { header_id: [0u8; HKDE_HEADER_ID_LEN], epoch: 1, label: lbl(0xA1) };
        assert_eq!(
            validate_epoch_separation(&[h]),
            Err(EpochSeparationError::ZeroHeaderId(0))
        );
    }

    /// **HKDE-05** — duplicate header ID rejected.
    #[test]
    fn hkde_05_duplicate_header_rejected() {
        let hs = vec![
            hdr(0x01, 1, 0xA1),
            hdr(0x01, 2, 0xA2),
        ];
        assert_eq!(
            validate_epoch_separation(&hs),
            Err(EpochSeparationError::DuplicateHeaderId { idx: 1 })
        );
    }

    /// **HKDE-06** — too many rejected.
    #[test]
    fn hkde_06_too_many_rejected() {
        let hs: Vec<HeaderDerivationRecord> = (0..=HKDE_MAX_HEADERS)
            .map(|i| {
                let mut id = [0u8; HKDE_HEADER_ID_LEN];
                let mut lb = [0u8; HKDE_LABEL_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                lb[0..8].copy_from_slice(&val.to_be_bytes());
                HeaderDerivationRecord { header_id: id, epoch: val, label: lb }
            })
            .collect();
        assert_eq!(
            validate_epoch_separation(&hs),
            Err(EpochSeparationError::TooMany {
                got: HKDE_MAX_HEADERS + 1,
                max: HKDE_MAX_HEADERS,
            })
        );
    }

    /// **HKDE-07** — valid accepted.
    #[test]
    fn hkde_07_valid_accepted() {
        assert_eq!(validate_epoch_separation(&valid_headers()), Ok(()));
    }

    /// **HKDE-08** — empty accepted.
    #[test]
    fn hkde_08_empty_accepted() {
        assert_eq!(validate_epoch_separation(&[]), Ok(()));
    }

    /// **HKDE-09** — same label different epoch accepted.
    #[test]
    fn hkde_09_same_label_diff_epoch() {
        let hs = vec![
            hdr(0x01, 1, 0xA1),
            hdr(0x02, 2, 0xA1),
        ];
        assert_eq!(validate_epoch_separation(&hs), Ok(()));
    }

    /// **HKDE-10** — many unique pairs accepted.
    #[test]
    fn hkde_10_many_unique_accepted() {
        let hs: Vec<HeaderDerivationRecord> = (0..30u8)
            .map(|i| hdr(i + 1, (i as u64) / 10 + 1, 0xA0 + i % 10))
            .collect();
        assert_eq!(validate_epoch_separation(&hs), Ok(()));
    }
}
