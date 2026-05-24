//! # CR-CHAT-03 — Parent hash chain validation guard (Wave-67 Lane A)
//!
//! RATCHET TREE — parent_hash chain must be continuous leaf→root, R-CHAT-2.
//!
//! Each inner node in the TreeKEM ratchet tree stores a `parent_hash`
//! binding it to its parent. If the chain is broken, an attacker can
//! splice in a rogue subtree without detection:
//!
//! * **Broken chain** — a node's `parent_hash` does not match its
//!   parent's hash, allowing a subtree swap.
//! * **Blank node in path** — a blank node breaks the chain; the
//!   tree must be resolved before hashing.
//! * **Wrong leaf start** — the leaf node's `parent_hash` does not
//!   match its direct parent.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Every non-blank inner node has a `parent_hash`.
//! 2. `parent_hash` matches the parent node's computed hash.
//! 3. No blank nodes between leaf and root.
//! 4. Chain length <= `PHCV_MAX_DEPTH`.
//! 5. Hash length == `PHCV_HASH_LEN`.
//! 6. Root has no parent_hash (it is the trust anchor).
//!
//! Tests **PHCV-01..10**. Error enum [`ParentHashError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PARENT-HASH-CHAIN`

#![forbid(unsafe_code)]

/// Expected hash length (bytes).
pub const PHCV_HASH_LEN: usize = 32;

/// Maximum tree depth (chain length).
pub const PHCV_MAX_DEPTH: usize = 64;

/// A node in the parent-hash chain.
#[derive(Debug, Clone)]
pub struct HashNode {
    /// Node hash (32 bytes).
    pub hash: Vec<u8>,
    /// Parent hash (empty for root).
    pub parent_hash: Vec<u8>,
    /// Is this node blank?
    pub blank: bool,
}

/// All ways parent-hash chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParentHashError {
    /// Chain too deep.
    ChainTooDeep,
    /// Hash length wrong.
    HashLengthWrong,
    /// Blank node in chain.
    BlankInChain,
    /// Parent hash mismatch.
    ParentHashMismatch(usize),
    /// Missing parent hash on non-root.
    MissingParentHash(usize),
    /// Root must not have parent hash.
    RootHasParentHash,
}

/// `[VERIFIED]` Validate that a parent-hash chain from leaf to root is continuous.
pub fn validate_parent_hash_chain(
    nodes: &[HashNode],
) -> Result<(), ParentHashError> {
    if nodes.is_empty() {
        return Ok(());
    }
    if nodes.len() > PHCV_MAX_DEPTH {
        return Err(ParentHashError::ChainTooDeep);
    }
    let last = nodes.len() - 1;
    for (i, node) in nodes.iter().enumerate() {
        if node.hash.len() != PHCV_HASH_LEN {
            return Err(ParentHashError::HashLengthWrong);
        }
        if node.blank {
            return Err(ParentHashError::BlankInChain);
        }
        if i == last {
            if !node.parent_hash.is_empty() {
                return Err(ParentHashError::RootHasParentHash);
            }
        } else {
            if node.parent_hash.is_empty() {
                return Err(ParentHashError::MissingParentHash(i));
            }
            if node.parent_hash.len() != PHCV_HASH_LEN {
                return Err(ParentHashError::HashLengthWrong);
            }
            let parent = &nodes[i + 1];
            if node.parent_hash != parent.hash {
                return Err(ParentHashError::ParentHashMismatch(i));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(byte: u8) -> Vec<u8> {
        vec![byte; PHCV_HASH_LEN]
    }

    fn valid_chain_3() -> Vec<HashNode> {
        vec![
            HashNode { hash: h(0x01), parent_hash: h(0x02), blank: false },
            HashNode { hash: h(0x02), parent_hash: h(0x03), blank: false },
            HashNode { hash: h(0x03), parent_hash: vec![], blank: false },
        ]
    }

    /// **PHCV-01** — parent hash mismatch rejected.
    #[test]
    fn phcv_01_mismatch_rejected() {
        let chain = vec![
            HashNode { hash: h(0x01), parent_hash: h(0xFF), blank: false },
            HashNode { hash: h(0x02), parent_hash: vec![], blank: false },
        ];
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::ParentHashMismatch(0))
        );
    }

    /// **PHCV-02** — blank node in chain rejected.
    #[test]
    fn phcv_02_blank_rejected() {
        let chain = vec![
            HashNode { hash: h(0x01), parent_hash: h(0x02), blank: false },
            HashNode { hash: h(0x02), parent_hash: vec![], blank: true },
        ];
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::BlankInChain)
        );
    }

    /// **PHCV-03** — missing parent hash on non-root rejected.
    #[test]
    fn phcv_03_missing_parent_hash_rejected() {
        let chain = vec![
            HashNode { hash: h(0x01), parent_hash: vec![], blank: false },
            HashNode { hash: h(0x02), parent_hash: vec![], blank: false },
        ];
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::MissingParentHash(0))
        );
    }

    /// **PHCV-04** — root with parent hash rejected.
    #[test]
    fn phcv_04_root_has_parent_hash_rejected() {
        let chain = vec![
            HashNode { hash: h(0x01), parent_hash: h(0x02), blank: false },
        ];
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::RootHasParentHash)
        );
    }

    /// **PHCV-05** — hash length wrong rejected.
    #[test]
    fn phcv_05_hash_len_rejected() {
        let chain = vec![
            HashNode { hash: vec![0x01; 16], parent_hash: vec![], blank: false },
        ];
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::HashLengthWrong)
        );
    }

    /// **PHCV-06** — chain too deep rejected.
    #[test]
    fn phcv_06_too_deep_rejected() {
        let chain: Vec<HashNode> = (0..=PHCV_MAX_DEPTH)
            .map(|i| HashNode {
                hash: h(i as u8),
                parent_hash: if i < PHCV_MAX_DEPTH { h((i + 1) as u8) } else { vec![] },
                blank: false,
            })
            .collect();
        assert_eq!(
            validate_parent_hash_chain(&chain),
            Err(ParentHashError::ChainTooDeep)
        );
    }

    /// **PHCV-07** — valid 3-node chain accepted.
    #[test]
    fn phcv_07_valid_3_accepted() {
        assert_eq!(validate_parent_hash_chain(&valid_chain_3()), Ok(()));
    }

    /// **PHCV-08** — single root accepted.
    #[test]
    fn phcv_08_single_root_accepted() {
        let root = HashNode { hash: h(0xAA), parent_hash: vec![], blank: false };
        assert_eq!(validate_parent_hash_chain(&[root]), Ok(()));
    }

    /// **PHCV-09** — empty chain accepted.
    #[test]
    fn phcv_09_empty_accepted() {
        assert_eq!(validate_parent_hash_chain(&[]), Ok(()));
    }

    /// **PHCV-10** — max depth chain accepted.
    #[test]
    fn phcv_10_max_depth_accepted() {
        let n = PHCV_MAX_DEPTH;
        let chain: Vec<HashNode> = (0..n)
            .map(|i| {
                let byte = ((i % 254) + 1) as u8;
                HashNode {
                    hash: h(byte),
                    parent_hash: if i < n - 1 {
                        h(((i + 1) % 254 + 1) as u8)
                    } else {
                        vec![]
                    },
                    blank: false,
                }
            })
            .collect();
        assert_eq!(validate_parent_hash_chain(&chain), Ok(()));
    }
}
