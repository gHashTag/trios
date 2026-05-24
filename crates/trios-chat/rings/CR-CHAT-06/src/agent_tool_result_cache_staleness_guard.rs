//! # CR-CHAT-06 — Agent tool result cache staleness guard (Wave-127 Lane B)
//!
//! AGENT SAFETY — cached tool results must not exceed a maximum age;
//! stale cache results may contain outdated security-relevant data.
//!
//! Caching tool results improves performance, but stale data is dangerous:
//!
//! * **Permission drift** — a cached "allowed" result may be stale
//!   after the user revokes the permission.
//! * **Data staleness** — file contents or API responses may have
//!   changed since caching, leading to decisions based on old data.
//! * **Security bypass** — an attacker who can read the cache can
//!   replay stale results to bypass security checks.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cache age must be <= `ATRC_MAX_AGE_MS`.
//! 2. Cache entry ID must not be zero.
//! 3. No duplicate entry IDs.
//! 4. Tool name must not be empty.
//! 5. Result hash must not be zero.
//! 6. Total entries <= `ATRC_MAX_ENTRIES`.
//!
//! Tests **ATRC-01..10**. Error enum [`CacheStalenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CACHE-FRESH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum cache age in milliseconds.
pub const ATRC_MAX_AGE_MS: u64 = 300_000;

/// Maximum cache entries per batch.
pub const ATRC_MAX_ENTRIES: usize = 1024;

/// Entry ID length.
pub const ATRC_ENTRY_ID_LEN: usize = 32;

/// Result hash length.
pub const ATRC_HASH_LEN: usize = 32;

/// A cached tool result entry.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cache entry identifier.
    pub entry_id: [u8; ATRC_ENTRY_ID_LEN],
    /// Tool name that produced this result.
    pub tool_name: String,
    /// Hash of the cached result.
    pub result_hash: [u8; ATRC_HASH_LEN],
    /// Age of the cache entry in milliseconds.
    pub age_ms: u64,
}

/// All ways cache staleness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheStalenessError {
    /// Cache entry too old.
    TooOld { idx: usize, age_ms: u64, max: u64 },
    /// Zero entry ID.
    ZeroEntryId(usize),
    /// Duplicate entry ID.
    DuplicateEntryId { idx: usize },
    /// Empty tool name.
    EmptyToolName(usize),
    /// Zero result hash.
    ZeroResultHash(usize),
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent tool result cache staleness.
pub fn validate_cache_staleness(
    entries: &[CacheEntry],
) -> Result<(), CacheStalenessError> {
    if entries.len() > ATRC_MAX_ENTRIES {
        return Err(CacheStalenessError::TooMany {
            got: entries.len(),
            max: ATRC_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; ATRC_ENTRY_ID_LEN]> = BTreeSet::new();
    for (i, e) in entries.iter().enumerate() {
        if e.entry_id == [0u8; ATRC_ENTRY_ID_LEN] {
            return Err(CacheStalenessError::ZeroEntryId(i));
        }
        if !seen.insert(e.entry_id) {
            return Err(CacheStalenessError::DuplicateEntryId { idx: i });
        }
        if e.tool_name.is_empty() {
            return Err(CacheStalenessError::EmptyToolName(i));
        }
        if e.result_hash == [0u8; ATRC_HASH_LEN] {
            return Err(CacheStalenessError::ZeroResultHash(i));
        }
        if e.age_ms > ATRC_MAX_AGE_MS {
            return Err(CacheStalenessError::TooOld {
                idx: i,
                age_ms: e.age_ms,
                max: ATRC_MAX_AGE_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(byte: u8) -> [u8; ATRC_ENTRY_ID_LEN] {
        [byte; ATRC_ENTRY_ID_LEN]
    }

    fn rhash(byte: u8) -> [u8; ATRC_HASH_LEN] {
        [byte; ATRC_HASH_LEN]
    }

    fn entry(id: u8, tool: &str, hash: u8, age: u64) -> CacheEntry {
        CacheEntry { entry_id: eid(id), tool_name: tool.to_string(), result_hash: rhash(hash), age_ms: age }
    }

    fn valid_cache() -> Vec<CacheEntry> {
        vec![
            entry(0x01, "read_file", 0xA1, 1000),
            entry(0x02, "list_dir", 0xA2, 5000),
            entry(0x03, "search", 0xA3, ATRC_MAX_AGE_MS),
        ]
    }

    /// **ATRC-01** — too old rejected.
    #[test]
    fn atrc_01_too_old_rejected() {
        let es = vec![entry(0x01, "read_file", 0xAA, ATRC_MAX_AGE_MS + 1)];
        assert_eq!(
            validate_cache_staleness(&es),
            Err(CacheStalenessError::TooOld {
                idx: 0,
                age_ms: ATRC_MAX_AGE_MS + 1,
                max: ATRC_MAX_AGE_MS,
            })
        );
    }

    /// **ATRC-02** — zero entry ID rejected.
    #[test]
    fn atrc_02_zero_entry_rejected() {
        let e = CacheEntry { entry_id: [0u8; ATRC_ENTRY_ID_LEN], tool_name: "read_file".to_string(), result_hash: rhash(0xAA), age_ms: 1000 };
        assert_eq!(
            validate_cache_staleness(&[e]),
            Err(CacheStalenessError::ZeroEntryId(0))
        );
    }

    /// **ATRC-03** — duplicate entry ID rejected.
    #[test]
    fn atrc_03_duplicate_rejected() {
        let es = vec![
            entry(0x01, "read_file", 0xA1, 1000),
            entry(0x01, "list_dir", 0xA2, 2000),
        ];
        assert_eq!(
            validate_cache_staleness(&es),
            Err(CacheStalenessError::DuplicateEntryId { idx: 1 })
        );
    }

    /// **ATRC-04** — empty tool name rejected.
    #[test]
    fn atrc_04_empty_tool_rejected() {
        let e = CacheEntry { entry_id: eid(0x01), tool_name: String::new(), result_hash: rhash(0xAA), age_ms: 1000 };
        assert_eq!(
            validate_cache_staleness(&[e]),
            Err(CacheStalenessError::EmptyToolName(0))
        );
    }

    /// **ATRC-05** — zero result hash rejected.
    #[test]
    fn atrc_05_zero_hash_rejected() {
        let e = CacheEntry { entry_id: eid(0x01), tool_name: "read_file".to_string(), result_hash: [0u8; ATRC_HASH_LEN], age_ms: 1000 };
        assert_eq!(
            validate_cache_staleness(&[e]),
            Err(CacheStalenessError::ZeroResultHash(0))
        );
    }

    /// **ATRC-06** — too many rejected.
    #[test]
    fn atrc_06_too_many_rejected() {
        let es: Vec<CacheEntry> = (0..=ATRC_MAX_ENTRIES)
            .map(|i| {
                let mut id = [0u8; ATRC_ENTRY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut h = [0u8; ATRC_HASH_LEN];
                h[0] = (i as u8).wrapping_add(1);
                CacheEntry { entry_id: id, tool_name: "tool".to_string(), result_hash: h, age_ms: 1000 }
            })
            .collect();
        assert_eq!(
            validate_cache_staleness(&es),
            Err(CacheStalenessError::TooMany {
                got: ATRC_MAX_ENTRIES + 1,
                max: ATRC_MAX_ENTRIES,
            })
        );
    }

    /// **ATRC-07** — valid accepted.
    #[test]
    fn atrc_07_valid_accepted() {
        assert_eq!(validate_cache_staleness(&valid_cache()), Ok(()));
    }

    /// **ATRC-08** — empty accepted.
    #[test]
    fn atrc_08_empty_accepted() {
        assert_eq!(validate_cache_staleness(&[]), Ok(()));
    }

    /// **ATRC-09** — fresh entry accepted.
    #[test]
    fn atrc_09_fresh_accepted() {
        let es = vec![entry(0x01, "read_file", 0xAA, 0)];
        assert_eq!(validate_cache_staleness(&es), Ok(()));
    }

    /// **ATRC-10** — boundary age accepted.
    #[test]
    fn atrc_10_boundary_age_accepted() {
        let es = vec![entry(0x01, "read_file", 0xAA, ATRC_MAX_AGE_MS)];
        assert_eq!(validate_cache_staleness(&es), Ok(()));
    }
}
