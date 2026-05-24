//! # CR-CHAT-03 — TreeKEM update path coverage guard (Wave-98 Lane B)
//!
//! RATCHET TREE — update path must cover all direct-path nodes,
//! R-CHAT-3.
//!
//! In TreeKEM, an UpdatePath contains fresh path secrets from the
//! sender's leaf to the root. If the path is incomplete:
//!
//! * **Stale subtree** — nodes not covered by the update retain old
//!   secrets, so a removed member can still derive group keys.
//! * **Key inconsistency** — peers compute different tree hashes
//!   because some nodes were updated and others were not.
//! * **Resolution failure** — tree resolution skips stale nodes,
//!   producing incorrect encryption targets.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Path length must equal expected depth.
//! 2. Node indices must be strictly increasing (ascending path).
//! 3. No duplicate node indices.
//! 4. Sender leaf must be < `TUPC_MAX_LEAVES`.
//! 5. Total paths <= `TUPC_MAX_PATHS`.
//! 6. All path secrets must be non-zero.
//!
//! Tests **TUPC-01..10**. Error enum [`PathCoverageError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-COVERAGE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum leaves in the tree.
pub const TUPC_MAX_LEAVES: u64 = 1024;

/// Maximum paths to validate.
pub const TUPC_MAX_PATHS: usize = 256;

/// An update path record.
#[derive(Debug, Clone)]
pub struct UpdatePathRecord {
    /// Sender leaf index.
    pub sender_leaf: u64,
    /// Node indices along the direct path (ascending).
    pub node_indices: Vec<u64>,
    /// Expected path length (tree depth).
    pub expected_len: usize,
    /// Whether all path secrets are non-zero.
    pub secrets_non_zero: bool,
}

/// All ways path coverage validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathCoverageError {
    /// Path length mismatch.
    LengthMismatch { expected: usize, got: usize },
    /// Indices not increasing.
    NotIncreasing(u64),
    /// Duplicate index.
    DuplicateIndex(u64),
    /// Sender out of range.
    SenderOutOfRange(u64),
    /// Too many paths.
    TooManyPaths,
    /// Zero secret in path.
    ZeroSecret(u64),
}

/// `[VERIFIED]` Validate TreeKEM update path coverage.
pub fn validate_update_path_coverage(
    paths: &[UpdatePathRecord],
) -> Result<(), PathCoverageError> {
    if paths.len() > TUPC_MAX_PATHS {
        return Err(PathCoverageError::TooManyPaths);
    }
    for p in paths {
        if p.sender_leaf >= TUPC_MAX_LEAVES {
            return Err(PathCoverageError::SenderOutOfRange(p.sender_leaf));
        }
        if p.node_indices.len() != p.expected_len {
            return Err(PathCoverageError::LengthMismatch {
                expected: p.expected_len,
                got: p.node_indices.len(),
            });
        }
        if !p.secrets_non_zero {
            return Err(PathCoverageError::ZeroSecret(p.sender_leaf));
        }
        let mut seen = BTreeSet::new();
        for (i, &idx) in p.node_indices.iter().enumerate() {
            if !seen.insert(idx) {
                return Err(PathCoverageError::DuplicateIndex(idx));
            }
            if i > 0 && idx <= p.node_indices[i - 1] {
                return Err(PathCoverageError::NotIncreasing(idx));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(sender: u64, nodes: Vec<u64>) -> UpdatePathRecord {
        let expected = nodes.len();
        UpdatePathRecord {
            sender_leaf: sender,
            node_indices: nodes,
            expected_len: expected,
            secrets_non_zero: true,
        }
    }

    fn valid_paths() -> Vec<UpdatePathRecord> {
        vec![path(0, vec![1, 3, 7]), path(2, vec![3, 7, 15])]
    }

    /// **TUPC-01** — length mismatch rejected.
    #[test]
    fn tupc_01_length_mismatch_rejected() {
        let p = UpdatePathRecord {
            sender_leaf: 0,
            node_indices: vec![1, 3],
            expected_len: 3,
            secrets_non_zero: true,
        };
        assert_eq!(
            validate_update_path_coverage(&[p]),
            Err(PathCoverageError::LengthMismatch { expected: 3, got: 2 })
        );
    }

    /// **TUPC-02** — not increasing rejected.
    #[test]
    fn tupc_02_not_increasing_rejected() {
        let p = path(0, vec![7, 3, 15]);
        assert_eq!(
            validate_update_path_coverage(&[p]),
            Err(PathCoverageError::NotIncreasing(3))
        );
    }

    /// **TUPC-03** — duplicate index rejected.
    #[test]
    fn tupc_03_duplicate_rejected() {
        let p = path(0, vec![1, 3, 3]);
        assert_eq!(
            validate_update_path_coverage(&[p]),
            Err(PathCoverageError::DuplicateIndex(3))
        );
    }

    /// **TUPC-04** — sender out of range rejected.
    #[test]
    fn tupc_04_sender_out_of_range_rejected() {
        let p = path(TUPC_MAX_LEAVES, vec![1]);
        assert_eq!(
            validate_update_path_coverage(&[p]),
            Err(PathCoverageError::SenderOutOfRange(TUPC_MAX_LEAVES))
        );
    }

    /// **TUPC-05** — too many paths rejected.
    #[test]
    fn tupc_05_too_many_rejected() {
        let paths: Vec<UpdatePathRecord> = (0..=TUPC_MAX_PATHS as u64)
            .map(|i| path(i, vec![i + 1]))
            .collect();
        assert_eq!(validate_update_path_coverage(&paths), Err(PathCoverageError::TooManyPaths));
    }

    /// **TUPC-06** — zero secret rejected.
    #[test]
    fn tupc_06_zero_secret_rejected() {
        let p = UpdatePathRecord {
            sender_leaf: 0,
            node_indices: vec![1, 3],
            expected_len: 2,
            secrets_non_zero: false,
        };
        assert_eq!(
            validate_update_path_coverage(&[p]),
            Err(PathCoverageError::ZeroSecret(0))
        );
    }

    /// **TUPC-07** — valid paths accepted.
    #[test]
    fn tupc_07_valid_accepted() {
        assert_eq!(validate_update_path_coverage(&valid_paths()), Ok(()));
    }

    /// **TUPC-08** — empty accepted.
    #[test]
    fn tupc_08_empty_accepted() {
        assert_eq!(validate_update_path_coverage(&[]), Ok(()));
    }

    /// **TUPC-09** — single node path accepted.
    #[test]
    fn tupc_09_single_node_accepted() {
        assert_eq!(validate_update_path_coverage(&[path(0, vec![1])]), Ok(()));
    }

    /// **TUPC-10** — max paths boundary accepted.
    #[test]
    fn tupc_10_max_boundary_accepted() {
        let paths: Vec<UpdatePathRecord> = (0..TUPC_MAX_PATHS as u64)
            .map(|i| path(i, vec![i + 1]))
            .collect();
        assert_eq!(validate_update_path_coverage(&paths), Ok(()));
    }
}
