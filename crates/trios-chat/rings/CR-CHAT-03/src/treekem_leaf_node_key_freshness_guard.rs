//! # CR-CHAT-03 — TreeKEM leaf node key freshness guard (Wave-122 Lane B)
//!
//! RATCHET TREE — leaf node keys must have been generated within a
//! maximum age window; stale leaf keys weaken the tree's forward
//! secrecy properties.
//!
//! Each leaf in the TreeKEM ratchet tree holds a key pair. If a leaf
//! key is not refreshed within a bounded time:
//!
//! * **Forward secrecy degradation** — the longer a leaf key is used,
//!   the more messages are protected by it; compromise reveals more.
//! * **Compounding risk** — stale leaf keys mean more derived keys
//!   (chain keys, message keys) depend on the same material.
//! * **Compliance violation** — many protocols mandate periodic key
//!   refresh (e.g., MLS recommends leaf key updates every N messages).
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Key age <= `TLNF_MAX_AGE_MS`.
//! 2. Leaf index must be unique.
//! 3. Key hash must not be zero.
//! 4. Key hash must not be duplicated across leaves.
//! 5. Generated-at timestamp must be > 0.
//! 6. Total leaves <= `TLNF_MAX_LEAVES`.
//!
//! Tests **TLNF-01..10**. Error enum [`LeafFreshnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LEAF-FRESH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum key age in milliseconds.
pub const TLNF_MAX_AGE_MS: u64 = 24 * 3600 * 1000;

/// Maximum leaves per batch.
pub const TLNF_MAX_LEAVES: usize = 1024;

/// Key hash length.
pub const TLNF_HASH_LEN: usize = 32;

/// A leaf node key freshness record.
#[derive(Debug, Clone)]
pub struct LeafKeyRecord {
    /// Leaf index in the tree.
    pub leaf_index: u32,
    /// Hash of the leaf's public key.
    pub key_hash: [u8; TLNF_HASH_LEN],
    /// Timestamp when the key was generated.
    pub generated_at_ms: u64,
    /// Current time for age calculation.
    pub now_ms: u64,
}

/// All ways leaf freshness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeafFreshnessError {
    /// Key too old.
    TooOld { idx: usize, age_ms: u64, max: u64 },
    /// Duplicate leaf index.
    DuplicateIndex { idx: usize, leaf_index: u32 },
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Duplicate key hash.
    DuplicateKeyHash { idx: usize },
    /// Zero generated-at timestamp.
    ZeroTimestamp(usize),
    /// Too many leaves.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM leaf node key freshness.
pub fn validate_leaf_freshness(
    leaves: &[LeafKeyRecord],
) -> Result<(), LeafFreshnessError> {
    if leaves.len() > TLNF_MAX_LEAVES {
        return Err(LeafFreshnessError::TooMany {
            got: leaves.len(),
            max: TLNF_MAX_LEAVES,
        });
    }
    let mut seen_indices: BTreeSet<u32> = BTreeSet::new();
    let mut seen_hashes: BTreeSet<[u8; TLNF_HASH_LEN]> = BTreeSet::new();
    for (i, l) in leaves.iter().enumerate() {
        if l.generated_at_ms == 0 {
            return Err(LeafFreshnessError::ZeroTimestamp(i));
        }
        if l.key_hash == [0u8; TLNF_HASH_LEN] {
            return Err(LeafFreshnessError::ZeroKeyHash(i));
        }
        if !seen_indices.insert(l.leaf_index) {
            return Err(LeafFreshnessError::DuplicateIndex {
                idx: i,
                leaf_index: l.leaf_index,
            });
        }
        if !seen_hashes.insert(l.key_hash) {
            return Err(LeafFreshnessError::DuplicateKeyHash { idx: i });
        }
        if l.now_ms >= l.generated_at_ms {
            let age = l.now_ms - l.generated_at_ms;
            if age > TLNF_MAX_AGE_MS {
                return Err(LeafFreshnessError::TooOld {
                    idx: i,
                    age_ms: age,
                    max: TLNF_MAX_AGE_MS,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; TLNF_HASH_LEN] {
        [byte; TLNF_HASH_LEN]
    }

    fn leaf(idx: u32, key: u8, generated: u64, now: u64) -> LeafKeyRecord {
        LeafKeyRecord { leaf_index: idx, key_hash: hash(key), generated_at_ms: generated, now_ms: now }
    }

    fn valid_leaves() -> Vec<LeafKeyRecord> {
        vec![
            leaf(0, 0xA1, 1_000_000, 1_000_001),
            leaf(1, 0xA2, 1_000_000, 1_000_001),
            leaf(2, 0xA3, 1_000_000, 1_000_001),
        ]
    }

    /// **TLNF-01** — too old rejected.
    #[test]
    fn tlnf_01_too_old_rejected() {
        let ls = vec![leaf(0, 0xAA, 100, 100 + TLNF_MAX_AGE_MS + 1)];
        assert_eq!(
            validate_leaf_freshness(&ls),
            Err(LeafFreshnessError::TooOld {
                idx: 0,
                age_ms: TLNF_MAX_AGE_MS + 1,
                max: TLNF_MAX_AGE_MS,
            })
        );
    }

    /// **TLNF-02** — duplicate index rejected.
    #[test]
    fn tlnf_02_duplicate_index_rejected() {
        let ls = vec![
            leaf(0, 0xA1, 1_000_000, 1_000_001),
            leaf(0, 0xA2, 1_000_000, 1_000_001),
        ];
        assert_eq!(
            validate_leaf_freshness(&ls),
            Err(LeafFreshnessError::DuplicateIndex { idx: 1, leaf_index: 0 })
        );
    }

    /// **TLNF-03** — zero key hash rejected.
    #[test]
    fn tlnf_03_zero_key_rejected() {
        let l = LeafKeyRecord { leaf_index: 0, key_hash: [0u8; TLNF_HASH_LEN], generated_at_ms: 1_000_000, now_ms: 1_000_001 };
        assert_eq!(
            validate_leaf_freshness(&[l]),
            Err(LeafFreshnessError::ZeroKeyHash(0))
        );
    }

    /// **TLNF-04** — duplicate key hash rejected.
    #[test]
    fn tlnf_04_duplicate_hash_rejected() {
        let ls = vec![
            leaf(0, 0xAA, 1_000_000, 1_000_001),
            leaf(1, 0xAA, 1_000_000, 1_000_001),
        ];
        assert_eq!(
            validate_leaf_freshness(&ls),
            Err(LeafFreshnessError::DuplicateKeyHash { idx: 1 })
        );
    }

    /// **TLNF-05** — zero timestamp rejected.
    #[test]
    fn tlnf_05_zero_timestamp_rejected() {
        let l = LeafKeyRecord { leaf_index: 0, key_hash: hash(0xAA), generated_at_ms: 0, now_ms: 1_000 };
        assert_eq!(
            validate_leaf_freshness(&[l]),
            Err(LeafFreshnessError::ZeroTimestamp(0))
        );
    }

    /// **TLNF-06** — too many rejected.
    #[test]
    fn tlnf_06_too_many_rejected() {
        let ls: Vec<LeafKeyRecord> = (0..=TLNF_MAX_LEAVES)
            .map(|i| {
                let mut h = [0u8; TLNF_HASH_LEN];
                let val = (i as u64) + 1;
                h[0..8].copy_from_slice(&val.to_be_bytes());
                LeafKeyRecord { leaf_index: i as u32, key_hash: h, generated_at_ms: 1_000_000, now_ms: 1_000_001 }
            })
            .collect();
        assert_eq!(
            validate_leaf_freshness(&ls),
            Err(LeafFreshnessError::TooMany {
                got: TLNF_MAX_LEAVES + 1,
                max: TLNF_MAX_LEAVES,
            })
        );
    }

    /// **TLNF-07** — valid accepted.
    #[test]
    fn tlnf_07_valid_accepted() {
        assert_eq!(validate_leaf_freshness(&valid_leaves()), Ok(()));
    }

    /// **TLNF-08** — empty accepted.
    #[test]
    fn tlnf_08_empty_accepted() {
        assert_eq!(validate_leaf_freshness(&[]), Ok(()));
    }

    /// **TLNF-09** — boundary age accepted.
    #[test]
    fn tlnf_09_boundary_accepted() {
        let ls = vec![leaf(0, 0xAA, 100, 100 + TLNF_MAX_AGE_MS)];
        assert_eq!(validate_leaf_freshness(&ls), Ok(()));
    }

    /// **TLNF-10** — future key accepted (now < generated).
    #[test]
    fn tlnf_10_future_key_accepted() {
        let ls = vec![leaf(0, 0xAA, 2_000_000, 1_000_000)];
        assert_eq!(validate_leaf_freshness(&ls), Ok(()));
    }
}
