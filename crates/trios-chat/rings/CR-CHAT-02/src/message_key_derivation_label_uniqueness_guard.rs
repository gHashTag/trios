//! # CR-CHAT-02 — Message key derivation label uniqueness guard (Wave-130 Lane B)
//!
//! RATCHET — message keys must be derived with unique labels; reusing
//! labels produces identical keys from the same chain key.
//!
//! The Double Ratchet derives message keys using a KDF with a label.
//! If two derivations use the same label with the same chain key:
//!
//! * **Key collision** — identical (chain_key, label) pairs produce
//!   identical message keys, breaking key uniqueness.
//! * **Replay enabling** — identical message keys allow message
//!   replay to go undetected by key-based replay guards.
//! * **KDF misuse** — the KDF is only secure when each input is
//!   unique; label reuse violates this assumption.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate (chain_id, label) pairs.
//! 2. Label must not be empty.
//! 3. Chain ID must not be zero.
//! 4. Key hash must not be zero.
//! 5. Label must be <= `MKDL_MAX_LABEL_LEN`.
//! 6. Total derivations <= `MKDL_MAX_DERIVATIONS`.
//!
//! Tests **MKDL-01..10**. Error enum [`LabelUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LABEL-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum label length.
pub const MKDL_MAX_LABEL_LEN: usize = 128;

/// Maximum derivations per batch.
pub const MKDL_MAX_DERIVATIONS: usize = 1024;

/// Chain ID length.
pub const MKDL_CHAIN_ID_LEN: usize = 32;

/// Key hash length.
pub const MKDL_HASH_LEN: usize = 32;

/// A message key derivation record.
#[derive(Debug, Clone)]
pub struct DerivationLabel {
    /// Chain identifier.
    pub chain_id: [u8; MKDL_CHAIN_ID_LEN],
    /// Derivation label.
    pub label: String,
    /// Hash of the derived key.
    pub key_hash: [u8; MKDL_HASH_LEN],
}

/// All ways label uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LabelUniquenessError {
    /// Duplicate (chain_id, label) pair.
    DuplicateLabel { idx: usize },
    /// Empty label.
    EmptyLabel(usize),
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Label too long.
    LabelTooLong { idx: usize, got: usize, max: usize },
    /// Too many derivations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate message key derivation label uniqueness.
pub fn validate_label_uniqueness(
    derivations: &[DerivationLabel],
) -> Result<(), LabelUniquenessError> {
    if derivations.len() > MKDL_MAX_DERIVATIONS {
        return Err(LabelUniquenessError::TooMany {
            got: derivations.len(),
            max: MKDL_MAX_DERIVATIONS,
        });
    }
    let mut seen: BTreeSet<([u8; MKDL_CHAIN_ID_LEN], String)> = BTreeSet::new();
    for (i, d) in derivations.iter().enumerate() {
        if d.chain_id == [0u8; MKDL_CHAIN_ID_LEN] {
            return Err(LabelUniquenessError::ZeroChainId(i));
        }
        if d.label.is_empty() {
            return Err(LabelUniquenessError::EmptyLabel(i));
        }
        if d.label.len() > MKDL_MAX_LABEL_LEN {
            return Err(LabelUniquenessError::LabelTooLong {
                idx: i,
                got: d.label.len(),
                max: MKDL_MAX_LABEL_LEN,
            });
        }
        if d.key_hash == [0u8; MKDL_HASH_LEN] {
            return Err(LabelUniquenessError::ZeroKeyHash(i));
        }
        let key = (d.chain_id, d.label.clone());
        if !seen.insert(key) {
            return Err(LabelUniquenessError::DuplicateLabel { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; MKDL_CHAIN_ID_LEN] {
        [byte; MKDL_CHAIN_ID_LEN]
    }

    fn khash(byte: u8) -> [u8; MKDL_HASH_LEN] {
        [byte; MKDL_HASH_LEN]
    }

    fn deriv(chain: u8, label: &str, key: u8) -> DerivationLabel {
        DerivationLabel { chain_id: cid(chain), label: label.to_string(), key_hash: khash(key) }
    }

    fn valid_batch() -> Vec<DerivationLabel> {
        vec![
            deriv(0x01, "msg-0", 0xA1),
            deriv(0x01, "msg-1", 0xA2),
            deriv(0x01, "msg-2", 0xA3),
            deriv(0x02, "msg-0", 0xA4),
        ]
    }

    /// **MKDL-01** — duplicate label rejected.
    #[test]
    fn mkdl_01_duplicate_rejected() {
        let ds = vec![
            deriv(0x01, "msg-0", 0xA1),
            deriv(0x01, "msg-0", 0xA2),
        ];
        assert_eq!(
            validate_label_uniqueness(&ds),
            Err(LabelUniquenessError::DuplicateLabel { idx: 1 })
        );
    }

    /// **MKDL-02** — empty label rejected.
    #[test]
    fn mkdl_02_empty_label_rejected() {
        let d = DerivationLabel { chain_id: cid(0x01), label: String::new(), key_hash: khash(0xAA) };
        assert_eq!(
            validate_label_uniqueness(&[d]),
            Err(LabelUniquenessError::EmptyLabel(0))
        );
    }

    /// **MKDL-03** — zero chain ID rejected.
    #[test]
    fn mkdl_03_zero_chain_rejected() {
        let d = DerivationLabel { chain_id: [0u8; MKDL_CHAIN_ID_LEN], label: "msg".to_string(), key_hash: khash(0xAA) };
        assert_eq!(
            validate_label_uniqueness(&[d]),
            Err(LabelUniquenessError::ZeroChainId(0))
        );
    }

    /// **MKDL-04** — zero key hash rejected.
    #[test]
    fn mkdl_04_zero_hash_rejected() {
        let d = DerivationLabel { chain_id: cid(0x01), label: "msg".to_string(), key_hash: [0u8; MKDL_HASH_LEN] };
        assert_eq!(
            validate_label_uniqueness(&[d]),
            Err(LabelUniquenessError::ZeroKeyHash(0))
        );
    }

    /// **MKDL-05** — label too long rejected.
    #[test]
    fn mkdl_05_label_too_long_rejected() {
        let d = DerivationLabel { chain_id: cid(0x01), label: "x".repeat(MKDL_MAX_LABEL_LEN + 1), key_hash: khash(0xAA) };
        assert_eq!(
            validate_label_uniqueness(&[d]),
            Err(LabelUniquenessError::LabelTooLong { idx: 0, got: MKDL_MAX_LABEL_LEN + 1, max: MKDL_MAX_LABEL_LEN })
        );
    }

    /// **MKDL-06** — too many rejected.
    #[test]
    fn mkdl_06_too_many_rejected() {
        let ds: Vec<DerivationLabel> = (0..=MKDL_MAX_DERIVATIONS)
            .map(|i| {
                let mut c = [0u8; MKDL_CHAIN_ID_LEN];
                let val = (i as u64) + 1;
                c[0..8].copy_from_slice(&val.to_be_bytes());
                DerivationLabel { chain_id: c, label: format!("msg-{i}"), key_hash: khash((i as u8).wrapping_add(1)) }
            })
            .collect();
        assert_eq!(
            validate_label_uniqueness(&ds),
            Err(LabelUniquenessError::TooMany {
                got: MKDL_MAX_DERIVATIONS + 1,
                max: MKDL_MAX_DERIVATIONS,
            })
        );
    }

    /// **MKDL-07** — valid accepted.
    #[test]
    fn mkdl_07_valid_accepted() {
        assert_eq!(validate_label_uniqueness(&valid_batch()), Ok(()));
    }

    /// **MKDL-08** — empty accepted.
    #[test]
    fn mkdl_08_empty_accepted() {
        assert_eq!(validate_label_uniqueness(&[]), Ok(()));
    }

    /// **MKDL-09** — same label different chains accepted.
    #[test]
    fn mkdl_09_same_label_diff_chain_accepted() {
        let ds = vec![
            deriv(0x01, "msg-0", 0xA1),
            deriv(0x02, "msg-0", 0xA2),
        ];
        assert_eq!(validate_label_uniqueness(&ds), Ok(()));
    }

    /// **MKDL-10** — boundary label length accepted.
    #[test]
    fn mkdl_10_boundary_label_accepted() {
        let d = DerivationLabel { chain_id: cid(0x01), label: "x".repeat(MKDL_MAX_LABEL_LEN), key_hash: khash(0xAA) };
        assert_eq!(validate_label_uniqueness(&[d]), Ok(()));
    }
}
