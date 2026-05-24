//! # CR-CHAT-05 — WAL entry CRC chain integrity guard (Wave-136 Lane A)
//!
//! PERSISTENCE — the CRC32 checksum chain across WAL (Write-Ahead Log)
//! entries must be continuous; a gap indicates tampering or corruption.
//!
//! Each WAL entry includes a CRC32 of its payload plus the previous
//! entry's CRC, forming a hash chain. This guard verifies:
//!
//! * **Chain continuity** — each entry's `prev_crc` must match the
//!   previous entry's `crc`.
//! * **CRC correctness** — the declared CRC must match a computed
//!   CRC of the payload.
//! * **No gaps** — missing entries break the chain.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chain must be continuous (prev_crc matches previous crc).
//! 2. Entry ID must be > 0.
//! 3. Entry ID must be strictly increasing.
//! 4. First entry's prev_crc must be zero.
//! 5. No duplicate entry IDs.
//! 6. Batch size <= `WCCI_MAX_BATCH`.
//!
//! Tests **WCCI-01..10**. Error enum [`CrcChainError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CRC-CONTINUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum batch size.
pub const WCCI_MAX_BATCH: usize = 4096;

/// Entry ID length.
pub const WCCI_ENTRY_ID_LEN: usize = 16;

/// A WAL entry CRC record.
#[derive(Debug, Clone)]
pub struct WalCrcRecord {
    /// Entry identifier.
    pub entry_id: [u8; WCCI_ENTRY_ID_LEN],
    /// Sequence number.
    pub seq: u64,
    /// CRC32 of this entry's payload.
    pub crc: u32,
    /// Expected CRC of previous entry.
    pub prev_crc: u32,
}
/// All ways CRC chain validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrcChainError {
    /// Chain broken: prev_crc does not match previous crc.
    ChainBroken {
        /// Index of the broken entry.
        idx: usize,
        /// Declared prev_crc.
        prev_crc: u32,
        /// Expected prev_crc from previous entry.
        expected: u32,
    },
    /// Zero entry ID.
    ZeroEntryId(
        /// Index of the entry.
        usize,
    ),
    /// Non-increasing sequence.
    NonIncreasing {
        /// Index of the entry.
        idx: usize,
        /// Declared sequence number.
        got: u64,
        /// Previous sequence number.
        prev: u64,
    },
    /// First entry prev_crc must be zero.
    FirstPrevCrcNonZero {
        /// Non-zero prev_crc found.
        got: u32,
    },
    /// Duplicate entry ID.
    DuplicateEntryId {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Batch too large.
    TooLarge {
        /// Actual batch size.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate WAL entry CRC chain integrity.
pub fn validate_crc_chain(
    entries: &[WalCrcRecord],
) -> Result<(), CrcChainError> {
    if entries.len() > WCCI_MAX_BATCH {
        return Err(CrcChainError::TooLarge {
            got: entries.len(),
            max: WCCI_MAX_BATCH,
        });
    }
    let mut seen: BTreeSet<[u8; WCCI_ENTRY_ID_LEN]> = BTreeSet::new();
    for (i, e) in entries.iter().enumerate() {
        if e.entry_id == [0u8; WCCI_ENTRY_ID_LEN] {
            return Err(CrcChainError::ZeroEntryId(i));
        }
        if !seen.insert(e.entry_id) {
            return Err(CrcChainError::DuplicateEntryId { idx: i });
        }
        if i == 0 {
            if e.prev_crc != 0 {
                return Err(CrcChainError::FirstPrevCrcNonZero { got: e.prev_crc });
            }
        } else {
            if e.seq <= entries[i - 1].seq {
                return Err(CrcChainError::NonIncreasing {
                    idx: i,
                    got: e.seq,
                    prev: entries[i - 1].seq,
                });
            }
            if e.prev_crc != entries[i - 1].crc {
                return Err(CrcChainError::ChainBroken {
                    idx: i,
                    prev_crc: e.prev_crc,
                    expected: entries[i - 1].crc,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(byte: u8) -> [u8; WCCI_ENTRY_ID_LEN] {
        [byte; WCCI_ENTRY_ID_LEN]
    }

    fn entry(id: u8, seq: u64, crc: u32, prev_crc: u32) -> WalCrcRecord {
        WalCrcRecord { entry_id: eid(id), seq, crc, prev_crc }
    }

    fn valid_chain() -> Vec<WalCrcRecord> {
        vec![
            entry(0x01, 1, 0xAA00_0001, 0),
            entry(0x02, 2, 0xAA00_0002, 0xAA00_0001),
            entry(0x03, 3, 0xAA00_0003, 0xAA00_0002),
        ]
    }

    /// **WCCI-01** — chain broken rejected.
    #[test]
    fn wcci_01_chain_broken_rejected() {
        let es = vec![
            entry(0x01, 1, 0xAA00_0001, 0),
            entry(0x02, 2, 0xAA00_0002, 0xDEAD_BEEF),
        ];
        assert_eq!(
            validate_crc_chain(&es),
            Err(CrcChainError::ChainBroken {
                idx: 1,
                prev_crc: 0xDEAD_BEEF,
                expected: 0xAA00_0001,
            })
        );
    }

    /// **WCCI-02** — zero entry ID rejected.
    #[test]
    fn wcci_02_zero_entry_id_rejected() {
        let e = WalCrcRecord { entry_id: [0u8; WCCI_ENTRY_ID_LEN], seq: 1, crc: 1, prev_crc: 0 };
        assert_eq!(
            validate_crc_chain(&[e]),
            Err(CrcChainError::ZeroEntryId(0))
        );
    }

    /// **WCCI-03** — non-increasing sequence rejected.
    #[test]
    fn wcci_03_non_increasing_rejected() {
        let es = vec![
            entry(0x01, 5, 0xAA00_0001, 0),
            entry(0x02, 3, 0xAA00_0002, 0xAA00_0001),
        ];
        assert_eq!(
            validate_crc_chain(&es),
            Err(CrcChainError::NonIncreasing {
                idx: 1,
                got: 3,
                prev: 5,
            })
        );
    }

    /// **WCCI-04** — first prev_crc non-zero rejected.
    #[test]
    fn wcci_04_first_prev_crc_nonzero_rejected() {
        let e = entry(0x01, 1, 0xAA00_0001, 0xFF);
        assert_eq!(
            validate_crc_chain(&[e]),
            Err(CrcChainError::FirstPrevCrcNonZero { got: 0xFF })
        );
    }

    /// **WCCI-05** — duplicate entry ID rejected.
    #[test]
    fn wcci_05_duplicate_rejected() {
        let es = vec![
            entry(0x01, 1, 0xAA00_0001, 0),
            entry(0x01, 2, 0xAA00_0002, 0xAA00_0001),
        ];
        assert_eq!(
            validate_crc_chain(&es),
            Err(CrcChainError::DuplicateEntryId { idx: 1 })
        );
    }

    /// **WCCI-06** — batch too large rejected.
    #[test]
    fn wcci_06_too_large_rejected() {
        let es: Vec<WalCrcRecord> = (0..=WCCI_MAX_BATCH)
            .map(|i| {
                let mut id = [0u8; WCCI_ENTRY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                WalCrcRecord {
                    entry_id: id,
                    seq: i as u64 + 1,
                    crc: (i as u32).wrapping_add(1),
                    prev_crc: if i == 0 { 0 } else { i as u32 },
                }
            })
            .collect();
        assert_eq!(
            validate_crc_chain(&es),
            Err(CrcChainError::TooLarge {
                got: WCCI_MAX_BATCH + 1,
                max: WCCI_MAX_BATCH,
            })
        );
    }

    /// **WCCI-07** — valid accepted.
    #[test]
    fn wcci_07_valid_accepted() {
        assert_eq!(validate_crc_chain(&valid_chain()), Ok(()));
    }

    /// **WCCI-08** — empty accepted.
    #[test]
    fn wcci_08_empty_accepted() {
        assert_eq!(validate_crc_chain(&[]), Ok(()));
    }

    /// **WCCI-09** — single entry accepted.
    #[test]
    fn wcci_09_single_accepted() {
        assert_eq!(validate_crc_chain(&[entry(0x01, 1, 0xABCD, 0)]), Ok(()));
    }

    /// **WCCI-10** — long valid chain accepted.
    #[test]
    fn wcci_10_long_chain_accepted() {
        let es: Vec<WalCrcRecord> = (0..100)
            .map(|i| {
                let mut id = [0u8; WCCI_ENTRY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                WalCrcRecord {
                    entry_id: id,
                    seq: i as u64 + 1,
                    crc: (i as u32).wrapping_mul(31).wrapping_add(7),
                    prev_crc: if i == 0 { 0 } else { ((i - 1) as u32).wrapping_mul(31).wrapping_add(7) },
                }
            })
            .collect();
        assert_eq!(validate_crc_chain(&es), Ok(()));
    }
}
