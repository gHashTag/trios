//! # CR-CHAT-03 — TreeKEM parent hash chain continuity guard (Wave-137 Lane B)
//!
//! RATCHET TREE — the parent hash chain from leaf to root must be
//! continuous; gaps indicate tree tampering.
//!
//! In TreeKEM, each node carries a parent hash that links it to its
//! parent in the ratchet tree. This forms a hash chain from each leaf
//! to the root. If the chain is broken:
//!
//! * **Tree tampering** — an attacker who modifies a node's parent
//!   hash can inject a rogue public key without detection.
//! * **Path integrity** — the update path from leaf to root must be
//!   a continuous hash chain; a gap invalidates all path secrets.
//! * **Root trust** — the root hash is the trust anchor; if any
//!   intermediate link is broken, the root hash is unreliable.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each node's parent_hash must match parent's computed hash.
//! 2. Node ID must not be zero.
//! 3. No duplicate node IDs.
//! 4. Root node's parent_hash must be zero.
//! 5. Chain depth <= `TPHC_MAX_DEPTH`.
//! 6. Batch size <= `TPHC_MAX_CHAINS`.
//!
//! Tests **TPHC-01..10**. Error enum [`ParentChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-CONTINUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chain depth.
pub const TPHC_MAX_DEPTH: usize = 32;

/// Maximum chains per batch.
pub const TPHC_MAX_CHAINS: usize = 256;

/// Hash length.
pub const TPHC_HASH_LEN: usize = 32;

/// Node ID length.
pub const TPHC_NODE_ID_LEN: usize = 16;

/// A node in the parent hash chain.
#[derive(Debug, Clone)]
pub struct ParentHashNode {
    /// Node identifier.
    pub node_id: [u8; TPHC_NODE_ID_LEN],
    /// Hash value of this node.
    pub hash: [u8; TPHC_HASH_LEN],
    /// Expected parent hash (zero for root).
    pub parent_hash: [u8; TPHC_HASH_LEN],
}

/// A complete parent hash chain.
#[derive(Debug, Clone)]
pub struct ParentHashChain {
    /// Chain identifier.
    pub chain_id: [u8; TPHC_NODE_ID_LEN],
    /// Nodes from leaf to root.
    pub nodes: Vec<ParentHashNode>,
}

/// All ways parent hash chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParentChainError {
    /// Chain broken: parent_hash mismatch.
    ChainBroken {
        /// Index in chain.
        idx: usize,
    },
    /// Zero node ID.
    ZeroNodeId {
        /// Chain index.
        chain: usize,
        /// Node index.
        node: usize,
    },
    /// Duplicate node ID within chain.
    DuplicateNodeId {
        /// Chain index.
        chain: usize,
        /// Node index.
        node: usize,
    },
    /// Root parent_hash non-zero.
    RootParentNonZero {
        /// Chain index.
        chain: usize,
    },
    /// Chain too deep.
    TooDeep {
        /// Chain index.
        chain: usize,
        /// Actual depth.
        got: usize,
        /// Maximum depth.
        max: usize,
    },
    /// Too many chains.
    TooManyChains {
        /// Actual count.
        got: usize,
        /// Maximum count.
        max: usize,
    },
}

/// `[VERIFIED]` Validate TreeKEM parent hash chain continuity.
pub fn validate_parent_hash_chains(
    chains: &[ParentHashChain],
) -> Result<(), ParentChainError> {
    if chains.len() > TPHC_MAX_CHAINS {
        return Err(ParentChainError::TooManyChains {
            got: chains.len(),
            max: TPHC_MAX_CHAINS,
        });
    }
    for (ci, chain) in chains.iter().enumerate() {
        if chain.nodes.len() > TPHC_MAX_DEPTH {
            return Err(ParentChainError::TooDeep {
                chain: ci,
                got: chain.nodes.len(),
                max: TPHC_MAX_DEPTH,
            });
        }
        if chain.nodes.is_empty() {
            continue;
        }
        let mut seen: BTreeSet<[u8; TPHC_NODE_ID_LEN]> = BTreeSet::new();
        for (ni, node) in chain.nodes.iter().enumerate() {
            if node.node_id == [0u8; TPHC_NODE_ID_LEN] {
                return Err(ParentChainError::ZeroNodeId { chain: ci, node: ni });
            }
            if !seen.insert(node.node_id) {
                return Err(ParentChainError::DuplicateNodeId { chain: ci, node: ni });
            }
        }
        let last_idx = chain.nodes.len() - 1;
        if chain.nodes[last_idx].parent_hash != [0u8; TPHC_HASH_LEN] {
            return Err(ParentChainError::RootParentNonZero { chain: ci });
        }
        for i in 0..chain.nodes.len().saturating_sub(1) {
            if chain.nodes[i].parent_hash != chain.nodes[i + 1].hash {
                return Err(ParentChainError::ChainBroken { idx: i });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nid(byte: u8) -> [u8; TPHC_NODE_ID_LEN] {
        [byte; TPHC_NODE_ID_LEN]
    }

    fn h(byte: u8) -> [u8; TPHC_HASH_LEN] {
        [byte; TPHC_HASH_LEN]
    }

    fn node(id: u8, hash: u8, parent_hash: u8) -> ParentHashNode {
        ParentHashNode { node_id: nid(id), hash: h(hash), parent_hash: h(parent_hash) }
    }

    fn valid_chain() -> ParentHashChain {
        ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![
                node(0x03, 0xCC, 0xBB),
                node(0x02, 0xBB, 0xAA),
                node(0x01, 0xAA, 0x00),
            ],
        }
    }

    /// **TPHC-01** — chain broken rejected.
    #[test]
    fn tphc_01_chain_broken_rejected() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![
                node(0x03, 0xCC, 0xFF),
                node(0x02, 0xBB, 0xAA),
                node(0x01, 0xAA, 0x00),
            ],
        };
        assert_eq!(
            validate_parent_hash_chains(&[chain]),
            Err(ParentChainError::ChainBroken { idx: 0 })
        );
    }

    /// **TPHC-02** — zero node ID rejected.
    #[test]
    fn tphc_02_zero_node_id_rejected() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![ParentHashNode {
                node_id: [0u8; TPHC_NODE_ID_LEN],
                hash: h(0xAA),
                parent_hash: [0u8; TPHC_HASH_LEN],
            }],
        };
        assert_eq!(
            validate_parent_hash_chains(&[chain]),
            Err(ParentChainError::ZeroNodeId { chain: 0, node: 0 })
        );
    }

    /// **TPHC-03** — duplicate node ID rejected.
    #[test]
    fn tphc_03_duplicate_rejected() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![
                node(0x01, 0xCC, 0xBB),
                node(0x01, 0xBB, 0xAA),
                node(0x02, 0xAA, 0x00),
            ],
        };
        assert_eq!(
            validate_parent_hash_chains(&[chain]),
            Err(ParentChainError::DuplicateNodeId { chain: 0, node: 1 })
        );
    }

    /// **TPHC-04** — root parent non-zero rejected.
    #[test]
    fn tphc_04_root_parent_nonzero_rejected() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![node(0x01, 0xAA, 0xFF)],
        };
        assert_eq!(
            validate_parent_hash_chains(&[chain]),
            Err(ParentChainError::RootParentNonZero { chain: 0 })
        );
    }

    /// **TPHC-05** — chain too deep rejected.
    #[test]
    fn tphc_05_too_deep_rejected() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: (0..=TPHC_MAX_DEPTH)
                .map(|i| {
                    let mut id = [0u8; TPHC_NODE_ID_LEN];
                    let val = (i as u64) + 1;
                    id[0..8].copy_from_slice(&val.to_be_bytes());
                    ParentHashNode { node_id: id, hash: h((i % 256) as u8), parent_hash: [0u8; TPHC_HASH_LEN] }
                })
                .collect(),
        };
        assert_eq!(
            validate_parent_hash_chains(&[chain]),
            Err(ParentChainError::TooDeep { chain: 0, got: TPHC_MAX_DEPTH + 1, max: TPHC_MAX_DEPTH })
        );
    }

    /// **TPHC-06** — too many chains rejected.
    #[test]
    fn tphc_06_too_many_rejected() {
        let chains: Vec<ParentHashChain> = (0..=TPHC_MAX_CHAINS)
            .map(|i| {
                let mut id = [0u8; TPHC_NODE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                ParentHashChain { chain_id: id, nodes: vec![] }
            })
            .collect();
        assert_eq!(
            validate_parent_hash_chains(&chains),
            Err(ParentChainError::TooManyChains { got: TPHC_MAX_CHAINS + 1, max: TPHC_MAX_CHAINS })
        );
    }

    /// **TPHC-07** — valid accepted.
    #[test]
    fn tphc_07_valid_accepted() {
        assert_eq!(validate_parent_hash_chains(&[valid_chain()]), Ok(()));
    }

    /// **TPHC-08** — empty chains accepted.
    #[test]
    fn tphc_08_empty_accepted() {
        assert_eq!(validate_parent_hash_chains(&[]), Ok(()));
    }

    /// **TPHC-09** — single root node accepted.
    #[test]
    fn tphc_09_single_root_accepted() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![node(0x01, 0xAA, 0x00)],
        };
        assert_eq!(validate_parent_hash_chains(&[chain]), Ok(()));
    }

    /// **TPHC-10** — empty chain nodes accepted.
    #[test]
    fn tphc_10_empty_chain_nodes_accepted() {
        let chain = ParentHashChain {
            chain_id: nid(0x01),
            nodes: vec![],
        };
        assert_eq!(validate_parent_hash_chains(&[chain]), Ok(()));
    }
}
