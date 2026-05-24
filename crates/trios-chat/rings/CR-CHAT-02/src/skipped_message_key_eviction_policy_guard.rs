//! # CR-CHAT-02 — Skipped message key eviction policy guard (Wave-133 Lane B)
//!
//! RATCHET — skipped message keys must be evicted after use or
//! timeout; retained skipped keys enable decryption of future
//! messages if compromised.
//!
//! The Double Ratchet stores skipped message keys for out-of-order
//! message processing. These keys must be evicted:
//!
//! * **Key compromise** — a retained skipped key, if compromised,
//!   allows decryption of the corresponding message even after
//!   the legitimate recipient has deleted it.
//! * **Storage exhaustion** — unbounded skipped key storage
//!   enables a DoS attack where the sender skips many messages.
//! * **Forward secrecy** — skipped keys should be deleted after
//!   use to maintain forward secrecy for that message.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Skipped key count per chain <= `SMEP_MAX_PER_CHAIN`.
//! 2. Total skipped keys <= `SMEP_MAX_TOTAL`.
//! 3. Key age must be <= `SMEP_MAX_AGE_MS`.
//! 4. Key hash must not be zero.
//! 5. Chain ID must not be zero.
//! 6. Total records <= `SMEP_MAX_RECORDS`.
//!
//! Tests **SMEP-01..10**. Error enum [`EvictionPolicyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EVICT-PROMPT`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Maximum skipped keys per chain.
pub const SMEP_MAX_PER_CHAIN: usize = 10;

/// Maximum total skipped keys.
pub const SMEP_MAX_TOTAL: usize = 100;

/// Maximum key age in milliseconds.
pub const SMEP_MAX_AGE_MS: u64 = 60_000;

/// Maximum records per batch.
pub const SMEP_MAX_RECORDS: usize = 512;

/// Chain ID length.
pub const SMEP_CHAIN_ID_LEN: usize = 32;

/// Key hash length.
pub const SMEP_HASH_LEN: usize = 32;

/// A skipped message key record.
#[derive(Debug, Clone)]
pub struct SkippedKeyRecord {
    /// Chain identifier.
    pub chain_id: [u8; SMEP_CHAIN_ID_LEN],
    /// Hash of the skipped key.
    pub key_hash: [u8; SMEP_HASH_LEN],
    /// Age of the key in milliseconds.
    pub age_ms: u64,
}

/// All ways eviction policy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvictionPolicyError {
    /// Too many keys per chain.
    TooManyPerChain { chain_id: [u8; SMEP_CHAIN_ID_LEN], count: usize, max: usize },
    /// Total skipped keys exceeded.
    TotalExceeded { got: usize, max: usize },
    /// Key too old.
    TooOld { idx: usize, age_ms: u64, max: u64 },
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Too many records.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate skipped message key eviction policy.
pub fn validate_eviction_policy(
    records: &[SkippedKeyRecord],
) -> Result<(), EvictionPolicyError> {
    if records.len() > SMEP_MAX_RECORDS {
        return Err(EvictionPolicyError::TooMany {
            got: records.len(),
            max: SMEP_MAX_RECORDS,
        });
    }
    if records.len() > SMEP_MAX_TOTAL {
        return Err(EvictionPolicyError::TotalExceeded {
            got: records.len(),
            max: SMEP_MAX_TOTAL,
        });
    }
    let mut chain_counts: BTreeMap<[u8; SMEP_CHAIN_ID_LEN], usize> = BTreeMap::new();
    for (i, r) in records.iter().enumerate() {
        if r.chain_id == [0u8; SMEP_CHAIN_ID_LEN] {
            return Err(EvictionPolicyError::ZeroChainId(i));
        }
        if r.key_hash == [0u8; SMEP_HASH_LEN] {
            return Err(EvictionPolicyError::ZeroKeyHash(i));
        }
        if r.age_ms > SMEP_MAX_AGE_MS {
            return Err(EvictionPolicyError::TooOld {
                idx: i,
                age_ms: r.age_ms,
                max: SMEP_MAX_AGE_MS,
            });
        }
        *chain_counts.entry(r.chain_id).or_insert(0) += 1;
    }
    for (&chain_id, &count) in &chain_counts {
        if count > SMEP_MAX_PER_CHAIN {
            return Err(EvictionPolicyError::TooManyPerChain {
                chain_id,
                count,
                max: SMEP_MAX_PER_CHAIN,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; SMEP_CHAIN_ID_LEN] {
        [byte; SMEP_CHAIN_ID_LEN]
    }

    fn khash(byte: u8) -> [u8; SMEP_HASH_LEN] {
        [byte; SMEP_HASH_LEN]
    }

    fn skipped(chain: u8, key: u8, age: u64) -> SkippedKeyRecord {
        SkippedKeyRecord { chain_id: cid(chain), key_hash: khash(key), age_ms: age }
    }

    fn valid_records() -> Vec<SkippedKeyRecord> {
        vec![
            skipped(0x01, 0xA1, 1000),
            skipped(0x01, 0xA2, 2000),
            skipped(0x02, 0xA3, 1500),
            skipped(0x03, 0xA4, 3000),
        ]
    }

    /// **SMEP-01** — too many per chain rejected.
    #[test]
    fn smep_01_too_many_per_chain_rejected() {
        let rs: Vec<SkippedKeyRecord> = (0..=SMEP_MAX_PER_CHAIN)
            .map(|i| skipped(0x01, (i as u8).wrapping_add(1), (i as u64) * 100))
            .collect();
        assert_eq!(
            validate_eviction_policy(&rs),
            Err(EvictionPolicyError::TooManyPerChain {
                chain_id: cid(0x01),
                count: SMEP_MAX_PER_CHAIN + 1,
                max: SMEP_MAX_PER_CHAIN,
            })
        );
    }

    /// **SMEP-02** — total exceeded rejected.
    #[test]
    fn smep_02_total_exceeded_rejected() {
        let rs: Vec<SkippedKeyRecord> = (0..=SMEP_MAX_TOTAL)
            .map(|i| {
                let chain = (i % 20 + 1) as u8;
                let key = (i as u8).wrapping_add(1);
                skipped(chain, key, (i as u64) * 100)
            })
            .collect();
        assert_eq!(
            validate_eviction_policy(&rs),
            Err(EvictionPolicyError::TotalExceeded {
                got: SMEP_MAX_TOTAL + 1,
                max: SMEP_MAX_TOTAL,
            })
        );
    }

    /// **SMEP-03** — too old rejected.
    #[test]
    fn smep_03_too_old_rejected() {
        let r = SkippedKeyRecord { chain_id: cid(0x01), key_hash: khash(0xAA), age_ms: SMEP_MAX_AGE_MS + 1 };
        assert_eq!(
            validate_eviction_policy(&[r]),
            Err(EvictionPolicyError::TooOld {
                idx: 0,
                age_ms: SMEP_MAX_AGE_MS + 1,
                max: SMEP_MAX_AGE_MS,
            })
        );
    }

    /// **SMEP-04** — zero key hash rejected.
    #[test]
    fn smep_04_zero_key_rejected() {
        let r = SkippedKeyRecord { chain_id: cid(0x01), key_hash: [0u8; SMEP_HASH_LEN], age_ms: 1000 };
        assert_eq!(
            validate_eviction_policy(&[r]),
            Err(EvictionPolicyError::ZeroKeyHash(0))
        );
    }

    /// **SMEP-05** — zero chain ID rejected.
    #[test]
    fn smep_05_zero_chain_rejected() {
        let r = SkippedKeyRecord { chain_id: [0u8; SMEP_CHAIN_ID_LEN], key_hash: khash(0xAA), age_ms: 1000 };
        assert_eq!(
            validate_eviction_policy(&[r]),
            Err(EvictionPolicyError::ZeroChainId(0))
        );
    }

    /// **SMEP-06** — too many records rejected.
    #[test]
    fn smep_06_too_many_rejected() {
        let rs: Vec<SkippedKeyRecord> = (0..=SMEP_MAX_RECORDS)
            .map(|i| {
                let chain = (i % 20 + 1) as u8;
                skipped(chain, (i as u8).wrapping_add(1), (i as u64) * 100)
            })
            .collect();
        assert_eq!(
            validate_eviction_policy(&rs),
            Err(EvictionPolicyError::TooMany {
                got: SMEP_MAX_RECORDS + 1,
                max: SMEP_MAX_RECORDS,
            })
        );
    }

    /// **SMEP-07** — valid accepted.
    #[test]
    fn smep_07_valid_accepted() {
        assert_eq!(validate_eviction_policy(&valid_records()), Ok(()));
    }

    /// **SMEP-08** — empty accepted.
    #[test]
    fn smep_08_empty_accepted() {
        assert_eq!(validate_eviction_policy(&[]), Ok(()));
    }

    /// **SMEP-09** — boundary age accepted.
    #[test]
    fn smep_09_boundary_age_accepted() {
        let r = SkippedKeyRecord { chain_id: cid(0x01), key_hash: khash(0xAA), age_ms: SMEP_MAX_AGE_MS };
        assert_eq!(validate_eviction_policy(&[r]), Ok(()));
    }

    /// **SMEP-10** — max per chain boundary accepted.
    #[test]
    fn smep_10_max_per_chain_accepted() {
        let rs: Vec<SkippedKeyRecord> = (0..SMEP_MAX_PER_CHAIN)
            .map(|i| skipped(0x01, (i as u8).wrapping_add(1), (i as u64) * 100))
            .collect();
        assert_eq!(validate_eviction_policy(&rs), Ok(()));
    }
}
