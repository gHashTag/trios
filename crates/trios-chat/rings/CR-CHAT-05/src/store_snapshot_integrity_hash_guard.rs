//! # CR-CHAT-05 — Store snapshot integrity hash guard (Wave-139 Lane B)
//!
//! PERSISTENCE — each store snapshot must carry a valid integrity
//! hash; missing or mismatched hashes indicate corruption.
//!
//! Store snapshots capture the full state at a point in time. Each
//! snapshot includes an integrity hash (SHA-256 of the serialized
//! state). Without integrity validation:
//!
//! * **Silent corruption** — disk bit flips or partial writes can
//!   corrupt the snapshot without detection.
//! * **Tampering** — an attacker with disk access can modify the
//!   snapshot state and recalculate the hash, but chain continuity
//!   catches inconsistencies across snapshots.
//! * **Recovery failure** — loading a corrupted snapshot causes
//!   undefined behavior in the application.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Computed hash must match declared hash.
//! 2. Snapshot ID must not be zero.
//! 3. No duplicate snapshot IDs.
//! 4. Snapshot sequence must be strictly increasing.
//! 5. First snapshot in batch must have seq = 1.
//! 6. Batch size <= `SSIH_MAX_SNAPSHOTS`.
//!
//! Tests **SSIH-01..10**. Error enum [`SnapshotIntegrityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * INTEGRITY-VERIFIED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum snapshots per batch.
pub const SSIH_MAX_SNAPSHOTS: usize = 1024;

/// Snapshot ID length.
pub const SSIH_SNAPSHOT_ID_LEN: usize = 16;

/// Hash length.
pub const SSIH_HASH_LEN: usize = 32;

/// A store snapshot integrity record.
#[derive(Debug, Clone)]
pub struct SnapshotIntegrityRecord {
    /// Snapshot identifier.
    pub snapshot_id: [u8; SSIH_SNAPSHOT_ID_LEN],
    /// Sequence number.
    pub seq: u64,
    /// Declared integrity hash.
    pub declared_hash: [u8; SSIH_HASH_LEN],
    /// Computed integrity hash.
    pub computed_hash: [u8; SSIH_HASH_LEN],
}

/// All ways snapshot integrity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SnapshotIntegrityError {
    /// Hash mismatch.
    HashMismatch {
        /// Index of the snapshot.
        idx: usize,
    },
    /// Zero snapshot ID.
    ZeroSnapshotId(
        /// Index.
        usize,
    ),
    /// Duplicate snapshot ID.
    DuplicateSnapshotId {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Non-increasing sequence.
    NonIncreasing {
        /// Index.
        idx: usize,
        /// Declared seq.
        got: u64,
        /// Expected minimum.
        expected: u64,
    },
    /// First seq must be 1.
    FirstSeqNotOne {
        /// Declared seq.
        got: u64,
    },
    /// Too many snapshots.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store snapshot integrity hash.
pub fn validate_snapshot_integrity(
    snapshots: &[SnapshotIntegrityRecord],
) -> Result<(), SnapshotIntegrityError> {
    if snapshots.len() > SSIH_MAX_SNAPSHOTS {
        return Err(SnapshotIntegrityError::TooMany {
            got: snapshots.len(),
            max: SSIH_MAX_SNAPSHOTS,
        });
    }
    let mut seen: BTreeSet<[u8; SSIH_SNAPSHOT_ID_LEN]> = BTreeSet::new();
    for (i, s) in snapshots.iter().enumerate() {
        if s.snapshot_id == [0u8; SSIH_SNAPSHOT_ID_LEN] {
            return Err(SnapshotIntegrityError::ZeroSnapshotId(i));
        }
        if !seen.insert(s.snapshot_id) {
            return Err(SnapshotIntegrityError::DuplicateSnapshotId { idx: i });
        }
        if i == 0 {
            if s.seq != 1 {
                return Err(SnapshotIntegrityError::FirstSeqNotOne { got: s.seq });
            }
        } else {
            if s.seq <= snapshots[i - 1].seq {
                return Err(SnapshotIntegrityError::NonIncreasing {
                    idx: i,
                    got: s.seq,
                    expected: snapshots[i - 1].seq + 1,
                });
            }
        }
        if s.declared_hash != s.computed_hash {
            return Err(SnapshotIntegrityError::HashMismatch { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; SSIH_SNAPSHOT_ID_LEN] {
        [byte; SSIH_SNAPSHOT_ID_LEN]
    }

    fn h(byte: u8) -> [u8; SSIH_HASH_LEN] {
        [byte; SSIH_HASH_LEN]
    }

    fn snap(id: u8, seq: u64, hash_byte: u8, match_hash: bool) -> SnapshotIntegrityRecord {
        SnapshotIntegrityRecord {
            snapshot_id: sid(id),
            seq,
            declared_hash: h(hash_byte),
            computed_hash: if match_hash { h(hash_byte) } else { h(hash_byte ^ 0xFF) },
        }
    }

    fn valid_snapshots() -> Vec<SnapshotIntegrityRecord> {
        vec![
            snap(0x01, 1, 0xAA, true),
            snap(0x02, 2, 0xBB, true),
            snap(0x03, 3, 0xCC, true),
        ]
    }

    /// **SSIH-01** — hash mismatch rejected.
    #[test]
    fn ssih_01_hash_mismatch_rejected() {
        let s = snap(0x01, 1, 0xAA, false);
        assert_eq!(
            validate_snapshot_integrity(&[s]),
            Err(SnapshotIntegrityError::HashMismatch { idx: 0 })
        );
    }

    /// **SSIH-02** — zero snapshot ID rejected.
    #[test]
    fn ssih_02_zero_id_rejected() {
        let s = SnapshotIntegrityRecord {
            snapshot_id: [0u8; SSIH_SNAPSHOT_ID_LEN],
            seq: 1,
            declared_hash: h(0xAA),
            computed_hash: h(0xAA),
        };
        assert_eq!(
            validate_snapshot_integrity(&[s]),
            Err(SnapshotIntegrityError::ZeroSnapshotId(0))
        );
    }

    /// **SSIH-03** — duplicate snapshot ID rejected.
    #[test]
    fn ssih_03_duplicate_rejected() {
        let ss = vec![
            snap(0x01, 1, 0xAA, true),
            snap(0x01, 2, 0xBB, true),
        ];
        assert_eq!(
            validate_snapshot_integrity(&ss),
            Err(SnapshotIntegrityError::DuplicateSnapshotId { idx: 1 })
        );
    }

    /// **SSIH-04** — non-increasing seq rejected.
    #[test]
    fn ssih_04_non_increasing_rejected() {
        let ss = vec![
            snap(0x01, 1, 0xAA, true),
            snap(0x02, 1, 0xBB, true),
        ];
        assert_eq!(
            validate_snapshot_integrity(&ss),
            Err(SnapshotIntegrityError::NonIncreasing { idx: 1, got: 1, expected: 2 })
        );
    }

    /// **SSIH-05** — first seq not 1 rejected.
    #[test]
    fn ssih_05_first_seq_not_one_rejected() {
        let s = snap(0x01, 5, 0xAA, true);
        assert_eq!(
            validate_snapshot_integrity(&[s]),
            Err(SnapshotIntegrityError::FirstSeqNotOne { got: 5 })
        );
    }

    /// **SSIH-06** — too many snapshots rejected.
    #[test]
    fn ssih_06_too_many_rejected() {
        let ss: Vec<SnapshotIntegrityRecord> = (0..=SSIH_MAX_SNAPSHOTS)
            .map(|i| {
                let mut id = [0u8; SSIH_SNAPSHOT_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                SnapshotIntegrityRecord {
                    snapshot_id: id,
                    seq: (i as u64) + 1,
                    declared_hash: h((i % 256) as u8),
                    computed_hash: h((i % 256) as u8),
                }
            })
            .collect();
        assert_eq!(
            validate_snapshot_integrity(&ss),
            Err(SnapshotIntegrityError::TooMany {
                got: SSIH_MAX_SNAPSHOTS + 1,
                max: SSIH_MAX_SNAPSHOTS,
            })
        );
    }

    /// **SSIH-07** — valid accepted.
    #[test]
    fn ssih_07_valid_accepted() {
        assert_eq!(validate_snapshot_integrity(&valid_snapshots()), Ok(()));
    }

    /// **SSIH-08** — empty accepted.
    #[test]
    fn ssih_08_empty_accepted() {
        assert_eq!(validate_snapshot_integrity(&[]), Ok(()));
    }

    /// **SSIH-09** — single snapshot seq=1 accepted.
    #[test]
    fn ssih_09_single_accepted() {
        assert_eq!(validate_snapshot_integrity(&[snap(0x01, 1, 0xAA, true)]), Ok(()));
    }

    /// **SSIH-10** — long valid chain accepted.
    #[test]
    fn ssih_10_long_chain_accepted() {
        let ss: Vec<SnapshotIntegrityRecord> = (0..50)
            .map(|i| {
                let mut id = [0u8; SSIH_SNAPSHOT_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                SnapshotIntegrityRecord {
                    snapshot_id: id,
                    seq: (i as u64) + 1,
                    declared_hash: h((i % 256) as u8),
                    computed_hash: h((i % 256) as u8),
                }
            })
            .collect();
        assert_eq!(validate_snapshot_integrity(&ss), Ok(()));
    }
}
