//! # CR-CHAT-03 — TreeKEM path secret uniqueness guard (Wave-91 Lane B)
//!
//! RATCHET TREE — path secrets must be unique per node, R-CHAT-3.
//!
//! In a TreeKEM UpdatePath, each node along the direct path receives a
//! fresh path secret. If secrets are reused:
//!
//! * **Cross-node derivation** — two nodes derive the same HPKE key
//!   pair, allowing one node to decrypt messages intended for the other.
//! * **Path secret inference** — an observer who knows one node's
//!   secret can derive all nodes sharing that secret.
//! * **Compromise amplification** — compromising one path secret
//!   compromises all nodes that share it, violating the pairwise
//!   secrecy guarantee.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate path secrets.
//! 2. Node indices must be unique.
//! 3. Path secret must not be all zeros.
//! 4. Total nodes <= `TPSU_MAX_NODES`.
//! 5. Node index must be < `TPSU_MAX_NODES`.
//! 6. Path secret length must be `TPSU_SECRET_LEN`.
//!
//! Tests **TPSU-01..10**. Error enum [`PathSecretUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-SECRET-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum nodes.
pub const TPSU_MAX_NODES: u64 = 1024;

/// Path secret length.
pub const TPSU_SECRET_LEN: usize = 32;

/// A node with its path secret.
#[derive(Debug, Clone)]
pub struct PathSecretNode {
    /// Node index in the tree.
    pub node_index: u64,
    /// Path secret for this node.
    pub secret: [u8; TPSU_SECRET_LEN],
}

/// All ways path secret uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSecretUniquenessError {
    /// Duplicate path secret.
    DuplicateSecret { node_a: u64, node_b: u64 },
    /// Duplicate node index.
    DuplicateIndex(u64),
    /// Zero secret.
    ZeroSecret(u64),
    /// Too many nodes.
    TooManyNodes,
    /// Index out of range.
    IndexOutOfRange(u64),
}

/// `[VERIFIED]` Validate TreeKEM path secret uniqueness.
pub fn validate_path_secret_uniqueness(
    nodes: &[PathSecretNode],
) -> Result<(), PathSecretUniquenessError> {
    if nodes.len() > TPSU_MAX_NODES as usize {
        return Err(PathSecretUniquenessError::TooManyNodes);
    }
    let mut seen_secrets = BTreeSet::new();
    let mut seen_indices = BTreeSet::new();
    let mut secret_first_node: std::collections::HashMap<[u8; TPSU_SECRET_LEN], u64> =
        std::collections::HashMap::new();
    for n in nodes {
        if n.node_index >= TPSU_MAX_NODES {
            return Err(PathSecretUniquenessError::IndexOutOfRange(n.node_index));
        }
        if n.secret == [0u8; TPSU_SECRET_LEN] {
            return Err(PathSecretUniquenessError::ZeroSecret(n.node_index));
        }
        if !seen_indices.insert(n.node_index) {
            return Err(PathSecretUniquenessError::DuplicateIndex(n.node_index));
        }
        if !seen_secrets.insert(n.secret) {
            let first = secret_first_node.get(&n.secret).copied().unwrap_or(n.node_index);
            return Err(PathSecretUniquenessError::DuplicateSecret {
                node_a: first,
                node_b: n.node_index,
            });
        }
        secret_first_node.insert(n.secret, n.node_index);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(byte: u8) -> [u8; TPSU_SECRET_LEN] {
        [byte; TPSU_SECRET_LEN]
    }

    fn node(index: u64, secret_byte: u8) -> PathSecretNode {
        PathSecretNode { node_index: index, secret: secret(secret_byte) }
    }

    fn valid_nodes() -> Vec<PathSecretNode> {
        vec![node(0, 0xAA), node(1, 0xBB), node(2, 0xCC)]
    }

    /// **TPSU-01** — duplicate secret rejected.
    #[test]
    fn tpsu_01_duplicate_secret_rejected() {
        let ns = vec![node(0, 0xAA), node(1, 0xAA)];
        assert_eq!(
            validate_path_secret_uniqueness(&ns),
            Err(PathSecretUniquenessError::DuplicateSecret { node_a: 0, node_b: 1 })
        );
    }

    /// **TPSU-02** — duplicate index rejected.
    #[test]
    fn tpsu_02_duplicate_index_rejected() {
        let ns = vec![node(0, 0xAA), node(0, 0xBB)];
        assert_eq!(
            validate_path_secret_uniqueness(&ns),
            Err(PathSecretUniquenessError::DuplicateIndex(0))
        );
    }

    /// **TPSU-03** — zero secret rejected.
    #[test]
    fn tpsu_03_zero_secret_rejected() {
        let n = PathSecretNode { node_index: 0, secret: [0u8; TPSU_SECRET_LEN] };
        assert_eq!(
            validate_path_secret_uniqueness(&[n]),
            Err(PathSecretUniquenessError::ZeroSecret(0))
        );
    }

    /// **TPSU-04** — too many nodes rejected.
    #[test]
    fn tpsu_04_too_many_rejected() {
        let ns: Vec<PathSecretNode> = (0..=TPSU_MAX_NODES)
            .map(|i| {
                let mut s = [0u8; TPSU_SECRET_LEN];
                let bytes = i.to_le_bytes();
                s[..8].copy_from_slice(&bytes);
                s[8] = 0x01;
                PathSecretNode { node_index: i, secret: s }
            })
            .collect();
        assert_eq!(
            validate_path_secret_uniqueness(&ns),
            Err(PathSecretUniquenessError::TooManyNodes)
        );
    }

    /// **TPSU-05** — index out of range rejected.
    #[test]
    fn tpsu_05_index_out_of_range_rejected() {
        let n = PathSecretNode { node_index: TPSU_MAX_NODES, secret: secret(0xAA) };
        assert_eq!(
            validate_path_secret_uniqueness(&[n]),
            Err(PathSecretUniquenessError::IndexOutOfRange(TPSU_MAX_NODES))
        );
    }

    /// **TPSU-06** — valid nodes accepted.
    #[test]
    fn tpsu_06_valid_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&valid_nodes()), Ok(()));
    }

    /// **TPSU-07** — empty accepted.
    #[test]
    fn tpsu_07_empty_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&[]), Ok(()));
    }

    /// **TPSU-08** — single node accepted.
    #[test]
    fn tpsu_08_single_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&[node(0, 0xFF)]), Ok(()));
    }

    /// **TPSU-09** — max nodes boundary accepted.
    #[test]
    fn tpsu_09_max_boundary_accepted() {
        let ns: Vec<PathSecretNode> = (0..TPSU_MAX_NODES)
            .map(|i| {
                let mut s = [0u8; TPSU_SECRET_LEN];
                let bytes = i.to_le_bytes();
                s[..8].copy_from_slice(&bytes);
                s[8] = 0x01;
                PathSecretNode { node_index: i, secret: s }
            })
            .collect();
        assert_eq!(validate_path_secret_uniqueness(&ns), Ok(()));
    }

    /// **TPSU-10** — unique secrets unique indices accepted.
    #[test]
    fn tpsu_10_unique_accepted() {
        let ns = vec![node(0, 0x01), node(1, 0x02), node(2, 0x03), node(3, 0x04)];
        assert_eq!(validate_path_secret_uniqueness(&ns), Ok(()));
    }
}
