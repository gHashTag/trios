//! # CR-CHAT-04 — Padding key derivation uniqueness guard (Wave-85 Lane A)
//!
//! PADDING — each padding operation must use a unique derived key, R-CHAT-4.
//!
//! Padding keys are derived from the ratchet chain to ensure
//! indistinguishability from random. If keys are reused:
//!
//! * **Cross-message correlation** — same padding key applied to two
//!   messages of different lengths reveals which parts are payload
//!   vs padding through XOR analysis.
//! * **Key reuse detection** — an observer detecting identical padding
//!   patterns across messages can group them by sender.
//! * **Deterministic padding** — reusing keys makes padding predictable,
//!   enabling an attacker to strip it and recover exact payload sizes.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate padding keys.
//! 2. Key derivation index must be strictly increasing.
//! 3. Key length must be exactly `PKDU_KEY_LEN`.
//! 4. Total derivations <= `PKDU_MAX_DERIVATIONS`.
//! 5. Derivation label must be non-empty.
//! 6. Key material must not be all zeros.
//!
//! Tests **PKDU-01..10**. Error enum [`PadKeyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-KEY-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Expected padding key length.
pub const PKDU_KEY_LEN: usize = 32;

/// Maximum derivations per session.
pub const PKDU_MAX_DERIVATIONS: usize = 65536;

/// A padding key derivation record.
#[derive(Debug, Clone)]
pub struct PadKeyDerivation {
    /// Derivation index.
    pub index: u64,
    /// Derived key material.
    pub key: Vec<u8>,
    /// Label used in derivation.
    pub label: String,
}

/// All ways padding key validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PadKeyError {
    /// Duplicate key material.
    DuplicateKey,
    /// Index not increasing.
    IndexNotIncreasing(u64),
    /// Wrong key length.
    WrongKeyLen { expected: usize, got: usize },
    /// Too many derivations.
    TooManyDerivations,
    /// Empty label.
    EmptyLabel,
    /// All-zero key.
    ZeroKey(u64),
}

/// `[VERIFIED]` Validate padding key derivation uniqueness.
pub fn validate_pad_key_derivations(
    derivations: &[PadKeyDerivation],
) -> Result<(), PadKeyError> {
    if derivations.len() > PKDU_MAX_DERIVATIONS {
        return Err(PadKeyError::TooManyDerivations);
    }
    let mut seen_keys = BTreeSet::new();
    for (i, d) in derivations.iter().enumerate() {
        if d.label.is_empty() {
            return Err(PadKeyError::EmptyLabel);
        }
        if d.key.len() != PKDU_KEY_LEN {
            return Err(PadKeyError::WrongKeyLen {
                expected: PKDU_KEY_LEN,
                got: d.key.len(),
            });
        }
        if d.key.iter().all(|&b| b == 0) {
            return Err(PadKeyError::ZeroKey(d.index));
        }
        if i > 0 && d.index <= derivations[i - 1].index {
            return Err(PadKeyError::IndexNotIncreasing(d.index));
        }
        let key_arr: [u8; 32] = d.key.clone().try_into().unwrap_or([0u8; 32]);
        if !seen_keys.insert(key_arr) {
            return Err(PadKeyError::DuplicateKey);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; PKDU_KEY_LEN]
    }

    fn derivation(index: u64, key_byte: u8) -> PadKeyDerivation {
        PadKeyDerivation {
            index,
            key: key(key_byte),
            label: "pad-v1".to_string(),
        }
    }

    fn valid_derivations() -> Vec<PadKeyDerivation> {
        vec![derivation(1, 0xAA), derivation(2, 0xBB), derivation(3, 0xCC)]
    }

    /// **PKDU-01** — duplicate key rejected.
    #[test]
    fn pkdu_01_duplicate_rejected() {
        let ds = vec![derivation(1, 0xAA), derivation(2, 0xAA)];
        assert_eq!(validate_pad_key_derivations(&ds), Err(PadKeyError::DuplicateKey));
    }

    /// **PKDU-02** — index not increasing rejected.
    #[test]
    fn pkdu_02_index_not_increasing_rejected() {
        let ds = vec![derivation(2, 0xAA), derivation(1, 0xBB)];
        assert_eq!(
            validate_pad_key_derivations(&ds),
            Err(PadKeyError::IndexNotIncreasing(1))
        );
    }

    /// **PKDU-03** — wrong key length rejected.
    #[test]
    fn pkdu_03_wrong_key_len_rejected() {
        let d = PadKeyDerivation {
            index: 1,
            key: vec![0xAA; 16],
            label: "pad-v1".to_string(),
        };
        assert_eq!(
            validate_pad_key_derivations(&[d]),
            Err(PadKeyError::WrongKeyLen { expected: 32, got: 16 })
        );
    }

    /// **PKDU-04** — too many derivations rejected.
    #[test]
    fn pkdu_04_too_many_rejected() {
        let ds: Vec<PadKeyDerivation> = (0..=PKDU_MAX_DERIVATIONS as u64)
            .map(|i| {
                let b = ((i % 254) + 1) as u8;
                PadKeyDerivation { index: i, key: vec![b; PKDU_KEY_LEN], label: "x".to_string() }
            })
            .collect();
        assert_eq!(validate_pad_key_derivations(&ds), Err(PadKeyError::TooManyDerivations));
    }

    /// **PKDU-05** — empty label rejected.
    #[test]
    fn pkdu_05_empty_label_rejected() {
        let d = PadKeyDerivation { index: 1, key: key(0xAA), label: String::new() };
        assert_eq!(validate_pad_key_derivations(&[d]), Err(PadKeyError::EmptyLabel));
    }

    /// **PKDU-06** — all-zero key rejected.
    #[test]
    fn pkdu_06_zero_key_rejected() {
        let d = PadKeyDerivation { index: 1, key: key(0x00), label: "pad".to_string() };
        assert_eq!(
            validate_pad_key_derivations(&[d]),
            Err(PadKeyError::ZeroKey(1))
        );
    }

    /// **PKDU-07** — valid derivations accepted.
    #[test]
    fn pkdu_07_valid_accepted() {
        assert_eq!(validate_pad_key_derivations(&valid_derivations()), Ok(()));
    }

    /// **PKDU-08** — empty accepted.
    #[test]
    fn pkdu_08_empty_accepted() {
        assert_eq!(validate_pad_key_derivations(&[]), Ok(()));
    }

    /// **PKDU-09** — single derivation accepted.
    #[test]
    fn pkdu_09_single_accepted() {
        assert_eq!(validate_pad_key_derivations(&[derivation(1, 0xFF)]), Ok(()));
    }

    /// **PKDU-10** — max derivations boundary accepted.
    #[test]
    fn pkdu_10_max_boundary_accepted() {
        let ds: Vec<PadKeyDerivation> = (0..PKDU_MAX_DERIVATIONS as u64)
            .map(|i| {
                let mut k = vec![0u8; PKDU_KEY_LEN];
                let bytes = i.to_le_bytes();
                k[..8].copy_from_slice(&bytes);
                k[8] = 0x01;
                PadKeyDerivation { index: i, key: k, label: "x".to_string() }
            })
            .collect();
        assert_eq!(validate_pad_key_derivations(&ds), Ok(()));
    }
}
