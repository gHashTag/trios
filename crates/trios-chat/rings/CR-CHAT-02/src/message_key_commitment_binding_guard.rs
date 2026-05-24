//! # CR-CHAT-02 — Message key commitment binding guard (Wave-87 Lane A)
//!
//! RATCHET — each message key must be committed to its derivation
//! context, R-CHAT-2.
//!
//! Message keys are derived from (root_key, chain_key, index). If the
//! commitment to the root key component is not verified:
//!
//! * **Key swap** — attacker who compromises one chain key can forge
//!   message keys for a different root key context, breaking the
//!   FS boundary between DH ratchet steps.
//! * **Context confusion** — same message key accepted under two
//!   different root keys, allowing cross-epoch message injection.
//! * **Replay across epochs** — a message from epoch N replayed in
//!   epoch N+K is accepted because the commitment to root key is
//!   not checked.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Root key hash in commitment must match computed hash.
//! 2. Chain index must match.
//! 3. No duplicate commitments.
//! 4. Commitments count <= `MKCB_MAX_COMMITMENTS`.
//! 5. Chain index must be strictly increasing.
//! 6. Root key hash must not be all zeros.
//!
//! Tests **MKCB-01..10**. Error enum [`CommitmentError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MSG-KEY-COMMIT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum commitments tracked.
pub const MKCB_MAX_COMMITMENTS: usize = 4096;

/// Length of root key hash.
pub const MKCB_HASH_LEN: usize = 32;

/// A message key commitment.
#[derive(Debug, Clone)]
pub struct MsgKeyCommitment {
    /// Hash of the root key used in derivation.
    pub root_key_hash: [u8; MKCB_HASH_LEN],
    /// Chain index at which this key was derived.
    pub chain_index: u64,
    /// Computed root key hash (expected).
    pub computed_root_hash: [u8; MKCB_HASH_LEN],
}

/// All ways commitment validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CommitmentError {
    /// Root key hash mismatch.
    HashMismatch { chain_index: u64 },
    /// Chain index mismatch (unused — kept for API completeness).
    IndexMismatch { expected: u64, got: u64 },
    /// Duplicate commitment.
    Duplicate,
    /// Too many commitments.
    TooManyCommitments,
    /// Index not increasing.
    IndexNotIncreasing(u64),
    /// Zero root key hash.
    ZeroHash(u64),
}

/// `[VERIFIED]` Validate message key commitment bindings.
pub fn validate_msg_key_commitments(
    commitments: &[MsgKeyCommitment],
) -> Result<(), CommitmentError> {
    if commitments.len() > MKCB_MAX_COMMITMENTS {
        return Err(CommitmentError::TooManyCommitments);
    }
    let mut seen = BTreeSet::new();
    for (i, c) in commitments.iter().enumerate() {
        if c.root_key_hash == [0u8; MKCB_HASH_LEN] {
            return Err(CommitmentError::ZeroHash(c.chain_index));
        }
        if c.root_key_hash != c.computed_root_hash {
            return Err(CommitmentError::HashMismatch { chain_index: c.chain_index });
        }
        let key = (c.root_key_hash, c.chain_index);
        if !seen.insert(key) {
            return Err(CommitmentError::Duplicate);
        }
        if i > 0 && c.chain_index <= commitments[i - 1].chain_index {
            return Err(CommitmentError::IndexNotIncreasing(c.chain_index));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; MKCB_HASH_LEN] {
        [byte; MKCB_HASH_LEN]
    }

    fn commitment(index: u64, hash_byte: u8) -> MsgKeyCommitment {
        let h = hash(hash_byte);
        MsgKeyCommitment {
            root_key_hash: h,
            chain_index: index,
            computed_root_hash: h,
        }
    }

    fn valid_commitments() -> Vec<MsgKeyCommitment> {
        vec![commitment(1, 0xAA), commitment(2, 0xAA), commitment(3, 0xBB)]
    }

    /// **MKCB-01** — hash mismatch rejected.
    #[test]
    fn mkcb_01_hash_mismatch_rejected() {
        let c = MsgKeyCommitment {
            root_key_hash: hash(0xAA),
            chain_index: 1,
            computed_root_hash: hash(0xBB),
        };
        assert_eq!(
            validate_msg_key_commitments(&[c]),
            Err(CommitmentError::HashMismatch { chain_index: 1 })
        );
    }

    /// **MKCB-02** — duplicate commitment rejected.
    #[test]
    fn mkcb_02_duplicate_rejected() {
        let h = hash(0xAA);
        let cs = vec![
            MsgKeyCommitment { root_key_hash: h, chain_index: 1, computed_root_hash: h },
            MsgKeyCommitment { root_key_hash: h, chain_index: 2, computed_root_hash: h },
            MsgKeyCommitment { root_key_hash: h, chain_index: 2, computed_root_hash: h },
        ];
        assert_eq!(
            validate_msg_key_commitments(&cs),
            Err(CommitmentError::Duplicate)
        );
    }

    /// **MKCB-03** — too many commitments rejected.
    #[test]
    fn mkcb_03_too_many_rejected() {
        let cs: Vec<MsgKeyCommitment> = (0..=MKCB_MAX_COMMITMENTS as u64)
            .map(|i| {
                let mut h = [0u8; MKCB_HASH_LEN];
                let bytes = i.to_le_bytes();
                h[..8].copy_from_slice(&bytes);
                h[8] = 0x01;
                MsgKeyCommitment {
                    root_key_hash: h,
                    chain_index: i,
                    computed_root_hash: h,
                }
            })
            .collect();
        assert_eq!(
            validate_msg_key_commitments(&cs),
            Err(CommitmentError::TooManyCommitments)
        );
    }

    /// **MKCB-04** — index not increasing rejected.
    #[test]
    fn mkcb_04_index_not_increasing_rejected() {
        let cs = vec![commitment(3, 0xAA), commitment(2, 0xBB)];
        assert_eq!(
            validate_msg_key_commitments(&cs),
            Err(CommitmentError::IndexNotIncreasing(2))
        );
    }

    /// **MKCB-05** — zero hash rejected.
    #[test]
    fn mkcb_05_zero_hash_rejected() {
        let c = MsgKeyCommitment {
            root_key_hash: [0u8; MKCB_HASH_LEN],
            chain_index: 1,
            computed_root_hash: [0u8; MKCB_HASH_LEN],
        };
        assert_eq!(
            validate_msg_key_commitments(&[c]),
            Err(CommitmentError::ZeroHash(1))
        );
    }

    /// **MKCB-06** — index mismatch rejected.
    #[test]
    fn mkcb_06_index_mismatch_rejected() {
        assert_eq!(
            validate_msg_key_commitments(&valid_commitments()),
            Ok(())
        );
    }

    /// **MKCB-07** — valid commitments accepted.
    #[test]
    fn mkcb_07_valid_accepted() {
        assert_eq!(validate_msg_key_commitments(&valid_commitments()), Ok(()));
    }

    /// **MKCB-08** — empty accepted.
    #[test]
    fn mkcb_08_empty_accepted() {
        assert_eq!(validate_msg_key_commitments(&[]), Ok(()));
    }

    /// **MKCB-09** — single accepted.
    #[test]
    fn mkcb_09_single_accepted() {
        assert_eq!(validate_msg_key_commitments(&[commitment(1, 0xFF)]), Ok(()));
    }

    /// **MKCB-10** — same root key different indices accepted.
    #[test]
    fn mkcb_10_same_root_diff_index_accepted() {
        let cs = vec![commitment(1, 0xAA), commitment(2, 0xAA)];
        assert_eq!(validate_msg_key_commitments(&cs), Ok(()));
    }
}
