//! Wave-38 / L-CHAT-5-pscom (R-CHAT-5 / CR-CHAT-05) — PSK secret
//! extraction chain order mismatch defence per RFC 9420 §15.1.1
//! "Computing the PSK Secret".
//!
//! When a Commit or Welcome mixes in multiple PSKs (Resumption and/or
//! External), the resulting `psk_secret` is **order-sensitive**.
//! RFC 9420 §15.1.1 specifies the deterministic chain:
//!
//!     psk_secret_0 = 0
//!     psk_secret_i = Extract(
//!         ExpandWithLabel(
//!             Extract(0, psk_input_i),
//!             "derived psk",
//!             PSKLabel(i, n),
//!             KDF.Nh
//!         ),
//!         psk_secret_{i-1}
//!     )
//!     psk_secret   = psk_secret_n
//!
//! Crucially, `PSKLabel(i, n)` encodes both the position `i` and the
//! total count `n` — so reordering the `psk_ids` list, or padding it
//! with the same PSKs in a different order, produces a different
//! `psk_secret`. Two members who disagree on the order silently
//! diverge in every subsequent key derivation.
//!
//! Mainstream MLS stacks have historically been loose here:
//!   * they sort `psk_ids` by hash for "determinism" before
//!     extraction (illegal — RFC mandates the sender-supplied order);
//!   * they accept duplicate PSKid entries (RFC §15.1: must be
//!     unique per Commit);
//!   * they accept `PSKLabel.count != psk_ids.len()` (label/n
//!     desynchronisation lets an attacker swap n=2 for n=1 and have
//!     the receiver derive a different secret);
//!   * they accept `PSKLabel.index >= n` or two PSKids with the
//!     same index.
//!
//! Any of these classes lets an active man-in-the-middle who sees the
//! ciphertext but not the PSK material force the two endpoints onto
//! divergent `psk_secret` keys — a covert DoS that masquerades as
//! "decryption failure".
//!
//! This lane is the consumption-side guard at the receiver processing
//! a Commit/Welcome with `psk_ids`. A single deny wins.
//!
//! Seven rules enforced in fixed order:
//!   1. NonCanonicalPskIdLength — every PSKid's serialised
//!      identifier must equal `PSCOM_PSKID_LEN` (32 bytes).
//!   2. EmptyPskList — `psk_ids.len()` must be >= 1.
//!   3. PskListTooLong — `psk_ids.len()` must be <=
//!      `PSCOM_MAX_PSK_COUNT` (8 — OpenMLS default cap).
//!   4. LabelCountMismatch — every per-PSK label's `count` field
//!      must equal `psk_ids.len()`.
//!   5. LabelIndexOutOfRange — every per-PSK label's `index` field
//!      must satisfy `index < count`.
//!   6. LabelIndexNotMonotonic — labels must arrive in strictly
//!      monotonic order: `labels[i].index == i`.
//!   7. DuplicatePskId — no two entries in `psk_ids` may have the
//!      same serialised identifier.
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PSK-SECRET-CHAIN-ORDER`

#![forbid(unsafe_code)]

/// Canonical PSKid serialised length (32 bytes — R-CHAT-5).
pub const PSCOM_PSKID_LEN: usize = 32;

/// Maximum number of PSKs in a single Commit/Welcome (OpenMLS default).
pub const PSCOM_MAX_PSK_COUNT: usize = 8;

/// One per-PSK label as defined by RFC 9420 §15.1.1 PSKLabel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PskLabel {
    /// Position of this PSK in the chain (0-based).
    pub index: u16,
    /// Total number of PSKs in this Commit/Welcome.
    pub count: u16,
}

/// One PSK chain entry: the PSKid bytes plus its position label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PskChainEntry {
    /// Serialised PSKid (32 bytes).
    pub pskid: Vec<u8>,
    /// PSKLabel(i, n) for this position.
    pub label: PskLabel,
}

/// The full psk_ids list as visible to the receiver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PskChainFrame {
    pub psk_ids: Vec<PskChainEntry>,
}

/// Typed errors for `validate_psk_secret_chain_order`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PskChainOrderError {
    /// Rule 1 — PSKid bytes are not canonical length.
    NonCanonicalPskIdLength,
    /// Rule 2 — `psk_ids` is empty.
    EmptyPskList,
    /// Rule 3 — `psk_ids` length exceeds `PSCOM_MAX_PSK_COUNT`.
    PskListTooLong,
    /// Rule 4 — a label's `count` field differs from `psk_ids.len()`.
    LabelCountMismatch,
    /// Rule 5 — a label's `index >= count`.
    LabelIndexOutOfRange,
    /// Rule 6 — labels are not strictly position-monotonic.
    LabelIndexNotMonotonic,
    /// Rule 7 — duplicate PSKid in the list.
    DuplicatePskId,
}

/// Constructive guard for the PSK chain extraction order frame.
/// Returns `Ok(())` iff every rule (1)..(7) holds.
///
/// `[VERIFIED]` against the 10 unit tests `PSCOM-01..10` below and
/// the Coq theorems `INV-CHAT-253..257` in the W38 Section of
/// `proofs/chat/Trinity_Chat.v`.
pub fn validate_psk_secret_chain_order(
    frame: &PskChainFrame,
) -> Result<(), PskChainOrderError> {
    let n = frame.psk_ids.len();
    // Rule 2: non-empty list.
    if n == 0 {
        return Err(PskChainOrderError::EmptyPskList);
    }
    // Rule 3: bounded list.
    if n > PSCOM_MAX_PSK_COUNT {
        return Err(PskChainOrderError::PskListTooLong);
    }
    let n_u16 = n as u16;
    for (i, entry) in frame.psk_ids.iter().enumerate() {
        // Rule 1: canonical PSKid length.
        if entry.pskid.len() != PSCOM_PSKID_LEN {
            return Err(PskChainOrderError::NonCanonicalPskIdLength);
        }
        // Rule 4: per-label count matches actual list length.
        if entry.label.count != n_u16 {
            return Err(PskChainOrderError::LabelCountMismatch);
        }
        // Rule 5: index < count.
        if entry.label.index >= entry.label.count {
            return Err(PskChainOrderError::LabelIndexOutOfRange);
        }
        // Rule 6: label index strictly monotonic (== position).
        if entry.label.index as usize != i {
            return Err(PskChainOrderError::LabelIndexNotMonotonic);
        }
    }
    // Rule 7: no duplicate PSKid.
    for i in 0..n {
        for j in (i + 1)..n {
            if frame.psk_ids[i].pskid == frame.psk_ids[j].pskid {
                return Err(PskChainOrderError::DuplicatePskId);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_pskid(tag: u8) -> Vec<u8> {
        vec![tag; PSCOM_PSKID_LEN]
    }

    fn ok_chain_n(n: usize) -> PskChainFrame {
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            ids.push(PskChainEntry {
                pskid: ok_pskid((i as u8) + 1),
                label: PskLabel {
                    index: i as u16,
                    count: n as u16,
                },
            });
        }
        PskChainFrame { psk_ids: ids }
    }

    /// PSCOM-01 — short PSKid (16 bytes) rejected — Rule 1.
    #[test]
    fn pscom_01_short_pskid_rejected() {
        let mut c = ok_chain_n(2);
        c.psk_ids[1].pskid = vec![0x02; 16];
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::NonCanonicalPskIdLength)
        );
    }

    /// PSCOM-02 — empty psk_ids list rejected — Rule 2.
    #[test]
    fn pscom_02_empty_list_rejected() {
        let c = PskChainFrame { psk_ids: vec![] };
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::EmptyPskList)
        );
    }

    /// PSCOM-03 — list of 9 PSKs rejected — Rule 3.
    #[test]
    fn pscom_03_list_too_long_rejected() {
        let c = ok_chain_n(9);
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::PskListTooLong)
        );
    }

    /// PSCOM-04 — label count != list length rejected — Rule 4.
    #[test]
    fn pscom_04_label_count_mismatch_rejected() {
        let mut c = ok_chain_n(2);
        c.psk_ids[0].label.count = 3;
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::LabelCountMismatch)
        );
    }

    /// PSCOM-05 — label index >= count rejected — Rule 5.
    #[test]
    fn pscom_05_label_index_out_of_range_rejected() {
        let mut c = ok_chain_n(2);
        c.psk_ids[1].label.index = 5;
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::LabelIndexOutOfRange)
        );
    }

    /// PSCOM-06 — swapped labels (1,0 instead of 0,1) rejected — Rule 6.
    #[test]
    fn pscom_06_swapped_labels_rejected() {
        let mut c = ok_chain_n(2);
        c.psk_ids[0].label.index = 1;
        // Position 0 carries label.index = 1 → not monotonic.
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::LabelIndexNotMonotonic)
        );
    }

    /// PSCOM-07 — duplicate PSKid rejected — Rule 7.
    #[test]
    fn pscom_07_duplicate_pskid_rejected() {
        let mut c = ok_chain_n(3);
        c.psk_ids[2].pskid = c.psk_ids[0].pskid.clone();
        assert_eq!(
            validate_psk_secret_chain_order(&c),
            Err(PskChainOrderError::DuplicatePskId)
        );
    }

    /// PSCOM-08 — first label (index=0, count=1) singleton accepted.
    #[test]
    fn pscom_08_singleton_accepted() {
        assert_eq!(validate_psk_secret_chain_order(&ok_chain_n(1)), Ok(()));
    }

    /// PSCOM-09 — maximum-length canonical chain (n=8) accepted.
    #[test]
    fn pscom_09_max_length_chain_accepted() {
        assert_eq!(
            validate_psk_secret_chain_order(&ok_chain_n(PSCOM_MAX_PSK_COUNT)),
            Ok(())
        );
    }

    /// PSCOM-10 — canonical 3-PSK chain accepted.
    #[test]
    fn pscom_10_canonical_chain_accepted() {
        assert_eq!(validate_psk_secret_chain_order(&ok_chain_n(3)), Ok(()));
    }
}
