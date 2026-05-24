//! # CR-CHAT-03 — TreeKEM update path uniqueness guard (Wave-112 Lane B)
//!
//! RATCHET TREE — each update path must be unique across epochs.
//!
//! In TreeKEM, each member's update path is a sequence of nodes from
//! their leaf to the root. If the same path is reused:
//!
//! * **Cross-epoch correlation** — the adversary links two epochs by
//!   matching the update path structure.
//! * **Path predictability** — reused paths allow the adversary to
//!   predict which nodes will be updated, enabling targeted attacks.
//! * **State recovery** — if paths repeat, compromising one epoch's
//!   path secrets may compromise future epochs.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate update paths across epochs.
//! 2. Epoch number must be strictly increasing.
//! 3. Epoch number must not be zero.
//! 4. Path must not be empty.
//! 5. Path hash must not be zero.
//! 6. Total paths <= `TPUN_MAX_PATHS`.
//!
//! Tests **TPUN-01..10**. Error enum [`PathUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PATH-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum update paths per batch.
pub const TPUN_MAX_PATHS: usize = 1024;

/// Path hash length.
pub const TPUN_HASH_LEN: usize = 32;

/// An update path record.
#[derive(Debug, Clone)]
pub struct UpdatePath {
    /// Epoch number.
    pub epoch: u64,
    /// Hash of the update path (nodes from leaf to root).
    pub path_hash: [u8; TPUN_HASH_LEN],
    /// Number of nodes in the path.
    pub path_len: usize,
}

/// All ways path uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathUniquenessError {
    /// Duplicate path hash.
    DuplicatePath(usize),
    /// Not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Empty path.
    EmptyPath(usize),
    /// Zero hash.
    ZeroHash(usize),
    /// Too many paths.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM update path uniqueness.
pub fn validate_update_path_uniqueness(
    paths: &[UpdatePath],
) -> Result<(), PathUniquenessError> {
    if paths.len() > TPUN_MAX_PATHS {
        return Err(PathUniquenessError::TooMany {
            got: paths.len(),
            max: TPUN_MAX_PATHS,
        });
    }
    let mut seen: BTreeSet<[u8; TPUN_HASH_LEN]> = BTreeSet::new();
    let mut prev_epoch: u64 = 0;
    for (i, p) in paths.iter().enumerate() {
        if p.epoch == 0 {
            return Err(PathUniquenessError::ZeroEpoch(i));
        }
        if p.path_hash == [0u8; TPUN_HASH_LEN] {
            return Err(PathUniquenessError::ZeroHash(i));
        }
        if p.path_len == 0 {
            return Err(PathUniquenessError::EmptyPath(i));
        }
        if i > 0 && p.epoch <= prev_epoch {
            return Err(PathUniquenessError::NotIncreasing {
                idx: i,
                prev: prev_epoch,
                current: p.epoch,
            });
        }
        if !seen.insert(p.path_hash) {
            return Err(PathUniquenessError::DuplicatePath(i));
        }
        prev_epoch = p.epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; TPUN_HASH_LEN] {
        [byte; TPUN_HASH_LEN]
    }

    fn upath(epoch: u64, hash_byte: u8, len: usize) -> UpdatePath {
        UpdatePath { epoch, path_hash: hash(hash_byte), path_len: len }
    }

    fn valid_paths() -> Vec<UpdatePath> {
        vec![
            upath(1, 0x01, 4),
            upath(2, 0x02, 5),
            upath(3, 0x03, 4),
        ]
    }

    /// **TPUN-01** — duplicate path rejected.
    #[test]
    fn tpun_01_duplicate_rejected() {
        let ps = vec![upath(1, 0xAA, 4), upath(2, 0xAA, 5)];
        assert_eq!(
            validate_update_path_uniqueness(&ps),
            Err(PathUniquenessError::DuplicatePath(1))
        );
    }

    /// **TPUN-02** — not increasing rejected.
    #[test]
    fn tpun_02_not_increasing_rejected() {
        let ps = vec![upath(5, 0x01, 4), upath(3, 0x02, 4)];
        assert_eq!(
            validate_update_path_uniqueness(&ps),
            Err(PathUniquenessError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **TPUN-03** — zero epoch rejected.
    #[test]
    fn tpun_03_zero_epoch_rejected() {
        let p = UpdatePath { epoch: 0, path_hash: hash(0x01), path_len: 4 };
        assert_eq!(
            validate_update_path_uniqueness(&[p]),
            Err(PathUniquenessError::ZeroEpoch(0))
        );
    }

    /// **TPUN-04** — empty path rejected.
    #[test]
    fn tpun_04_empty_path_rejected() {
        let p = UpdatePath { epoch: 1, path_hash: hash(0x01), path_len: 0 };
        assert_eq!(
            validate_update_path_uniqueness(&[p]),
            Err(PathUniquenessError::EmptyPath(0))
        );
    }

    /// **TPUN-05** — zero hash rejected.
    #[test]
    fn tpun_05_zero_hash_rejected() {
        let p = UpdatePath { epoch: 1, path_hash: [0u8; TPUN_HASH_LEN], path_len: 4 };
        assert_eq!(
            validate_update_path_uniqueness(&[p]),
            Err(PathUniquenessError::ZeroHash(0))
        );
    }

    /// **TPUN-06** — too many rejected.
    #[test]
    fn tpun_06_too_many_rejected() {
        let ps: Vec<UpdatePath> = (0..=TPUN_MAX_PATHS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                UpdatePath { epoch: (i as u64) + 1, path_hash: hash(b), path_len: 4 }
            })
            .collect();
        assert_eq!(
            validate_update_path_uniqueness(&ps),
            Err(PathUniquenessError::TooMany {
                got: TPUN_MAX_PATHS + 1,
                max: TPUN_MAX_PATHS,
            })
        );
    }

    /// **TPUN-07** — valid accepted.
    #[test]
    fn tpun_07_valid_accepted() {
        assert_eq!(validate_update_path_uniqueness(&valid_paths()), Ok(()));
    }

    /// **TPUN-08** — empty accepted.
    #[test]
    fn tpun_08_empty_accepted() {
        assert_eq!(validate_update_path_uniqueness(&[]), Ok(()));
    }

    /// **TPUN-09** — single accepted.
    #[test]
    fn tpun_09_single_accepted() {
        let ps = vec![upath(1, 0x01, 4)];
        assert_eq!(validate_update_path_uniqueness(&ps), Ok(()));
    }

    /// **TPUN-10** — different paths same length accepted.
    #[test]
    fn tpun_10_diff_paths_accepted() {
        let ps = vec![
            upath(1, 0x01, 4),
            upath(2, 0x02, 4),
            upath(3, 0x03, 4),
        ];
        assert_eq!(validate_update_path_uniqueness(&ps), Ok(()));
    }
}
