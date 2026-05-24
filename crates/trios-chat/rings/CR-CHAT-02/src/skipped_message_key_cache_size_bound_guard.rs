//! # CR-CHAT-02 — Skipped message key cache size bound guard (Wave-111 Lane B)
//!
//! RATCHET — skipped message key cache must be bounded.
//!
//! When messages arrive out of order, the ratchet stores "skipped"
//! message keys in a cache for later use. Without bounds:
//!
//! * **Memory exhaustion** — an adversary sends message N then N+M
//!   where M is enormous, forcing M skipped keys to be cached.
//! * **CPU amplification** — each cached key requires a full HKDF
//!   derivation; caching millions of keys burns CPU.
//! * **Slow lookup** — a large cache makes skipped-key lookup O(n)
//!   instead of O(1), degrading message processing latency.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cache size <= `SMKB_MAX_CACHE_SIZE`.
//! 2. Per-chain cache <= `SMKB_MAX_PER_CHAIN`.
//! 3. Chain ID must not be zero.
//! 4. No duplicate (chain, step) pairs.
//! 5. Step must be > 0.
//! 6. Total entries <= `SMKB_MAX_ENTRIES`.
//!
//! Tests **SMKB-01..10**. Error enum [`CacheBoundError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CACHE-BOUND`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum total cache size.
pub const SMKB_MAX_CACHE_SIZE: usize = 1024;

/// Maximum per-chain entries.
pub const SMKB_MAX_PER_CHAIN: usize = 64;

/// Maximum total entries in batch.
pub const SMKB_MAX_ENTRIES: usize = 2048;

/// Chain ID length.
pub const SMKB_CHAIN_ID_LEN: usize = 16;

/// A skipped key cache entry.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Chain identifier.
    pub chain_id: [u8; SMKB_CHAIN_ID_LEN],
    /// Step number within the chain.
    pub step: u64,
}

/// All ways cache bound validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CacheBoundError {
    /// Cache size exceeded.
    CacheExceeded { total: usize, max: usize },
    /// Per-chain limit exceeded.
    PerChainExceeded { chain_idx: usize, count: usize, max: usize },
    /// Zero chain ID.
    ZeroChain(usize),
    /// Duplicate entry.
    DuplicateEntry(usize),
    /// Zero step.
    ZeroStep(usize),
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate skipped message key cache size bounds.
pub fn validate_cache_bounds(entries: &[CacheEntry]) -> Result<(), CacheBoundError> {
    if entries.len() > SMKB_MAX_ENTRIES {
        return Err(CacheBoundError::TooMany {
            got: entries.len(),
            max: SMKB_MAX_ENTRIES,
        });
    }
    if entries.len() > SMKB_MAX_CACHE_SIZE {
        return Err(CacheBoundError::CacheExceeded {
            total: entries.len(),
            max: SMKB_MAX_CACHE_SIZE,
        });
    }
    let mut seen: BTreeSet<([u8; SMKB_CHAIN_ID_LEN], u64)> = BTreeSet::new();
    let mut chain_counts: BTreeMap<[u8; SMKB_CHAIN_ID_LEN], usize> = BTreeMap::new();
    for (i, e) in entries.iter().enumerate() {
        if e.chain_id == [0u8; SMKB_CHAIN_ID_LEN] {
            return Err(CacheBoundError::ZeroChain(i));
        }
        if e.step == 0 {
            return Err(CacheBoundError::ZeroStep(i));
        }
        if !seen.insert((e.chain_id, e.step)) {
            return Err(CacheBoundError::DuplicateEntry(i));
        }
        let count = chain_counts.entry(e.chain_id).or_insert(0);
        *count += 1;
        if *count > SMKB_MAX_PER_CHAIN {
            return Err(CacheBoundError::PerChainExceeded {
                chain_idx: i,
                count: *count,
                max: SMKB_MAX_PER_CHAIN,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; SMKB_CHAIN_ID_LEN] {
        [byte; SMKB_CHAIN_ID_LEN]
    }

    fn entry(chain: u8, step: u64) -> CacheEntry {
        CacheEntry { chain_id: cid(chain), step }
    }

    fn valid_entries() -> Vec<CacheEntry> {
        vec![
            entry(0x01, 1),
            entry(0x01, 2),
            entry(0x01, 3),
            entry(0x02, 1),
            entry(0x02, 2),
        ]
    }

    /// **SMKB-01** — cache exceeded rejected.
    #[test]
    fn smkb_01_cache_exceeded_rejected() {
        let es: Vec<CacheEntry> = (0..=SMKB_MAX_CACHE_SIZE)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                CacheEntry { chain_id: cid(b), step: (i as u64) + 1 }
            })
            .collect();
        assert!(matches!(
            validate_cache_bounds(&es),
            Err(CacheBoundError::CacheExceeded { .. })
        ));
    }

    /// **SMKB-02** — per-chain exceeded rejected.
    #[test]
    fn smkb_02_per_chain_rejected() {
        let es: Vec<CacheEntry> = (0..=SMKB_MAX_PER_CHAIN)
            .map(|i| CacheEntry { chain_id: cid(0x01), step: (i as u64) + 1 })
            .collect();
        assert!(matches!(
            validate_cache_bounds(&es),
            Err(CacheBoundError::PerChainExceeded { .. })
        ));
    }

    /// **SMKB-03** — zero chain rejected.
    #[test]
    fn smkb_03_zero_chain_rejected() {
        let e = CacheEntry { chain_id: [0u8; SMKB_CHAIN_ID_LEN], step: 1 };
        assert_eq!(
            validate_cache_bounds(&[e]),
            Err(CacheBoundError::ZeroChain(0))
        );
    }

    /// **SMKB-04** — duplicate rejected.
    #[test]
    fn smkb_04_duplicate_rejected() {
        let es = vec![entry(0x01, 5), entry(0x01, 5)];
        assert_eq!(
            validate_cache_bounds(&es),
            Err(CacheBoundError::DuplicateEntry(1))
        );
    }

    /// **SMKB-05** — zero step rejected.
    #[test]
    fn smkb_05_zero_step_rejected() {
        let e = CacheEntry { chain_id: cid(0x01), step: 0 };
        assert_eq!(
            validate_cache_bounds(&[e]),
            Err(CacheBoundError::ZeroStep(0))
        );
    }

    /// **SMKB-06** — too many rejected.
    #[test]
    fn smkb_06_too_many_rejected() {
        let es: Vec<CacheEntry> = (0..=SMKB_MAX_ENTRIES)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                CacheEntry { chain_id: cid(b), step: (i as u64) + 1 }
            })
            .collect();
        assert_eq!(
            validate_cache_bounds(&es),
            Err(CacheBoundError::TooMany {
                got: SMKB_MAX_ENTRIES + 1,
                max: SMKB_MAX_ENTRIES,
            })
        );
    }

    /// **SMKB-07** — valid accepted.
    #[test]
    fn smkb_07_valid_accepted() {
        assert_eq!(validate_cache_bounds(&valid_entries()), Ok(()));
    }

    /// **SMKB-08** — empty accepted.
    #[test]
    fn smkb_08_empty_accepted() {
        assert_eq!(validate_cache_bounds(&[]), Ok(()));
    }

    /// **SMKB-09** — single accepted.
    #[test]
    fn smkb_09_single_accepted() {
        let es = vec![entry(0x01, 1)];
        assert_eq!(validate_cache_bounds(&es), Ok(()));
    }

    /// **SMKB-10** — per-chain boundary accepted.
    #[test]
    fn smkb_10_per_chain_boundary_accepted() {
        let es: Vec<CacheEntry> = (0..SMKB_MAX_PER_CHAIN)
            .map(|i| CacheEntry { chain_id: cid(0x01), step: (i as u64) + 1 })
            .collect();
        assert_eq!(validate_cache_bounds(&es), Ok(()));
    }
}
