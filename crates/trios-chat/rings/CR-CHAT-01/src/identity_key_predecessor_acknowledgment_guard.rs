//! # CR-CHAT-01 — Identity key predecessor acknowledgment guard (Wave-114 Lane A)
//!
//! IDENTITY — rotated keys must acknowledge their predecessor.
//!
//! When an identity key is rotated, the new key must include a signed
//! acknowledgment of the previous key. Without this:
//!
//! * **Audit trail break** — there is no cryptographic link between
//!   successive identity keys, making it impossible to verify the
//!   rotation chain.
//! * **Key injection** — an attacker can inject a new key without
//!   proving it was rotated from the legitimate previous key.
//! * **Rollback** — without acknowledgment, an old key can be
//!   reinstated without detection, breaking forward secrecy.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each rotation must reference its predecessor.
//! 2. Rotation index must be strictly increasing.
//! 3. Predecessor hash must not be zero (except first rotation).
//! 4. New key hash must not be zero.
//! 5. No duplicate new key hashes.
//! 6. Total rotations <= `IKPA_MAX_ROTATIONS`.
//!
//! Tests **IKPA-01..10**. Error enum [`PredecessorError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PREDECESSOR-ACK`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum rotations per batch.
pub const IKPA_MAX_ROTATIONS: usize = 256;

/// Hash length.
pub const IKPA_HASH_LEN: usize = 32;

/// A rotation acknowledgment record.
#[derive(Debug, Clone)]
pub struct RotationAck {
    /// Rotation index (1-based).
    pub index: u64,
    /// Hash of the new identity key.
    pub new_key_hash: [u8; IKPA_HASH_LEN],
    /// Hash of the predecessor key (zero for first rotation).
    pub predecessor_hash: [u8; IKPA_HASH_LEN],
}

/// All ways predecessor acknowledgment validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PredecessorError {
    /// Predecessor mismatch — must chain to previous new_key_hash.
    ChainBroken { idx: usize, expected: [u8; IKPA_HASH_LEN], got: [u8; IKPA_HASH_LEN] },
    /// Not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero predecessor (non-first rotation).
    ZeroPredecessor(usize),
    /// Zero new key hash.
    ZeroNewKey(usize),
    /// Duplicate new key hash.
    DuplicateNewKey(usize),
    /// Too many rotations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate identity key predecessor acknowledgment.
pub fn validate_predecessor_chain(
    rotations: &[RotationAck],
) -> Result<(), PredecessorError> {
    if rotations.len() > IKPA_MAX_ROTATIONS {
        return Err(PredecessorError::TooMany {
            got: rotations.len(),
            max: IKPA_MAX_ROTATIONS,
        });
    }
    let mut seen: BTreeSet<[u8; IKPA_HASH_LEN]> = BTreeSet::new();
    let mut prev_index: u64 = 0;
    let mut prev_new_key: Option<[u8; IKPA_HASH_LEN]> = None;
    for (i, r) in rotations.iter().enumerate() {
        if r.new_key_hash == [0u8; IKPA_HASH_LEN] {
            return Err(PredecessorError::ZeroNewKey(i));
        }
        if i > 0 && r.index <= prev_index {
            return Err(PredecessorError::NotIncreasing {
                idx: i,
                prev: prev_index,
                current: r.index,
            });
        }
        if i > 0 && r.predecessor_hash == [0u8; IKPA_HASH_LEN] {
            return Err(PredecessorError::ZeroPredecessor(i));
        }
        if let Some(expected) = prev_new_key {
            if r.predecessor_hash != expected {
                return Err(PredecessorError::ChainBroken {
                    idx: i,
                    expected,
                    got: r.predecessor_hash,
                });
            }
        }
        if !seen.insert(r.new_key_hash) {
            return Err(PredecessorError::DuplicateNewKey(i));
        }
        prev_index = r.index;
        prev_new_key = Some(r.new_key_hash);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; IKPA_HASH_LEN] {
        [byte; IKPA_HASH_LEN]
    }

    fn rotation(index: u64, new: u8, pred: u8) -> RotationAck {
        RotationAck { index, new_key_hash: hash(new), predecessor_hash: hash(pred) }
    }

    fn first_rotation() -> RotationAck {
        RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] }
    }

    fn valid_chain() -> Vec<RotationAck> {
        vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
            RotationAck { index: 2, new_key_hash: hash(0x02), predecessor_hash: hash(0x01) },
            RotationAck { index: 3, new_key_hash: hash(0x03), predecessor_hash: hash(0x02) },
        ]
    }

    /// **IKPA-01** — chain broken rejected.
    #[test]
    fn ikpa_01_chain_broken_rejected() {
        let rs = vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
            RotationAck { index: 2, new_key_hash: hash(0x03), predecessor_hash: hash(0x99) },
        ];
        assert_eq!(
            validate_predecessor_chain(&rs),
            Err(PredecessorError::ChainBroken {
                idx: 1,
                expected: hash(0x01),
                got: hash(0x99),
            })
        );
    }

    /// **IKPA-02** — not increasing rejected.
    #[test]
    fn ikpa_02_not_increasing_rejected() {
        let rs = vec![
            RotationAck { index: 5, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
            RotationAck { index: 3, new_key_hash: hash(0x02), predecessor_hash: hash(0x01) },
        ];
        assert_eq!(
            validate_predecessor_chain(&rs),
            Err(PredecessorError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **IKPA-03** — zero predecessor (non-first) rejected.
    #[test]
    fn ikpa_03_zero_predecessor_rejected() {
        let rs = vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
            RotationAck { index: 2, new_key_hash: hash(0x02), predecessor_hash: [0u8; IKPA_HASH_LEN] },
        ];
        assert_eq!(
            validate_predecessor_chain(&rs),
            Err(PredecessorError::ZeroPredecessor(1))
        );
    }

    /// **IKPA-04** — zero new key rejected.
    #[test]
    fn ikpa_04_zero_new_key_rejected() {
        let r = RotationAck { index: 1, new_key_hash: [0u8; IKPA_HASH_LEN], predecessor_hash: [0u8; IKPA_HASH_LEN] };
        assert_eq!(
            validate_predecessor_chain(&[r]),
            Err(PredecessorError::ZeroNewKey(0))
        );
    }

    /// **IKPA-05** — duplicate new key rejected.
    #[test]
    fn ikpa_05_duplicate_rejected() {
        let rs = vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
            RotationAck { index: 2, new_key_hash: hash(0x01), predecessor_hash: hash(0x01) },
        ];
        assert_eq!(
            validate_predecessor_chain(&rs),
            Err(PredecessorError::DuplicateNewKey(1))
        );
    }

    /// **IKPA-06** — too many rejected.
    #[test]
    fn ikpa_06_too_many_rejected() {
        let mut rs: Vec<RotationAck> = vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
        ];
        for i in 1..=IKPA_MAX_ROTATIONS {
            let new = (i as u8).wrapping_add(2);
            let pred = (i as u8).wrapping_add(1);
            rs.push(RotationAck { index: (i as u64) + 2, new_key_hash: hash(new), predecessor_hash: hash(pred) });
        }
        assert!(matches!(
            validate_predecessor_chain(&rs),
            Err(PredecessorError::TooMany { .. })
        ));
    }

    /// **IKPA-07** — valid chain accepted.
    #[test]
    fn ikpa_07_valid_accepted() {
        assert_eq!(validate_predecessor_chain(&valid_chain()), Ok(()));
    }

    /// **IKPA-08** — empty accepted.
    #[test]
    fn ikpa_08_empty_accepted() {
        assert_eq!(validate_predecessor_chain(&[]), Ok(()));
    }

    /// **IKPA-09** — first rotation with zero predecessor accepted.
    #[test]
    fn ikpa_09_first_rotation_accepted() {
        assert_eq!(validate_predecessor_chain(&[first_rotation()]), Ok(()));
    }

    /// **IKPA-10** — long valid chain accepted.
    #[test]
    fn ikpa_10_long_chain_accepted() {
        let mut rs = vec![
            RotationAck { index: 1, new_key_hash: hash(0x01), predecessor_hash: [0u8; IKPA_HASH_LEN] },
        ];
        for i in 1..10u8 {
            rs.push(RotationAck {
                index: (i as u64) + 1,
                new_key_hash: hash(i + 1),
                predecessor_hash: hash(i),
            });
        }
        assert_eq!(validate_predecessor_chain(&rs), Ok(()));
    }
}
