//! # CR-CHAT-02 — Sending chain advancement bound guard (Wave-127 Lane A)
//!
//! RATCHET — the sending chain must not advance more than a bounded
//! number of steps without a DH ratchet step; unlimited chain
//! advancement weakens forward secrecy.
//!
//! Each message key is derived by advancing the sending chain key.
//! Without periodic DH ratchet steps:
//!
//! * **Forward secrecy erosion** — if the chain key is compromised,
//!   all future message keys in that chain are recoverable.
//! * **Key chain exhaustion** — deriving too many keys from one
//!   chain key increases the risk of key collision.
//! * **Recommendation violation** — Signal protocol recommends a
//!   DH ratchet step at least every N messages.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Steps since last DH ratchet <= `SCAB_MAX_STEPS`.
//! 2. Chain ID must not be zero.
//! 3. Step count must be > 0.
//! 4. No duplicate chain IDs.
//! 5. Chain key hash must not be zero.
//! 6. Total records <= `SCAB_MAX_RECORDS`.
//!
//! Tests **SCAB-01..10**. Error enum [`ChainBoundError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum sending chain steps without DH ratchet.
pub const SCAB_MAX_STEPS: u64 = 1000;

/// Maximum records per batch.
pub const SCAB_MAX_RECORDS: usize = 1024;

/// Chain ID length.
pub const SCAB_CHAIN_ID_LEN: usize = 32;

/// Chain key hash length.
pub const SCAB_HASH_LEN: usize = 32;

/// A sending chain advancement record.
#[derive(Debug, Clone)]
pub struct ChainAdvanceRecord {
    /// Chain identifier.
    pub chain_id: [u8; SCAB_CHAIN_ID_LEN],
    /// Steps since last DH ratchet.
    pub steps_since_dh: u64,
    /// Hash of the current chain key.
    pub chain_key_hash: [u8; SCAB_HASH_LEN],
}

/// All ways chain bound validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainBoundError {
    /// Too many steps since last DH ratchet.
    TooManySteps { idx: usize, got: u64, max: u64 },
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Zero step count.
    ZeroSteps(usize),
    /// Duplicate chain ID.
    DuplicateChainId { idx: usize },
    /// Zero chain key hash.
    ZeroKeyHash(usize),
    /// Too many records.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate sending chain advancement bound.
pub fn validate_chain_bound(
    records: &[ChainAdvanceRecord],
) -> Result<(), ChainBoundError> {
    if records.len() > SCAB_MAX_RECORDS {
        return Err(ChainBoundError::TooMany {
            got: records.len(),
            max: SCAB_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<[u8; SCAB_CHAIN_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.chain_id == [0u8; SCAB_CHAIN_ID_LEN] {
            return Err(ChainBoundError::ZeroChainId(i));
        }
        if r.steps_since_dh == 0 {
            return Err(ChainBoundError::ZeroSteps(i));
        }
        if r.chain_key_hash == [0u8; SCAB_HASH_LEN] {
            return Err(ChainBoundError::ZeroKeyHash(i));
        }
        if !seen.insert(r.chain_id) {
            return Err(ChainBoundError::DuplicateChainId { idx: i });
        }
        if r.steps_since_dh > SCAB_MAX_STEPS {
            return Err(ChainBoundError::TooManySteps {
                idx: i,
                got: r.steps_since_dh,
                max: SCAB_MAX_STEPS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; SCAB_CHAIN_ID_LEN] {
        [byte; SCAB_CHAIN_ID_LEN]
    }

    fn khash(byte: u8) -> [u8; SCAB_HASH_LEN] {
        [byte; SCAB_HASH_LEN]
    }

    fn record(chain: u8, steps: u64, key: u8) -> ChainAdvanceRecord {
        ChainAdvanceRecord { chain_id: cid(chain), steps_since_dh: steps, chain_key_hash: khash(key) }
    }

    fn valid_records() -> Vec<ChainAdvanceRecord> {
        vec![
            record(0x01, 10, 0xA1),
            record(0x02, 500, 0xA2),
            record(0x03, SCAB_MAX_STEPS, 0xA3),
        ]
    }

    /// **SCAB-01** — too many steps rejected.
    #[test]
    fn scab_01_too_many_steps_rejected() {
        let rs = vec![record(0x01, SCAB_MAX_STEPS + 1, 0xAA)];
        assert_eq!(
            validate_chain_bound(&rs),
            Err(ChainBoundError::TooManySteps {
                idx: 0,
                got: SCAB_MAX_STEPS + 1,
                max: SCAB_MAX_STEPS,
            })
        );
    }

    /// **SCAB-02** — zero chain ID rejected.
    #[test]
    fn scab_02_zero_chain_rejected() {
        let r = ChainAdvanceRecord { chain_id: [0u8; SCAB_CHAIN_ID_LEN], steps_since_dh: 10, chain_key_hash: khash(0xAA) };
        assert_eq!(
            validate_chain_bound(&[r]),
            Err(ChainBoundError::ZeroChainId(0))
        );
    }

    /// **SCAB-03** — zero steps rejected.
    #[test]
    fn scab_03_zero_steps_rejected() {
        let r = ChainAdvanceRecord { chain_id: cid(0x01), steps_since_dh: 0, chain_key_hash: khash(0xAA) };
        assert_eq!(
            validate_chain_bound(&[r]),
            Err(ChainBoundError::ZeroSteps(0))
        );
    }

    /// **SCAB-04** — duplicate chain ID rejected.
    #[test]
    fn scab_04_duplicate_chain_rejected() {
        let rs = vec![
            record(0x01, 10, 0xA1),
            record(0x01, 20, 0xA2),
        ];
        assert_eq!(
            validate_chain_bound(&rs),
            Err(ChainBoundError::DuplicateChainId { idx: 1 })
        );
    }

    /// **SCAB-05** — zero key hash rejected.
    #[test]
    fn scab_05_zero_key_rejected() {
        let r = ChainAdvanceRecord { chain_id: cid(0x01), steps_since_dh: 10, chain_key_hash: [0u8; SCAB_HASH_LEN] };
        assert_eq!(
            validate_chain_bound(&[r]),
            Err(ChainBoundError::ZeroKeyHash(0))
        );
    }

    /// **SCAB-06** — too many records rejected.
    #[test]
    fn scab_06_too_many_rejected() {
        let rs: Vec<ChainAdvanceRecord> = (0..=SCAB_MAX_RECORDS)
            .map(|i| {
                let mut c = [0u8; SCAB_CHAIN_ID_LEN];
                let val = (i as u64) + 1;
                c[0..8].copy_from_slice(&val.to_be_bytes());
                let mut k = [0u8; SCAB_HASH_LEN];
                k[0] = (i as u8).wrapping_add(1);
                ChainAdvanceRecord { chain_id: c, steps_since_dh: 10, chain_key_hash: k }
            })
            .collect();
        assert_eq!(
            validate_chain_bound(&rs),
            Err(ChainBoundError::TooMany {
                got: SCAB_MAX_RECORDS + 1,
                max: SCAB_MAX_RECORDS,
            })
        );
    }

    /// **SCAB-07** — valid accepted.
    #[test]
    fn scab_07_valid_accepted() {
        assert_eq!(validate_chain_bound(&valid_records()), Ok(()));
    }

    /// **SCAB-08** — empty accepted.
    #[test]
    fn scab_08_empty_accepted() {
        assert_eq!(validate_chain_bound(&[]), Ok(()));
    }

    /// **SCAB-09** — single step accepted.
    #[test]
    fn scab_09_single_step_accepted() {
        let rs = vec![record(0x01, 1, 0xAA)];
        assert_eq!(validate_chain_bound(&rs), Ok(()));
    }

    /// **SCAB-10** — boundary steps accepted.
    #[test]
    fn scab_10_boundary_accepted() {
        let rs = vec![record(0x01, SCAB_MAX_STEPS, 0xAA)];
        assert_eq!(validate_chain_bound(&rs), Ok(()));
    }
}
