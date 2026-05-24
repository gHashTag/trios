//! # CR-CHAT-03 — TreeKEM path secret freshness guard (Wave-151 Lane A)
//!
//! RATCHET TREE — path secrets must be freshly generated per update;
//! reuse across updates enables insider attacks.
//!
//! In MLS/TreeKEM, each node update generates fresh path secrets
//! that are mixed up the tree. If the same path secret material is
//! reused across different updates:
//!
//! * **Insider attack** — a group member who observed a previous
//!   update can predict future path secrets if they're reused.
//! * **Path secret recovery** — reusing entropy across updates
//!   creates correlations that weaken forward secrecy.
//! * **Tree reconstruction** — an attacker can reconstruct partial
//!   tree state if path secrets are correlated.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All path secret IDs must be unique.
//! 2. Secret must not be zero.
//! 3. Update epoch must be > 0.
//! 4. No duplicate (epoch, node_index) pairs.
//! 5. Node index must be < `TPSF2_MAX_NODES`.
//! 6. Batch size <= `TPSF2_MAX_UPDATES`.
//!
//! Tests **TPSF-01..10**. Error enum [`PathFreshnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-FRESH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum updates per batch.
pub const TPSF2_MAX_UPDATES: usize = 512;

/// Maximum node index.
pub const TPSF2_MAX_NODES: u64 = 1024;

/// Secret length.
pub const TPSF2_SECRET_LEN: usize = 32;

/// Update ID length.
pub const TPSF2_UPDATE_ID_LEN: usize = 16;

/// A path secret update record.
#[derive(Debug, Clone)]
pub struct PathSecretUpdate {
    /// Update identifier.
    pub update_id: [u8; TPSF2_UPDATE_ID_LEN],
    /// Epoch number.
    pub epoch: u64,
    /// Node index in the tree.
    pub node_index: u64,
    /// Path secret material.
    pub secret: [u8; TPSF2_SECRET_LEN],
}

/// All ways path freshness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathFreshnessError {
    /// Duplicate update ID.
    DuplicateUpdateId {
        /// Index.
        idx: usize,
    },
    /// Zero secret.
    ZeroSecret(usize),
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Duplicate epoch+node pair.
    DuplicateEpochNode {
        /// Index.
        idx: usize,
    },
    /// Node index out of range.
    NodeOutOfRange {
        idx: usize,
        got: u64,
        max: u64,
    },
    /// Too many updates.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate path secret freshness.
pub fn validate_path_freshness(
    updates: &[PathSecretUpdate],
) -> Result<(), PathFreshnessError> {
    if updates.len() > TPSF2_MAX_UPDATES {
        return Err(PathFreshnessError::TooMany {
            got: updates.len(),
            max: TPSF2_MAX_UPDATES,
        });
    }
    let mut seen_ids: BTreeSet<[u8; TPSF2_UPDATE_ID_LEN]> = BTreeSet::new();
    let mut seen_pairs: BTreeSet<(u64, u64)> = BTreeSet::new();
    for (i, u) in updates.iter().enumerate() {
        if u.update_id == [0u8; TPSF2_UPDATE_ID_LEN] {
            return Err(PathFreshnessError::DuplicateUpdateId { idx: i });
        }
        if !seen_ids.insert(u.update_id) {
            return Err(PathFreshnessError::DuplicateUpdateId { idx: i });
        }
        if u.epoch == 0 {
            return Err(PathFreshnessError::ZeroEpoch(i));
        }
        if u.node_index >= TPSF2_MAX_NODES {
            return Err(PathFreshnessError::NodeOutOfRange {
                idx: i,
                got: u.node_index,
                max: TPSF2_MAX_NODES,
            });
        }
        if u.secret == [0u8; TPSF2_SECRET_LEN] {
            return Err(PathFreshnessError::ZeroSecret(i));
        }
        if !seen_pairs.insert((u.epoch, u.node_index)) {
            return Err(PathFreshnessError::DuplicateEpochNode { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uid(byte: u8) -> [u8; TPSF2_UPDATE_ID_LEN] {
        [byte; TPSF2_UPDATE_ID_LEN]
    }

    fn sec(byte: u8) -> [u8; TPSF2_SECRET_LEN] {
        [byte; TPSF2_SECRET_LEN]
    }

    fn upd(id: u8, epoch: u64, node: u64, secret: u8) -> PathSecretUpdate {
        PathSecretUpdate { update_id: uid(id), epoch, node_index: node, secret: sec(secret) }
    }

    fn valid_updates() -> Vec<PathSecretUpdate> {
        vec![
            upd(0x01, 1, 0, 0xA1),
            upd(0x02, 1, 1, 0xA2),
            upd(0x03, 2, 0, 0xB1),
        ]
    }

    /// **TPSF-01** — duplicate update ID rejected.
    #[test]
    fn tpsf_01_duplicate_id_rejected() {
        let us = vec![
            upd(0x01, 1, 0, 0xA1),
            upd(0x01, 2, 1, 0xA2),
        ];
        assert_eq!(
            validate_path_freshness(&us),
            Err(PathFreshnessError::DuplicateUpdateId { idx: 1 })
        );
    }

    /// **TPSF-02** — zero secret rejected.
    #[test]
    fn tpsf_02_zero_secret_rejected() {
        let u = PathSecretUpdate { update_id: uid(0x01), epoch: 1, node_index: 0, secret: [0u8; TPSF2_SECRET_LEN] };
        assert_eq!(
            validate_path_freshness(&[u]),
            Err(PathFreshnessError::ZeroSecret(0))
        );
    }

    /// **TPSF-03** — zero epoch rejected.
    #[test]
    fn tpsf_03_zero_epoch_rejected() {
        let u = PathSecretUpdate { update_id: uid(0x01), epoch: 0, node_index: 0, secret: sec(0xA1) };
        assert_eq!(
            validate_path_freshness(&[u]),
            Err(PathFreshnessError::ZeroEpoch(0))
        );
    }

    /// **TPSF-04** — duplicate epoch+node rejected.
    #[test]
    fn tpsf_04_duplicate_epoch_node_rejected() {
        let us = vec![
            upd(0x01, 1, 0, 0xA1),
            upd(0x02, 1, 0, 0xA2),
        ];
        assert_eq!(
            validate_path_freshness(&us),
            Err(PathFreshnessError::DuplicateEpochNode { idx: 1 })
        );
    }

    /// **TPSF-05** — node out of range rejected.
    #[test]
    fn tpsf_05_node_out_of_range_rejected() {
        let u = PathSecretUpdate { update_id: uid(0x01), epoch: 1, node_index: TPSF2_MAX_NODES, secret: sec(0xA1) };
        assert_eq!(
            validate_path_freshness(&[u]),
            Err(PathFreshnessError::NodeOutOfRange { idx: 0, got: TPSF2_MAX_NODES, max: TPSF2_MAX_NODES })
        );
    }

    /// **TPSF-06** — too many rejected.
    #[test]
    fn tpsf_06_too_many_rejected() {
        let us: Vec<PathSecretUpdate> = (0..=TPSF2_MAX_UPDATES)
            .map(|i| {
                let val = (i as u64) + 1;
                let mut id = [0u8; TPSF2_UPDATE_ID_LEN];
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut s = [0u8; TPSF2_SECRET_LEN];
                s[0] = (i as u8).wrapping_add(1);
                PathSecretUpdate { update_id: id, epoch: val, node_index: val % TPSF2_MAX_NODES, secret: s }
            })
            .collect();
        assert_eq!(
            validate_path_freshness(&us),
            Err(PathFreshnessError::TooMany {
                got: TPSF2_MAX_UPDATES + 1,
                max: TPSF2_MAX_UPDATES,
            })
        );
    }

    /// **TPSF-07** — valid accepted.
    #[test]
    fn tpsf_07_valid_accepted() {
        assert_eq!(validate_path_freshness(&valid_updates()), Ok(()));
    }

    /// **TPSF-08** — empty accepted.
    #[test]
    fn tpsf_08_empty_accepted() {
        assert_eq!(validate_path_freshness(&[]), Ok(()));
    }

    /// **TPSF-09** — same node different epoch accepted.
    #[test]
    fn tpsf_09_same_node_diff_epoch() {
        let us = vec![
            upd(0x01, 1, 0, 0xA1),
            upd(0x02, 2, 0, 0xB1),
        ];
        assert_eq!(validate_path_freshness(&us), Ok(()));
    }

    /// **TPSF-10** — boundary node accepted.
    #[test]
    fn tpsf_10_boundary_node_accepted() {
        let u = PathSecretUpdate { update_id: uid(0x01), epoch: 1, node_index: TPSF2_MAX_NODES - 1, secret: sec(0xA1) };
        assert_eq!(validate_path_freshness(&[u]), Ok(()));
    }
}
