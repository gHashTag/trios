//! # CR-CHAT-05 — Store record tombstone garbage collection guard (Wave-101 Lane B)
//!
//! PERSISTENCE — deleted-record tombstones must be garbage-collected.
//!
//! When a store record is deleted, a tombstone marker is left in its
//! place to prevent replay. Over time these tombstones accumulate:
//!
//! * **Storage exhaustion** — unbounded tombstone growth consumes disk
//!   space proportional to the number of deleted records.
//! * **Deletion pattern leakage** — the density and timing of
//!   tombstones reveals which records were deleted, exposing user
//!   behaviour (e.g., message deletion patterns).
//! * **Performance degradation** — scanning past tombstones slows
//!   every read operation in proportion to tombstone count.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Tombstone age must not exceed `STGC_MAX_AGE_MS`.
//! 2. Total tombstones must not exceed `STGC_MAX_TOMBSTONES`.
//! 3. No duplicate record IDs.
//! 4. Record ID must not be zero.
//! 5. Deletion timestamp must be > 0.
//! 6. Tombstones must be ordered by deletion time (oldest first).
//!
//! Tests **STGC-01..10**. Error enum [`GcTombstoneError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOMBSTONE-GC`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum tombstone age in milliseconds.
pub const STGC_MAX_AGE_MS: u64 = 86_400_000;

/// Maximum tombstones per batch.
pub const STGC_MAX_TOMBSTONES: usize = 10_000;

/// Record ID length.
pub const STGC_RECORD_ID_LEN: usize = 16;

/// A tombstone record.
#[derive(Debug, Clone)]
pub struct GcTombstone {
    /// Record identifier.
    pub record_id: [u8; STGC_RECORD_ID_LEN],
    /// Deletion timestamp (ms since epoch).
    pub deleted_at: u64,
    /// Current timestamp (ms since epoch).
    pub now_ms: u64,
}

/// All ways tombstone GC validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GcTombstoneError {
    /// Tombstone too old.
    TooOld {
        /// Index of the offending tombstone.
        idx: usize,
        /// Age in milliseconds.
        age_ms: u64,
        /// Maximum allowed age.
        max_ms: u64,
    },
    /// Too many tombstones.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
    /// Duplicate record ID.
    DuplicateRecord(usize),
    /// Zero record ID.
    ZeroRecord(usize),
    /// Zero deletion timestamp.
    ZeroTimestamp(usize),
    /// Not ordered by deletion time.
    NotOrdered {
        /// Index of the offending tombstone.
        idx: usize,
        /// Previous deletion timestamp.
        prev: u64,
        /// Current deletion timestamp.
        current: u64,
    },
}

/// `[VERIFIED]` Validate tombstone garbage collection constraints.
pub fn validate_tombstone_gc(
    tombstones: &[GcTombstone],
) -> Result<(), GcTombstoneError> {
    if tombstones.len() > STGC_MAX_TOMBSTONES {
        return Err(GcTombstoneError::TooMany {
            got: tombstones.len(),
            max: STGC_MAX_TOMBSTONES,
        });
    }
    let mut seen: BTreeSet<[u8; STGC_RECORD_ID_LEN]> = BTreeSet::new();
    let mut prev_ts: u64 = 0;
    for (i, t) in tombstones.iter().enumerate() {
        if t.record_id == [0u8; STGC_RECORD_ID_LEN] {
            return Err(GcTombstoneError::ZeroRecord(i));
        }
        if t.deleted_at == 0 {
            return Err(GcTombstoneError::ZeroTimestamp(i));
        }
        if t.now_ms >= t.deleted_at {
            let age = t.now_ms - t.deleted_at;
            if age > STGC_MAX_AGE_MS {
                return Err(GcTombstoneError::TooOld {
                    idx: i,
                    age_ms: age,
                    max_ms: STGC_MAX_AGE_MS,
                });
            }
        }
        if i > 0 && t.deleted_at < prev_ts {
            return Err(GcTombstoneError::NotOrdered {
                idx: i,
                prev: prev_ts,
                current: t.deleted_at,
            });
        }
        if !seen.insert(t.record_id) {
            return Err(GcTombstoneError::DuplicateRecord(i));
        }
        prev_ts = t.deleted_at;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; STGC_RECORD_ID_LEN] {
        [byte; STGC_RECORD_ID_LEN]
    }

    fn tombstone(record_byte: u8, deleted_at: u64, now_ms: u64) -> GcTombstone {
        GcTombstone { record_id: rid(record_byte), deleted_at, now_ms }
    }

    fn valid_tombstones() -> Vec<GcTombstone> {
        vec![
            tombstone(0x01, 1000, 2000),
            tombstone(0x02, 1500, 2000),
            tombstone(0x03, 1800, 2000),
        ]
    }

    /// **STGC-01** — too old rejected.
    #[test]
    fn stgc_01_too_old_rejected() {
        let ts = vec![tombstone(0x01, 100, STGC_MAX_AGE_MS + 200)];
        assert_eq!(
            validate_tombstone_gc(&ts),
            Err(GcTombstoneError::TooOld {
                idx: 0,
                age_ms: STGC_MAX_AGE_MS + 100,
                max_ms: STGC_MAX_AGE_MS,
            })
        );
    }

    /// **STGC-02** — too many rejected.
    #[test]
    fn stgc_02_too_many_rejected() {
        let ts: Vec<GcTombstone> = (0..=STGC_MAX_TOMBSTONES)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                GcTombstone { record_id: rid(b), deleted_at: (i as u64) + 1, now_ms: (i as u64) + 2 }
            })
            .collect();
        assert!(matches!(
            validate_tombstone_gc(&ts),
            Err(GcTombstoneError::TooMany { .. })
        ));
    }

    /// **STGC-03** — duplicate record rejected.
    #[test]
    fn stgc_03_duplicate_rejected() {
        let ts = vec![tombstone(0x01, 100, 200), tombstone(0x01, 150, 200)];
        assert_eq!(
            validate_tombstone_gc(&ts),
            Err(GcTombstoneError::DuplicateRecord(1))
        );
    }

    /// **STGC-04** — zero record rejected.
    #[test]
    fn stgc_04_zero_record_rejected() {
        let t = GcTombstone { record_id: [0u8; STGC_RECORD_ID_LEN], deleted_at: 100, now_ms: 200 };
        assert_eq!(
            validate_tombstone_gc(&[t]),
            Err(GcTombstoneError::ZeroRecord(0))
        );
    }

    /// **STGC-05** — zero timestamp rejected.
    #[test]
    fn stgc_05_zero_timestamp_rejected() {
        let t = GcTombstone { record_id: rid(0x01), deleted_at: 0, now_ms: 200 };
        assert_eq!(
            validate_tombstone_gc(&[t]),
            Err(GcTombstoneError::ZeroTimestamp(0))
        );
    }

    /// **STGC-06** — not ordered rejected.
    #[test]
    fn stgc_06_not_ordered_rejected() {
        let ts = vec![tombstone(0x01, 500, 600), tombstone(0x02, 400, 600)];
        assert_eq!(
            validate_tombstone_gc(&ts),
            Err(GcTombstoneError::NotOrdered {
                idx: 1,
                prev: 500,
                current: 400,
            })
        );
    }

    /// **STGC-07** — valid accepted.
    #[test]
    fn stgc_07_valid_accepted() {
        assert_eq!(validate_tombstone_gc(&valid_tombstones()), Ok(()));
    }

    /// **STGC-08** — empty accepted.
    #[test]
    fn stgc_08_empty_accepted() {
        assert_eq!(validate_tombstone_gc(&[]), Ok(()));
    }

    /// **STGC-09** — single accepted.
    #[test]
    fn stgc_09_single_accepted() {
        let ts = vec![tombstone(0x01, 1000, 2000)];
        assert_eq!(validate_tombstone_gc(&ts), Ok(()));
    }

    /// **STGC-10** — boundary age accepted.
    #[test]
    fn stgc_10_boundary_age_accepted() {
        let ts = vec![tombstone(0x01, 1000, 1000 + STGC_MAX_AGE_MS)];
        assert_eq!(validate_tombstone_gc(&ts), Ok(()));
    }
}
