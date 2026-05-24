//! # CR-CHAT-05 — Store WAL entry sequence continuity guard (Wave-126 Lane B)
//!
//! PERSISTENCE — WAL entries must form a contiguous sequence; gaps
//! in the WAL indicate lost or tampered entries.
//!
//! The Write-Ahead Log (WAL) is the authoritative record of all store
//! mutations. If entries are missing from the sequence:
//!
//! * **Silent data loss** — a gap in the WAL means writes between the
//!   gap boundaries are unrecoverable after a crash.
//! * **Tampering evidence** — an attacker deleting WAL entries to
//!   hide modifications leaves sequence gaps.
//! * **Recovery failure** — WAL replay during recovery stops at the
//!   first gap, losing all subsequent entries.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Sequence numbers must start at `WSCG_GENESIS`.
//! 2. Sequence numbers must be contiguous (no gaps).
//! 3. Entry hash must not be zero.
//! 4. Entry hash must chain to previous entry's hash.
//! 5. No duplicate sequence numbers.
//! 6. Total entries <= `WSCG_MAX_ENTRIES`.
//!
//! Tests **WSCG-01..10**. Error enum [`WalSequenceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WAL-CONTIGUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Genesis sequence number.
pub const WSCG_GENESIS: u64 = 1;

/// Maximum entries per batch.
pub const WSCG_MAX_ENTRIES: usize = 4096;

/// Hash length.
pub const WSCG_HASH_LEN: usize = 32;

/// A WAL entry in the sequence.
#[derive(Debug, Clone)]
pub struct WalSequenceEntry {
    /// Sequence number.
    pub seq: u64,
    /// Hash of this entry.
    pub entry_hash: [u8; WSCG_HASH_LEN],
    /// Hash of the previous entry (zero for genesis).
    pub prev_hash: [u8; WSCG_HASH_LEN],
}

/// All ways WAL sequence validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WalSequenceError {
    /// First entry not at genesis.
    NotGenesis {
        /// Index of the offending entry.
        idx: usize,
        /// Sequence number found.
        seq: u64,
        /// Expected genesis sequence.
        expected: u64,
    },
    /// Gap in sequence.
    Gap {
        /// Index of the gap.
        idx: usize,
        /// Expected sequence number.
        expected: u64,
        /// Found sequence number.
        found: u64,
    },
    /// Zero entry hash.
    ZeroHash(usize),
    /// Hash chain broken.
    ChainBroken {
        /// Index of the broken entry.
        idx: usize,
        /// Expected previous hash.
        expected_prev: [u8; WSCG_HASH_LEN],
        /// Found previous hash.
        found_prev: [u8; WSCG_HASH_LEN],
    },
    /// Duplicate sequence.
    DuplicateSeq {
        /// Index of the duplicate.
        idx: usize,
        /// Duplicate sequence number.
        seq: u64,
    },
    /// Too many entries.
    TooMany {
        /// Count received.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store WAL entry sequence continuity.
pub fn validate_wal_sequence(
    entries: &[WalSequenceEntry],
) -> Result<(), WalSequenceError> {
    if entries.len() > WSCG_MAX_ENTRIES {
        return Err(WalSequenceError::TooMany {
            got: entries.len(),
            max: WSCG_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut prev_hash: [u8; WSCG_HASH_LEN] = [0u8; WSCG_HASH_LEN];
    for (i, e) in entries.iter().enumerate() {
        if e.entry_hash == [0u8; WSCG_HASH_LEN] {
            return Err(WalSequenceError::ZeroHash(i));
        }
        if i == 0 {
            if e.seq != WSCG_GENESIS {
                return Err(WalSequenceError::NotGenesis {
                    idx: 0,
                    seq: e.seq,
                    expected: WSCG_GENESIS,
                });
            }
        } else {
            let expected_seq = entries[i - 1].seq + 1;
            if e.seq != expected_seq {
                return Err(WalSequenceError::Gap {
                    idx: i,
                    expected: expected_seq,
                    found: e.seq,
                });
            }
            if e.prev_hash != prev_hash {
                return Err(WalSequenceError::ChainBroken {
                    idx: i,
                    expected_prev: prev_hash,
                    found_prev: e.prev_hash,
                });
            }
        }
        if !seen.insert(e.seq) {
            return Err(WalSequenceError::DuplicateSeq { idx: i, seq: e.seq });
        }
        prev_hash = e.entry_hash;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; WSCG_HASH_LEN] {
        [byte; WSCG_HASH_LEN]
    }

    fn entry(seq: u64, h: u8, prev: u8) -> WalSequenceEntry {
        WalSequenceEntry { seq, entry_hash: hash(h), prev_hash: hash(prev) }
    }

    fn valid_chain() -> Vec<WalSequenceEntry> {
        vec![
            WalSequenceEntry { seq: 1, entry_hash: hash(0xA1), prev_hash: [0u8; WSCG_HASH_LEN] },
            WalSequenceEntry { seq: 2, entry_hash: hash(0xA2), prev_hash: hash(0xA1) },
            WalSequenceEntry { seq: 3, entry_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ]
    }

    /// **WSCG-01** — not genesis rejected.
    #[test]
    fn wscg_01_not_genesis_rejected() {
        let es = vec![WalSequenceEntry { seq: 5, entry_hash: hash(0xA1), prev_hash: [0u8; WSCG_HASH_LEN] }];
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::NotGenesis { idx: 0, seq: 5, expected: WSCG_GENESIS })
        );
    }

    /// **WSCG-02** — gap rejected.
    #[test]
    fn wscg_02_gap_rejected() {
        let es = vec![
            WalSequenceEntry { seq: 1, entry_hash: hash(0xA1), prev_hash: [0u8; WSCG_HASH_LEN] },
            WalSequenceEntry { seq: 5, entry_hash: hash(0xA2), prev_hash: hash(0xA1) },
        ];
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::Gap { idx: 1, expected: 2, found: 5 })
        );
    }

    /// **WSCG-03** — zero hash rejected.
    #[test]
    fn wscg_03_zero_hash_rejected() {
        let es = vec![WalSequenceEntry { seq: 1, entry_hash: [0u8; WSCG_HASH_LEN], prev_hash: [0u8; WSCG_HASH_LEN] }];
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::ZeroHash(0))
        );
    }

    /// **WSCG-04** — chain broken rejected.
    #[test]
    fn wscg_04_chain_broken_rejected() {
        let es = vec![
            WalSequenceEntry { seq: 1, entry_hash: hash(0xA1), prev_hash: [0u8; WSCG_HASH_LEN] },
            WalSequenceEntry { seq: 2, entry_hash: hash(0xA2), prev_hash: hash(0xBB) },
        ];
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::ChainBroken {
                idx: 1,
                expected_prev: hash(0xA1),
                found_prev: hash(0xBB),
            })
        );
    }

    /// **WSCG-05** — duplicate sequence rejected.
    #[test]
    fn wscg_05_duplicate_seq_rejected() {
        let es = vec![
            WalSequenceEntry { seq: 1, entry_hash: hash(0xA1), prev_hash: [0u8; WSCG_HASH_LEN] },
            WalSequenceEntry { seq: 2, entry_hash: hash(0xA2), prev_hash: hash(0xA1) },
            WalSequenceEntry { seq: 2, entry_hash: hash(0xA3), prev_hash: hash(0xA2) },
        ];
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::Gap { idx: 2, expected: 3, found: 2 })
        );
    }

    /// **WSCG-06** — too many rejected.
    #[test]
    fn wscg_06_too_many_rejected() {
        let mut es: Vec<WalSequenceEntry> = Vec::with_capacity(WSCG_MAX_ENTRIES + 1);
        let mut prev = [0u8; WSCG_HASH_LEN];
        for i in 0..=WSCG_MAX_ENTRIES {
            let seq = (i as u64) + 1;
            let mut h = [0u8; WSCG_HASH_LEN];
            h[0] = (i as u8).wrapping_add(1);
            es.push(WalSequenceEntry { seq, entry_hash: h, prev_hash: prev });
            prev = h;
        }
        assert_eq!(
            validate_wal_sequence(&es),
            Err(WalSequenceError::TooMany {
                got: WSCG_MAX_ENTRIES + 1,
                max: WSCG_MAX_ENTRIES,
            })
        );
    }

    /// **WSCG-07** — valid accepted.
    #[test]
    fn wscg_07_valid_accepted() {
        assert_eq!(validate_wal_sequence(&valid_chain()), Ok(()));
    }

    /// **WSCG-08** — empty accepted.
    #[test]
    fn wscg_08_empty_accepted() {
        assert_eq!(validate_wal_sequence(&[]), Ok(()));
    }

    /// **WSCG-09** — single genesis accepted.
    #[test]
    fn wscg_09_single_genesis_accepted() {
        let es = vec![WalSequenceEntry { seq: 1, entry_hash: hash(0xAA), prev_hash: [0u8; WSCG_HASH_LEN] }];
        assert_eq!(validate_wal_sequence(&es), Ok(()));
    }

    /// **WSCG-10** — long contiguous chain accepted.
    #[test]
    fn wscg_10_long_chain_accepted() {
        let mut es = Vec::new();
        let mut prev = [0u8; WSCG_HASH_LEN];
        for i in 0..200u64 {
            let mut h = [0u8; WSCG_HASH_LEN];
            h[0..8].copy_from_slice(&(i + 1).to_be_bytes());
            es.push(WalSequenceEntry { seq: i + 1, entry_hash: h, prev_hash: prev });
            prev = h;
        }
        assert_eq!(validate_wal_sequence(&es), Ok(()));
    }
}
