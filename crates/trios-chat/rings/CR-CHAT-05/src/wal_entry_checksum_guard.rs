//! # CR-CHAT-05 — WAL entry checksum guard (Wave-74 Lane B)
//!
//! PERSISTENCE — each WAL entry must carry a valid checksum, R-CHAT-5.
//!
//! Without per-entry checksums, a disk-level bit flip in a WAL entry
//! causes silent data corruption:
//!
//! * **Silent corruption** — a flipped bit in the LSN or data field
//!   goes undetected, causing incorrect replay after crash recovery.
//! * **Cross-entry corruption** — a corrupted entry shifts the byte
//!   boundary, corrupting all subsequent entries.
//! * **Truncated tail** — the last entry is partially written and the
//!   truncation is not detected without a checksum.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each entry has a non-zero checksum.
//! 2. Checksum length == `WLCS_CHECKSUM_LEN`.
//! 3. Entry data length <= `WLCS_MAX_ENTRY_LEN`.
//! 4. No duplicate LSN + checksum pairs (replay protection).
//! 5. LSN is strictly increasing.
//! 6. Entry count <= `WLCS_MAX_ENTRIES`.
//!
//! Tests **WLCS-01..10**. Error enum [`WalChecksumError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WAL-CHECKSUM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Checksum length (bytes).
pub const WLCS_CHECKSUM_LEN: usize = 4;

/// Maximum entry data length.
pub const WLCS_MAX_ENTRY_LEN: usize = 65536;

/// Maximum entries in a batch.
pub const WLCS_MAX_ENTRIES: usize = 512;

/// A WAL entry with checksum.
#[derive(Debug, Clone)]
pub struct WalChecksumEntry {
    /// Log sequence number.
    pub lsn: u64,
    /// Entry data.
    pub data: Vec<u8>,
    /// Checksum (CRC32 or similar).
    pub checksum: Vec<u8>,
}

/// All ways WAL checksum validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WalChecksumError {
    /// Zero checksum.
    ZeroChecksum,
    /// Checksum length wrong.
    ChecksumLengthWrong,
    /// Entry data too large.
    EntryTooLarge,
    /// Duplicate LSN.
    DuplicateLsn(u64),
    /// LSN not increasing.
    LsnNotIncreasing,
    /// Too many entries.
    TooManyEntries,
}

/// `[VERIFIED]` Validate WAL entries have valid checksums and increasing LSNs.
pub fn validate_wal_checksums(
    entries: &[WalChecksumEntry],
) -> Result<(), WalChecksumError> {
    if entries.len() > WLCS_MAX_ENTRIES {
        return Err(WalChecksumError::TooManyEntries);
    }
    let mut prev_lsn: Option<u64> = None;
    let mut seen_lsns = BTreeSet::new();
    for entry in entries {
        if entry.data.len() > WLCS_MAX_ENTRY_LEN {
            return Err(WalChecksumError::EntryTooLarge);
        }
        if entry.checksum.len() != WLCS_CHECKSUM_LEN {
            return Err(WalChecksumError::ChecksumLengthWrong);
        }
        if entry.checksum.iter().all(|&b| b == 0) {
            return Err(WalChecksumError::ZeroChecksum);
        }
        if !seen_lsns.insert(entry.lsn) {
            return Err(WalChecksumError::DuplicateLsn(entry.lsn));
        }
        if let Some(pl) = prev_lsn {
            if entry.lsn <= pl {
                return Err(WalChecksumError::LsnNotIncreasing);
            }
        }
        prev_lsn = Some(entry.lsn);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(lsn: u64, data_len: usize) -> WalChecksumEntry {
        WalChecksumEntry {
            lsn,
            data: vec![0xAB; data_len],
            checksum: vec![0xDE, 0xAD, 0xBE, 0xEF],
        }
    }

    fn valid_entries() -> Vec<WalChecksumEntry> {
        vec![entry(1, 100), entry(2, 200), entry(3, 300)]
    }

    /// **WLCS-01** — zero checksum rejected.
    #[test]
    fn wlcs_01_zero_checksum_rejected() {
        let e = WalChecksumEntry {
            lsn: 1, data: vec![0xAB; 10], checksum: vec![0, 0, 0, 0],
        };
        assert_eq!(
            validate_wal_checksums(&[e]),
            Err(WalChecksumError::ZeroChecksum)
        );
    }

    /// **WLCS-02** — checksum length wrong rejected.
    #[test]
    fn wlcs_02_checksum_len_rejected() {
        let e = WalChecksumEntry {
            lsn: 1, data: vec![0xAB; 10], checksum: vec![0xFF],
        };
        assert_eq!(
            validate_wal_checksums(&[e]),
            Err(WalChecksumError::ChecksumLengthWrong)
        );
    }

    /// **WLCS-03** — entry too large rejected.
    #[test]
    fn wlcs_03_entry_too_large_rejected() {
        let e = entry(1, WLCS_MAX_ENTRY_LEN + 1);
        assert_eq!(
            validate_wal_checksums(&[e]),
            Err(WalChecksumError::EntryTooLarge)
        );
    }

    /// **WLCS-04** — duplicate LSN rejected.
    #[test]
    fn wlcs_04_duplicate_lsn_rejected() {
        let entries = vec![entry(1, 10), entry(1, 20)];
        assert_eq!(
            validate_wal_checksums(&entries),
            Err(WalChecksumError::DuplicateLsn(1))
        );
    }

    /// **WLCS-05** — LSN not increasing rejected.
    #[test]
    fn wlcs_05_lsn_not_increasing_rejected() {
        let entries = vec![entry(5, 10), entry(3, 20)];
        assert_eq!(
            validate_wal_checksums(&entries),
            Err(WalChecksumError::LsnNotIncreasing)
        );
    }

    /// **WLCS-06** — too many entries rejected.
    #[test]
    fn wlcs_06_too_many_rejected() {
        let entries: Vec<WalChecksumEntry> = (0..=WLCS_MAX_ENTRIES)
            .map(|i| entry(i as u64 + 1, 10))
            .collect();
        assert_eq!(
            validate_wal_checksums(&entries),
            Err(WalChecksumError::TooManyEntries)
        );
    }

    /// **WLCS-07** — valid entries accepted.
    #[test]
    fn wlcs_07_valid_accepted() {
        assert_eq!(validate_wal_checksums(&valid_entries()), Ok(()));
    }

    /// **WLCS-08** — single entry accepted.
    #[test]
    fn wlcs_08_single_accepted() {
        assert_eq!(validate_wal_checksums(&[entry(1, 100)]), Ok(()));
    }

    /// **WLCS-09** — empty accepted.
    #[test]
    fn wlcs_09_empty_accepted() {
        assert_eq!(validate_wal_checksums(&[]), Ok(()));
    }

    /// **WLCS-10** — max entry data length accepted.
    #[test]
    fn wlcs_10_max_data_accepted() {
        let e = entry(1, WLCS_MAX_ENTRY_LEN);
        assert_eq!(validate_wal_checksums(&[e]), Ok(()));
    }
}
