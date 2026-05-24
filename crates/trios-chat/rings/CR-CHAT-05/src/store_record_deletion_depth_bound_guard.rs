//! # CR-CHAT-05 — Store record deletion depth bound guard (Wave-153 Lane A)
//!
//! PERSISTENCE — tombstoned records must have bounded deletion cascade
//! depth; unbounded cascades enable DoS.
//!
//! When records are deleted via cascading tombstones, the depth of the
//! cascade must be bounded. If cascades can grow without limit:
//!
//! * **Denial of service** — an attacker can trigger deep cascades
//!   that consume excessive CPU and I/O during garbage collection.
//! * **Resource exhaustion** — unbounded cascade depth leads to
//!   stack overflow or excessive memory usage.
//! * **Liveness violation** — deep cascades block other operations,
//!   violating liveness guarantees.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cascade depth <= `SRDD_MAX_DEPTH`.
//! 2. Record ID must not be zero.
//! 3. No duplicate record IDs.
//! 4. Parent ID must not equal own ID (self-loop).
//! 5. Root record must have zero parent.
//! 6. Batch size <= `SRDD_MAX_RECORDS`.
//!
//! Tests **SRDD-01..10**. Error enum [`DeletionDepthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DEPTH-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum cascade depth.
pub const SRDD_MAX_DEPTH: u32 = 64;

/// Maximum records per batch.
pub const SRDD_MAX_RECORDS: usize = 512;

/// Record ID length.
pub const SRDD_RECORD_ID_LEN: usize = 16;

/// A deletion cascade record.
#[derive(Debug, Clone)]
pub struct DeletionRecord {
    /// Unique record identifier.
    pub record_id: [u8; SRDD_RECORD_ID_LEN],
    /// Parent record ID (all-zero for root).
    pub parent_id: [u8; SRDD_RECORD_ID_LEN],
    /// Depth in the cascade tree (0 for root).
    pub depth: u32,
}

/// All ways deletion depth validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeletionDepthError {
    /// Depth exceeds maximum.
    TooDeep {
        /// Index of the offending record.
        idx: usize,
        /// Actual depth.
        got: u32,
        /// Maximum allowed depth.
        max: u32,
    },
    /// Zero record ID.
    ZeroRecordId(usize),
    /// Duplicate record ID.
    DuplicateRecordId {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Self-loop (parent == self).
    SelfLoop(usize),
    /// Non-zero parent on root.
    NonZeroParentOnRoot(usize),
    /// Too many records.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store record deletion depth bound.
pub fn validate_deletion_depth(
    records: &[DeletionRecord],
) -> Result<(), DeletionDepthError> {
    if records.len() > SRDD_MAX_RECORDS {
        return Err(DeletionDepthError::TooMany {
            got: records.len(),
            max: SRDD_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<[u8; SRDD_RECORD_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.record_id == [0u8; SRDD_RECORD_ID_LEN] {
            return Err(DeletionDepthError::ZeroRecordId(i));
        }
        if !seen.insert(r.record_id) {
            return Err(DeletionDepthError::DuplicateRecordId { idx: i });
        }
        if r.parent_id == r.record_id {
            return Err(DeletionDepthError::SelfLoop(i));
        }
        if r.depth == 0 && r.parent_id != [0u8; SRDD_RECORD_ID_LEN] {
            return Err(DeletionDepthError::NonZeroParentOnRoot(i));
        }
        if r.depth > SRDD_MAX_DEPTH {
            return Err(DeletionDepthError::TooDeep {
                idx: i,
                got: r.depth,
                max: SRDD_MAX_DEPTH,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; SRDD_RECORD_ID_LEN] {
        [byte; SRDD_RECORD_ID_LEN]
    }

    fn rec(id: u8, parent: u8, depth: u32) -> DeletionRecord {
        DeletionRecord { record_id: rid(id), parent_id: rid(parent), depth }
    }

    fn root(id: u8) -> DeletionRecord {
        DeletionRecord { record_id: rid(id), parent_id: [0u8; SRDD_RECORD_ID_LEN], depth: 0 }
    }

    fn valid_records() -> Vec<DeletionRecord> {
        vec![
            root(0x01),
            rec(0x02, 0x01, 1),
            rec(0x03, 0x01, 1),
            rec(0x04, 0x02, 2),
        ]
    }

    /// **SRDD-01** — too deep rejected.
    #[test]
    fn srdd_01_too_deep_rejected() {
        let r = DeletionRecord { record_id: rid(0x01), parent_id: rid(0x02), depth: SRDD_MAX_DEPTH + 1 };
        assert_eq!(
            validate_deletion_depth(&[r]),
            Err(DeletionDepthError::TooDeep { idx: 0, got: SRDD_MAX_DEPTH + 1, max: SRDD_MAX_DEPTH })
        );
    }

    /// **SRDD-02** — zero record ID rejected.
    #[test]
    fn srdd_02_zero_id_rejected() {
        let r = DeletionRecord { record_id: [0u8; SRDD_RECORD_ID_LEN], parent_id: [0u8; SRDD_RECORD_ID_LEN], depth: 0 };
        assert_eq!(
            validate_deletion_depth(&[r]),
            Err(DeletionDepthError::ZeroRecordId(0))
        );
    }

    /// **SRDD-03** — duplicate record ID rejected.
    #[test]
    fn srdd_03_duplicate_rejected() {
        let rs = vec![
            root(0x01),
            root(0x01),
        ];
        assert_eq!(
            validate_deletion_depth(&rs),
            Err(DeletionDepthError::DuplicateRecordId { idx: 1 })
        );
    }

    /// **SRDD-04** — self-loop rejected.
    #[test]
    fn srdd_04_self_loop_rejected() {
        let r = DeletionRecord { record_id: rid(0x01), parent_id: rid(0x01), depth: 1 };
        assert_eq!(
            validate_deletion_depth(&[r]),
            Err(DeletionDepthError::SelfLoop(0))
        );
    }

    /// **SRDD-05** — non-zero parent on root rejected.
    #[test]
    fn srdd_05_nonzero_parent_on_root_rejected() {
        let r = DeletionRecord { record_id: rid(0x01), parent_id: rid(0x02), depth: 0 };
        assert_eq!(
            validate_deletion_depth(&[r]),
            Err(DeletionDepthError::NonZeroParentOnRoot(0))
        );
    }

    /// **SRDD-06** — too many rejected.
    #[test]
    fn srdd_06_too_many_rejected() {
        let rs: Vec<DeletionRecord> = (0..=SRDD_MAX_RECORDS)
            .map(|i| {
                let mut id = [0u8; SRDD_RECORD_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                DeletionRecord { record_id: id, parent_id: [0u8; SRDD_RECORD_ID_LEN], depth: 0 }
            })
            .collect();
        assert_eq!(
            validate_deletion_depth(&rs),
            Err(DeletionDepthError::TooMany {
                got: SRDD_MAX_RECORDS + 1,
                max: SRDD_MAX_RECORDS,
            })
        );
    }

    /// **SRDD-07** — valid accepted.
    #[test]
    fn srdd_07_valid_accepted() {
        assert_eq!(validate_deletion_depth(&valid_records()), Ok(()));
    }

    /// **SRDD-08** — empty accepted.
    #[test]
    fn srdd_08_empty_accepted() {
        assert_eq!(validate_deletion_depth(&[]), Ok(()));
    }

    /// **SRDD-09** — boundary depth accepted.
    #[test]
    fn srdd_09_boundary_depth_accepted() {
        let r = DeletionRecord { record_id: rid(0x01), parent_id: rid(0x02), depth: SRDD_MAX_DEPTH };
        assert_eq!(validate_deletion_depth(&[r]), Ok(()));
    }

    /// **SRDD-10** — deep chain accepted.
    #[test]
    fn srdd_10_deep_chain_accepted() {
        let rs: Vec<DeletionRecord> = (0..20u8)
            .map(|i| {
                if i == 0 {
                    root(i + 1)
                } else {
                    rec(i + 1, i, i as u32)
                }
            })
            .collect();
        assert_eq!(validate_deletion_depth(&rs), Ok(()));
    }
}
