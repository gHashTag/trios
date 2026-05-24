//! # CR-CHAT-03 — TreeKEM path secret uniqueness guard (Wave-126 Lane A)
//!
//! RATCHET TREE — path secrets derived during an Update must be unique
//! across all updates; reusing path secrets breaks tree isolation.
//!
//! Each TreeKEM Update generates fresh path secrets for every node on
//! the direct path. If a path secret is reused:
//!
//! * **Tree isolation break** — two different tree nodes sharing a
//!   path secret means compromising one compromises the other.
//! * **Update linkage** — path secret reuse across updates links
//!   the updates, breaking update unlinkability.
//! * **Key derivation weakness** — the parent secret is derived from
//!   child secrets; reused child secrets produce weak parent secrets.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate path secret hashes.
//! 2. Path secret hash must not be zero.
//! 3. Update ID must not be zero.
//! 4. No duplicate update IDs.
//! 5. Node index must be <= `TPSU_MAX_NODE_INDEX`.
//! 6. Total secrets <= `TPSU_MAX_SECRETS`.
//!
//! Tests **TPSU-01..10**. Error enum [`PathSecretUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum node index.
pub const TPSU_MAX_NODE_INDEX: u32 = 65535;

/// Maximum secrets per batch.
pub const TPSU_MAX_SECRETS: usize = 4096;

/// Update ID length.
pub const TPSU_UPDATE_ID_LEN: usize = 32;

/// Path secret hash length.
pub const TPSU_HASH_LEN: usize = 32;

/// A path secret record.
#[derive(Debug, Clone)]
pub struct PathSecretRecord {
    /// Update identifier.
    pub update_id: [u8; TPSU_UPDATE_ID_LEN],
    /// Node index in the tree.
    pub node_index: u32,
    /// Hash of the path secret.
    pub secret_hash: [u8; TPSU_HASH_LEN],
}

/// All ways path secret uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSecretUniquenessError {
    /// Duplicate path secret hash.
    DuplicateSecret { idx: usize },
    /// Zero secret hash.
    ZeroSecret(usize),
    /// Zero update ID.
    ZeroUpdateId(usize),
    /// Duplicate update ID.
    DuplicateUpdateId { idx: usize },
    /// Node index too high.
    NodeIndexTooHigh { idx: usize, got: u32, max: u32 },
    /// Too many secrets.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM path secret uniqueness.
pub fn validate_path_secret_uniqueness(
    secrets: &[PathSecretRecord],
) -> Result<(), PathSecretUniquenessError> {
    if secrets.len() > TPSU_MAX_SECRETS {
        return Err(PathSecretUniquenessError::TooMany {
            got: secrets.len(),
            max: TPSU_MAX_SECRETS,
        });
    }
    let mut seen_hashes: BTreeSet<[u8; TPSU_HASH_LEN]> = BTreeSet::new();
    let mut seen_updates: BTreeSet<[u8; TPSU_UPDATE_ID_LEN]> = BTreeSet::new();
    for (i, s) in secrets.iter().enumerate() {
        if s.secret_hash == [0u8; TPSU_HASH_LEN] {
            return Err(PathSecretUniquenessError::ZeroSecret(i));
        }
        if s.update_id == [0u8; TPSU_UPDATE_ID_LEN] {
            return Err(PathSecretUniquenessError::ZeroUpdateId(i));
        }
        if s.node_index > TPSU_MAX_NODE_INDEX {
            return Err(PathSecretUniquenessError::NodeIndexTooHigh {
                idx: i,
                got: s.node_index,
                max: TPSU_MAX_NODE_INDEX,
            });
        }
        if !seen_updates.insert(s.update_id) {
            return Err(PathSecretUniquenessError::DuplicateUpdateId { idx: i });
        }
        if !seen_hashes.insert(s.secret_hash) {
            return Err(PathSecretUniquenessError::DuplicateSecret { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(byte: u8) -> [u8; TPSU_UPDATE_ID_LEN] {
        [byte; TPSU_UPDATE_ID_LEN]
    }

    fn hash(byte: u8) -> [u8; TPSU_HASH_LEN] {
        [byte; TPSU_HASH_LEN]
    }

    fn secret(update: u8, node: u32, h: u8) -> PathSecretRecord {
        PathSecretRecord { update_id: uid(update), node_index: node, secret_hash: hash(h) }
    }

    fn valid_batch() -> Vec<PathSecretRecord> {
        vec![
            secret(0x01, 0, 0xA1),
            secret(0x02, 1, 0xA2),
            secret(0x03, 0, 0xA3),
            secret(0x04, 1, 0xA4),
        ]
    }

    /// **TPSU-01** — duplicate secret rejected.
    #[test]
    fn tpsu_01_duplicate_secret_rejected() {
        let ss = vec![
            secret(0x01, 0, 0xAA),
            secret(0x02, 1, 0xAA),
        ];
        assert_eq!(
            validate_path_secret_uniqueness(&ss),
            Err(PathSecretUniquenessError::DuplicateSecret { idx: 1 })
        );
    }

    /// **TPSU-02** — zero secret rejected.
    #[test]
    fn tpsu_02_zero_secret_rejected() {
        let s = PathSecretRecord { update_id: uid(0x01), node_index: 0, secret_hash: [0u8; TPSU_HASH_LEN] };
        assert_eq!(
            validate_path_secret_uniqueness(&[s]),
            Err(PathSecretUniquenessError::ZeroSecret(0))
        );
    }

    /// **TPSU-03** — zero update ID rejected.
    #[test]
    fn tpsu_03_zero_update_rejected() {
        let s = PathSecretRecord { update_id: [0u8; TPSU_UPDATE_ID_LEN], node_index: 0, secret_hash: hash(0xAA) };
        assert_eq!(
            validate_path_secret_uniqueness(&[s]),
            Err(PathSecretUniquenessError::ZeroUpdateId(0))
        );
    }

    /// **TPSU-04** — duplicate update ID rejected.
    #[test]
    fn tpsu_04_duplicate_update_rejected() {
        let ss = vec![
            secret(0x01, 0, 0xA1),
            secret(0x01, 1, 0xA2),
        ];
        assert_eq!(
            validate_path_secret_uniqueness(&ss),
            Err(PathSecretUniquenessError::DuplicateUpdateId { idx: 1 })
        );
    }

    /// **TPSU-05** — node index too high rejected.
    #[test]
    fn tpsu_05_node_too_high_rejected() {
        let s = PathSecretRecord { update_id: uid(0x01), node_index: TPSU_MAX_NODE_INDEX + 1, secret_hash: hash(0xAA) };
        assert_eq!(
            validate_path_secret_uniqueness(&[s]),
            Err(PathSecretUniquenessError::NodeIndexTooHigh {
                idx: 0,
                got: TPSU_MAX_NODE_INDEX + 1,
                max: TPSU_MAX_NODE_INDEX,
            })
        );
    }

    /// **TPSU-06** — too many rejected.
    #[test]
    fn tpsu_06_too_many_rejected() {
        let ss: Vec<PathSecretRecord> = (0..=TPSU_MAX_SECRETS)
            .map(|i| {
                let mut u = [0u8; TPSU_UPDATE_ID_LEN];
                let val = (i as u64) + 1;
                u[0..8].copy_from_slice(&val.to_be_bytes());
                let mut h = [0u8; TPSU_HASH_LEN];
                h[0..8].copy_from_slice(&(val + 10000).to_be_bytes());
                PathSecretRecord { update_id: u, node_index: (i % 256) as u32, secret_hash: h }
            })
            .collect();
        assert_eq!(
            validate_path_secret_uniqueness(&ss),
            Err(PathSecretUniquenessError::TooMany {
                got: TPSU_MAX_SECRETS + 1,
                max: TPSU_MAX_SECRETS,
            })
        );
    }

    /// **TPSU-07** — valid accepted.
    #[test]
    fn tpsu_07_valid_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&valid_batch()), Ok(()));
    }

    /// **TPSU-08** — empty accepted.
    #[test]
    fn tpsu_08_empty_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&[]), Ok(()));
    }

    /// **TPSU-09** — single accepted.
    #[test]
    fn tpsu_09_single_accepted() {
        assert_eq!(validate_path_secret_uniqueness(&[secret(0x01, 0, 0xAA)]), Ok(()));
    }

    /// **TPSU-10** — max node index accepted.
    #[test]
    fn tpsu_10_max_node_accepted() {
        let s = PathSecretRecord { update_id: uid(0x01), node_index: TPSU_MAX_NODE_INDEX, secret_hash: hash(0xAA) };
        assert_eq!(validate_path_secret_uniqueness(&[s]), Ok(()));
    }
}
