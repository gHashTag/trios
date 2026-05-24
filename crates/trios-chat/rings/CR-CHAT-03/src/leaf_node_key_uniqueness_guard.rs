//! # CR-CHAT-03 — Leaf node key uniqueness guard (Wave-87 Lane B)
//!
//! RATCHET TREE — no two leaf nodes may share the same public key,
//! R-CHAT-3.
//!
//! In TreeKEM, each leaf has a distinct public key. If two leaves share
//! a key:
//!
//! * **Key reuse detection** — an observer identifies that two members
//!   use the same key, revealing a relationship (e.g. same person on
//!   two devices using a copied key).
//! * **Impersonation** — if leaf A and leaf B share a key, messages
//!   signed under that key are ambiguous, enabling impersonation.
//! * **TreeKEM path confusion** — TreeKEM path secrets derived from
//!   the shared parent node produce identical key material for both
//!   leaves, breaking the pairwise secrecy guarantee.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate public keys across leaves.
//! 2. Leaf indices must be unique.
//! 3. Public key must not be all zeros.
//! 4. Leaf count <= `LNKU_MAX_LEAVES`.
//! 5. Leaf indices must be < `LNKU_MAX_LEAVES`.
//! 6. Public key length must be `LNKU_KEY_LEN`.
//!
//! Tests **LNKU-01..10**. Error enum [`LeafKeyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * LEAF-KEY-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum leaves in the tree.
pub const LNKU_MAX_LEAVES: u32 = 1024;

/// Public key length.
pub const LNKU_KEY_LEN: usize = 32;

/// A leaf node entry.
#[derive(Debug, Clone)]
pub struct LeafNode {
    /// Leaf index.
    pub index: u32,
    /// Public key.
    pub public_key: [u8; LNKU_KEY_LEN],
}

/// All ways leaf key validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeafKeyError {
    /// Duplicate public key.
    DuplicateKey { index_a: u32, index_b: u32 },
    /// Duplicate leaf index.
    DuplicateIndex(u32),
    /// Zero key.
    ZeroKey(u32),
    /// Too many leaves.
    TooManyLeaves,
    /// Index out of range.
    IndexOutOfRange(u32),
}

/// `[VERIFIED]` Validate leaf node key uniqueness.
pub fn validate_leaf_key_uniqueness(
    leaves: &[LeafNode],
) -> Result<(), LeafKeyError> {
    if leaves.len() > LNKU_MAX_LEAVES as usize {
        return Err(LeafKeyError::TooManyLeaves);
    }
    let mut seen_keys: BTreeSet<[u8; LNKU_KEY_LEN]> = BTreeSet::new();
    let mut seen_indices = BTreeSet::new();
    let mut key_first_index: std::collections::HashMap<[u8; LNKU_KEY_LEN], u32> =
        std::collections::HashMap::new();
    for l in leaves {
        if l.index >= LNKU_MAX_LEAVES {
            return Err(LeafKeyError::IndexOutOfRange(l.index));
        }
        if l.public_key == [0u8; LNKU_KEY_LEN] {
            return Err(LeafKeyError::ZeroKey(l.index));
        }
        if !seen_indices.insert(l.index) {
            return Err(LeafKeyError::DuplicateIndex(l.index));
        }
        if !seen_keys.insert(l.public_key) {
            let first = key_first_index.get(&l.public_key).copied().unwrap_or(l.index);
            return Err(LeafKeyError::DuplicateKey {
                index_a: first,
                index_b: l.index,
            });
        }
        key_first_index.insert(l.public_key, l.index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; LNKU_KEY_LEN] {
        [byte; LNKU_KEY_LEN]
    }

    fn leaf(index: u32, key_byte: u8) -> LeafNode {
        LeafNode { index, public_key: key(key_byte) }
    }

    fn valid_leaves() -> Vec<LeafNode> {
        vec![leaf(0, 0xAA), leaf(1, 0xBB), leaf(2, 0xCC)]
    }

    /// **LNKU-01** — duplicate key rejected.
    #[test]
    fn lnku_01_duplicate_key_rejected() {
        let ls = vec![leaf(0, 0xAA), leaf(1, 0xAA)];
        assert_eq!(
            validate_leaf_key_uniqueness(&ls),
            Err(LeafKeyError::DuplicateKey { index_a: 0, index_b: 1 })
        );
    }

    /// **LNKU-02** — duplicate index rejected.
    #[test]
    fn lnku_02_duplicate_index_rejected() {
        let ls = vec![leaf(0, 0xAA), leaf(0, 0xBB)];
        assert_eq!(
            validate_leaf_key_uniqueness(&ls),
            Err(LeafKeyError::DuplicateIndex(0))
        );
    }

    /// **LNKU-03** — zero key rejected.
    #[test]
    fn lnku_03_zero_key_rejected() {
        let l = LeafNode { index: 0, public_key: [0u8; LNKU_KEY_LEN] };
        assert_eq!(
            validate_leaf_key_uniqueness(&[l]),
            Err(LeafKeyError::ZeroKey(0))
        );
    }

    /// **LNKU-04** — too many leaves rejected.
    #[test]
    fn lnku_04_too_many_rejected() {
        let ls: Vec<LeafNode> = (0..=LNKU_MAX_LEAVES)
            .map(|i| {
                let mut k = [0u8; LNKU_KEY_LEN];
                let bytes = i.to_le_bytes();
                k[..4].copy_from_slice(&bytes);
                k[4] = 0x01;
                LeafNode { index: i, public_key: k }
            })
            .collect();
        assert_eq!(
            validate_leaf_key_uniqueness(&ls),
            Err(LeafKeyError::TooManyLeaves)
        );
    }

    /// **LNKU-05** — index out of range rejected.
    #[test]
    fn lnku_05_index_out_of_range_rejected() {
        let l = LeafNode { index: LNKU_MAX_LEAVES, public_key: key(0xAA) };
        assert_eq!(
            validate_leaf_key_uniqueness(&[l]),
            Err(LeafKeyError::IndexOutOfRange(LNKU_MAX_LEAVES))
        );
    }

    /// **LNKU-06** — valid leaves accepted.
    #[test]
    fn lnku_06_valid_accepted() {
        assert_eq!(validate_leaf_key_uniqueness(&valid_leaves()), Ok(()));
    }

    /// **LNKU-07** — empty accepted.
    #[test]
    fn lnku_07_empty_accepted() {
        assert_eq!(validate_leaf_key_uniqueness(&[]), Ok(()));
    }

    /// **LNKU-08** — single leaf accepted.
    #[test]
    fn lnku_08_single_accepted() {
        assert_eq!(validate_leaf_key_uniqueness(&[leaf(0, 0xFF)]), Ok(()));
    }

    /// **LNKU-09** — max leaves boundary accepted.
    #[test]
    fn lnku_09_max_boundary_accepted() {
        let ls: Vec<LeafNode> = (0..LNKU_MAX_LEAVES)
            .map(|i| {
                let mut k = [0u8; LNKU_KEY_LEN];
                let bytes = i.to_le_bytes();
                k[..4].copy_from_slice(&bytes);
                k[4] = 0x01;
                LeafNode { index: i, public_key: k }
            })
            .collect();
        assert_eq!(validate_leaf_key_uniqueness(&ls), Ok(()));
    }

    /// **LNKU-10** — different keys different indices accepted.
    #[test]
    fn lnku_10_unique_keys_accepted() {
        let ls = vec![leaf(0, 0x01), leaf(1, 0x02), leaf(2, 0x03), leaf(3, 0x04)];
        assert_eq!(validate_leaf_key_uniqueness(&ls), Ok(()));
    }
}
