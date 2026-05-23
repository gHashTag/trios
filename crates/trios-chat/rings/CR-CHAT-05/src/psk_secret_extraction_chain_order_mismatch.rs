//! # CR-CHAT-05 · CR-CHAT-05 — PSK secret extraction chain order mismatch guard
//!
//! Wave-38 Lane B — `psk_secret_extraction_chain_order_mismatch` (CR-CHAT-05).
//!
//! Constructive guard at the receiver against PSK chain reorder attacks.
//! Per RFC 9420 §15.1.1, the `psk_secret` derivation is order-sensitive:
//! `PSKLabel(i, n)` encodes both position `i` and total count `n`, so
//! reordering `psk_ids`, swapping label.count, or duplicating PSKids all
//! produce a different `psk_secret`. Loose stacks have been caught sorting
//! `psk_ids` by hash, accepting duplicates, accepting
//! `PSKLabel.count != psk_ids.len()`, or accepting non-monotonic indices —
//! any one yielding a covert DoS where two endpoints derive divergent secrets.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Canonical 32-byte PSKid length.
//! 2. Non-empty list.
//! 3. List length ≤ 8 (OpenMLS cap).
//! 4. `label.count == list.len()`.
//! 5. `label.index < label.count`.
//! 6. `label.index == position` (strict monotonic).
//! 7. No duplicate PSKids.
//!
//! Tests **PSCOM-01..10**. Error enum [`PskChainOrderError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PSCOM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical PSK identifier length in bytes.
pub const PSCOM_PSKID_LEN: usize = 32;

/// Maximum number of PSKs per Commit/Welcome (OpenMLS cap).
pub const PSCOM_MAX_PSK_COUNT: usize = 8;

/// PSK label encoding as defined in RFC 9420 §15.1.1. Carries both the
/// positional index and total count, so the receiver can verify strict
/// monotonic ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PskLabel {
    /// Positional index of this PSK in the list.
    pub index: usize,
    /// Total number of PSKs in the Commit/Welcome.
    pub count: usize,
}

/// Parameters for a PSK chain validation. The receiver checks the entire
/// `psk_ids` list against the corresponding label at each position.
#[derive(Debug, Clone)]
pub struct PskChainParams {
    /// Ordered list of PSK identifiers (each 32 bytes).
    pub psk_ids: Vec<Vec<u8>>,
    /// Labels corresponding to each position in `psk_ids`.
    pub labels: Vec<PskLabel>,
}

/// All ways a PSK chain can be rejected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PskChainOrderError {
    /// A PSKid is not exactly 32 bytes.
    NonCanonicalPsKidLength,
    /// The `psk_ids` list is empty.
    EmptyList,
    /// The `psk_ids` list exceeds `PSCOM_MAX_PSK_COUNT` (8).
    ListTooLong,
    /// `label.count != psk_ids.len()`.
    LabelCountDesync,
    /// `label.index >= label.count`.
    IndexOutOfBounds,
    /// `label.index != position` (not strictly monotonic).
    IndexNotMonotonic,
    /// Two or more PSKids are identical.
    DuplicatePsKid,
}

/// `[VERIFIED]` Validate a PSK chain against the 7 canonical rules from
/// RFC 9420 §15.1.1. Returns `Ok(())` if all rules pass, else the first
/// failing rule as a [`PskChainOrderError`].
///
/// Rules enforced in fixed order:
///
/// 1. Every `psk_id.len() == 32`.
/// 2. `psk_ids` is non-empty.
/// 3. `psk_ids.len() <= 8`.
/// 4. Every `label.count == psk_ids.len()`.
/// 5. Every `label.index < label.count`.
/// 6. Every `label.index == position`.
/// 7. No duplicate `psk_ids`.
pub fn validate_psk_chain_order(
    params: &PskChainParams,
) -> Result<(), PskChainOrderError> {
    for id in &params.psk_ids {
        if id.len() != PSCOM_PSKID_LEN {
            return Err(PskChainOrderError::NonCanonicalPsKidLength);
        }
    }
    if params.psk_ids.is_empty() {
        return Err(PskChainOrderError::EmptyList);
    }
    if params.psk_ids.len() > PSCOM_MAX_PSK_COUNT {
        return Err(PskChainOrderError::ListTooLong);
    }
    for label in &params.labels {
        if label.count != params.psk_ids.len() {
            return Err(PskChainOrderError::LabelCountDesync);
        }
    }
    for label in &params.labels {
        if label.index >= label.count {
            return Err(PskChainOrderError::IndexOutOfBounds);
        }
    }
    for (pos, label) in params.labels.iter().enumerate() {
        if label.index != pos {
            return Err(PskChainOrderError::IndexNotMonotonic);
        }
    }
    let mut seen = BTreeSet::new();
    for id in &params.psk_ids {
        if !seen.insert(id.clone()) {
            return Err(PskChainOrderError::DuplicatePsKid);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pskid(byte: u8) -> Vec<u8> {
        vec![byte; PSCOM_PSKID_LEN]
    }

    fn monotonic_labels(count: usize) -> Vec<PskLabel> {
        (0..count)
            .map(|i| PskLabel {
                index: i,
                count,
            })
            .collect()
    }

    fn good_params() -> PskChainParams {
        PskChainParams {
            psk_ids: vec![pskid(0x01), pskid(0x02), pskid(0x03)],
            labels: monotonic_labels(3),
        }
    }

    /// **PSCOM-01** — non-canonical PSKid length (16 bytes) rejected.
    #[test]
    fn pscom_01_short_pskid_rejected() {
        let mut p = good_params();
        p.psk_ids[1] = vec![0x02; 16];
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::NonCanonicalPsKidLength)
        );
    }

    /// **PSCOM-02** — empty psk_ids list rejected.
    #[test]
    fn pscom_02_empty_list_rejected() {
        let p = PskChainParams {
            psk_ids: vec![],
            labels: vec![],
        };
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::EmptyList)
        );
    }

    /// **PSCOM-03** — list exceeding cap (9 > 8) rejected.
    #[test]
    fn pscom_03_list_too_long_rejected() {
        let mut ids = Vec::new();
        for i in 0..9u8 {
            ids.push(vec![i; PSCOM_PSKID_LEN]);
        }
        let p = PskChainParams {
            psk_ids: ids,
            labels: monotonic_labels(9),
        };
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::ListTooLong)
        );
    }

    /// **PSCOM-04** — label.count != psk_ids.len() rejected.
    #[test]
    fn pscom_04_label_count_desync_rejected() {
        let mut p = good_params();
        p.labels[0].count = 99;
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::LabelCountDesync)
        );
    }

    /// **PSCOM-05** — label.index >= label.count rejected.
    #[test]
    fn pscom_05_index_out_of_bounds_rejected() {
        let mut p = good_params();
        p.labels[0].index = 10;
        p.labels[0].count = 3;
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::IndexOutOfBounds)
        );
    }

    /// **PSCOM-06** — non-monotonic index rejected (swapped positions).
    #[test]
    fn pscom_06_index_not_monotonic_rejected() {
        let mut p = good_params();
        p.labels[0].index = 1;
        p.labels[1].index = 0;
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::IndexNotMonotonic)
        );
    }

    /// **PSCOM-07** — duplicate PSKid rejected.
    #[test]
    fn pscom_07_duplicate_pskid_rejected() {
        let mut p = good_params();
        p.psk_ids[1] = pskid(0x01);
        assert_eq!(
            validate_psk_chain_order(&p),
            Err(PskChainOrderError::DuplicatePsKid)
        );
    }

    /// **PSCOM-08** — exact-cap list (8 PSKs) accepted.
    #[test]
    fn pscom_08_exact_cap_accepted() {
        let ids: Vec<Vec<u8>> = (0..8u8).map(|i| vec![i; PSCOM_PSKID_LEN]).collect();
        let p = PskChainParams {
            psk_ids: ids,
            labels: monotonic_labels(8),
        };
        assert_eq!(validate_psk_chain_order(&p), Ok(()));
    }

    /// **PSCOM-09** — singleton list accepted.
    #[test]
    fn pscom_09_singleton_accepted() {
        let p = PskChainParams {
            psk_ids: vec![pskid(0xAA)],
            labels: monotonic_labels(1),
        };
        assert_eq!(validate_psk_chain_order(&p), Ok(()));
    }

    /// **PSCOM-10** — canonical multi-PSK list accepted.
    #[test]
    fn pscom_10_canonical_multi_psk_accepted() {
        assert_eq!(validate_psk_chain_order(&good_params()), Ok(()));
    }
}
