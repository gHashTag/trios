//! # CR-CHAT-05 — Store record tombstone resurrection guard (Wave-146 Lane A)
//!
//! PERSISTENCE — tombstoned records must not be resurrected;
//! reappearing tombstones indicate store corruption.
//!
//! When a store record is deleted, it is marked with a tombstone.
//! If a tombstoned record reappears (i.e. a new record with the same
//! key is written after deletion):
//!
//! * **Store corruption** — the tombstone was not properly persisted,
//!   allowing the old record to be overwritten.
//! * **Data resurrection** — deleted data reappears, violating the
//!   user's expectation of permanent deletion.
//! * **Audit inconsistency** — tombstone integrity is a key audit
//!   property; resurrection breaks the deletion guarantee.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No record key may appear after its tombstone.
//! 2. Record key must not be zero.
//! 3. No duplicate tombstone keys.
//! 4. Sequence must be strictly increasing.
//! 5. First entry must have seq = 1.
//! 6. Batch size <= `SRTG_MAX_ENTRIES`.
//!
//! Tests **SRTG-01..10**. Error enum [`TombstoneResurrectionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOMBSTONE-FINAL`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum entries per batch.
pub const SRTG_MAX_ENTRIES: usize = 2048;

/// Record key length.
pub const SRTG_KEY_LEN: usize = 32;

/// A tombstone event record.
#[derive(Debug, Clone)]
pub struct TombstoneRecord {
    /// Record key.
    pub key: [u8; SRTG_KEY_LEN],
    /// Sequence number.
    pub seq: u64,
    /// Whether this entry is a tombstone (deletion).
    pub is_tombstone: bool,
}

/// All ways tombstone resurrection validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TombstoneResurrectionError {
    /// Record appeared after tombstone.
    Resurrected {
        /// Index of the resurrected record.
        idx: usize,
    },
    /// Zero key.
    ZeroKey(
        /// Index.
        usize,
    ),
    /// Duplicate tombstone.
    DuplicateTombstone {
        /// Index.
        idx: usize,
    },
    /// Non-increasing sequence.
    NonIncreasing {
        /// Index.
        idx: usize,
        /// Declared seq.
        got: u64,
        /// Previous seq.
        prev: u64,
    },
    /// First seq must be 1.
    FirstSeqNotOne {
        /// Declared seq.
        got: u64,
    },
    /// Too many entries.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store record tombstone resurrection.
pub fn validate_tombstone_resurrection(
    entries: &[TombstoneRecord],
) -> Result<(), TombstoneResurrectionError> {
    if entries.len() > SRTG_MAX_ENTRIES {
        return Err(TombstoneResurrectionError::TooMany {
            got: entries.len(),
            max: SRTG_MAX_ENTRIES,
        });
    }
    let mut tombstoned: BTreeSet<[u8; SRTG_KEY_LEN]> = BTreeSet::new();
    let mut tombstone_set: BTreeSet<[u8; SRTG_KEY_LEN]> = BTreeSet::new();
    for (i, e) in entries.iter().enumerate() {
        if e.key == [0u8; SRTG_KEY_LEN] {
            return Err(TombstoneResurrectionError::ZeroKey(i));
        }
        if i == 0 {
            if e.seq != 1 {
                return Err(TombstoneResurrectionError::FirstSeqNotOne { got: e.seq });
            }
        } else {
            if e.seq <= entries[i - 1].seq {
                return Err(TombstoneResurrectionError::NonIncreasing {
                    idx: i,
                    got: e.seq,
                    prev: entries[i - 1].seq,
                });
            }
        }
        if e.is_tombstone {
            if !tombstone_set.insert(e.key) {
                return Err(TombstoneResurrectionError::DuplicateTombstone { idx: i });
            }
            tombstoned.insert(e.key);
        } else {
            if tombstoned.contains(&e.key) {
                return Err(TombstoneResurrectionError::Resurrected { idx: i });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; SRTG_KEY_LEN] {
        [byte; SRTG_KEY_LEN]
    }

    fn put(k: u8, seq: u64) -> TombstoneRecord {
        TombstoneRecord { key: key(k), seq, is_tombstone: false }
    }

    fn del(k: u8, seq: u64) -> TombstoneRecord {
        TombstoneRecord { key: key(k), seq, is_tombstone: true }
    }

    fn valid_sequence() -> Vec<TombstoneRecord> {
        vec![
            put(0x01, 1),
            put(0x02, 2),
            del(0x01, 3),
            put(0x03, 4),
            del(0x02, 5),
        ]
    }

    /// **SRTG-01** — resurrection rejected.
    #[test]
    fn srtg_01_resurrected_rejected() {
        let es = vec![
            put(0x01, 1),
            del(0x01, 2),
            put(0x01, 3),
        ];
        assert_eq!(
            validate_tombstone_resurrection(&es),
            Err(TombstoneResurrectionError::Resurrected { idx: 2 })
        );
    }

    /// **SRTG-02** — zero key rejected.
    #[test]
    fn srtg_02_zero_key_rejected() {
        let e = TombstoneRecord { key: [0u8; SRTG_KEY_LEN], seq: 1, is_tombstone: false };
        assert_eq!(
            validate_tombstone_resurrection(&[e]),
            Err(TombstoneResurrectionError::ZeroKey(0))
        );
    }

    /// **SRTG-03** — duplicate tombstone rejected.
    #[test]
    fn srtg_03_duplicate_tombstone_rejected() {
        let es = vec![
            put(0x01, 1),
            del(0x01, 2),
            put(0x02, 3),
            del(0x01, 4),
        ];
        assert_eq!(
            validate_tombstone_resurrection(&es),
            Err(TombstoneResurrectionError::DuplicateTombstone { idx: 3 })
        );
    }

    /// **SRTG-04** — non-increasing seq rejected.
    #[test]
    fn srtg_04_non_increasing_rejected() {
        let es = vec![
            put(0x01, 1),
            put(0x02, 1),
        ];
        assert_eq!(
            validate_tombstone_resurrection(&es),
            Err(TombstoneResurrectionError::NonIncreasing { idx: 1, got: 1, prev: 1 })
        );
    }

    /// **SRTG-05** — first seq not 1 rejected.
    #[test]
    fn srtg_05_first_seq_not_one_rejected() {
        let e = put(0x01, 5);
        assert_eq!(
            validate_tombstone_resurrection(&[e]),
            Err(TombstoneResurrectionError::FirstSeqNotOne { got: 5 })
        );
    }

    /// **SRTG-06** — too many rejected.
    #[test]
    fn srtg_06_too_many_rejected() {
        let es: Vec<TombstoneRecord> = (0..=SRTG_MAX_ENTRIES)
            .map(|i| {
                let mut k = [0u8; SRTG_KEY_LEN];
                let val = (i as u64) + 1;
                k[0..8].copy_from_slice(&val.to_be_bytes());
                TombstoneRecord { key: k, seq: (i as u64) + 1, is_tombstone: false }
            })
            .collect();
        assert_eq!(
            validate_tombstone_resurrection(&es),
            Err(TombstoneResurrectionError::TooMany {
                got: SRTG_MAX_ENTRIES + 1,
                max: SRTG_MAX_ENTRIES,
            })
        );
    }

    /// **SRTG-07** — valid accepted.
    #[test]
    fn srtg_07_valid_accepted() {
        assert_eq!(validate_tombstone_resurrection(&valid_sequence()), Ok(()));
    }

    /// **SRTG-08** — empty accepted.
    #[test]
    fn srtg_08_empty_accepted() {
        assert_eq!(validate_tombstone_resurrection(&[]), Ok(()));
    }

    /// **SRTG-09** — tombstone then different key accepted.
    #[test]
    fn srtg_09_tombstone_then_different_accepted() {
        let es = vec![
            del(0x01, 1),
            put(0x02, 2),
        ];
        assert_eq!(validate_tombstone_resurrection(&es), Ok(()));
    }

    /// **SRTG-10** — multiple tombstones different keys accepted.
    #[test]
    fn srtg_10_multiple_tombstones_accepted() {
        let es = vec![
            put(0x01, 1),
            put(0x02, 2),
            put(0x03, 3),
            del(0x01, 4),
            del(0x02, 5),
            put(0x04, 6),
        ];
        assert_eq!(validate_tombstone_resurrection(&es), Ok(()));
    }
}
