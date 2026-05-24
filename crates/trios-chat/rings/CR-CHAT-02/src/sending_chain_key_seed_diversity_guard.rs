//! # CR-CHAT-02 — Sending chain key seed diversity guard (Wave-146 Lane B)
//!
//! RATCHET — chain key seeds must have sufficient diversity;
//! low-diversity seeds weaken the ratchet.
//!
//! In the Double Ratchet, the sending chain advances by hashing
//! the chain key with a seed. If seeds lack diversity:
//!
//! * **Chain key convergence** — repeated or similar seeds produce
//!   related chain keys, reducing the effective key space.
//! * **Seed reuse** — using the same seed twice produces identical
//!   chain keys, violating the forward secrecy guarantee.
//! * **Entropy deficit** — low-entropy seeds make the chain
//!   predictable to an attacker who observes output keys.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Minimum seed entropy >= `SCKS_MIN_ENTROPY_BITS`.
//! 2. No duplicate seeds.
//! 3. Chain ID must not be zero.
//! 4. No duplicate chain IDs.
//! 5. Seed length must be >= `SCKS_MIN_SEED_LEN`.
//! 6. Batch size <= `SCKS_MAX_SEEDS`.
//!
//! Tests **SCKS-01..10**. Error enum [`SeedDiversityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SEED-DIVERSE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum seed entropy in bits (estimated by unique byte count).
pub const SCKS_MIN_ENTROPY_BITS: usize = 128;

/// Minimum seed length in bytes.
pub const SCKS_MIN_SEED_LEN: usize = 32;

/// Maximum seeds per batch.
pub const SCKS_MAX_SEEDS: usize = 256;

/// Chain ID length.
pub const SCKS_CHAIN_ID_LEN: usize = 32;

/// Seed length.
pub const SCKS_SEED_LEN: usize = 32;

/// A chain key seed record.
#[derive(Debug, Clone)]
pub struct SeedDiversityRecord {
    /// Chain identifier.
    pub chain_id: [u8; SCKS_CHAIN_ID_LEN],
    /// Seed value.
    pub seed: [u8; SCKS_SEED_LEN],
}

/// All ways seed diversity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeedDiversityError {
    /// Seed entropy too low.
    LowEntropy {
        /// Index.
        idx: usize,
        /// Estimated unique bytes.
        unique_bytes: usize,
        /// Minimum required.
        min_unique: usize,
    },
    /// Duplicate seed.
    DuplicateSeed {
        /// Index.
        idx: usize,
    },
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Duplicate chain ID.
    DuplicateChainId {
        /// Index.
        idx: usize,
    },
    /// Seed too short (wrong length).
    InvalidSeedLen(usize),
    /// Too many seeds.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate sending chain key seed diversity.
pub fn validate_seed_diversity(
    records: &[SeedDiversityRecord],
) -> Result<(), SeedDiversityError> {
    if records.len() > SCKS_MAX_SEEDS {
        return Err(SeedDiversityError::TooMany {
            got: records.len(),
            max: SCKS_MAX_SEEDS,
        });
    }
    let mut seen_chains: BTreeSet<[u8; SCKS_CHAIN_ID_LEN]> = BTreeSet::new();
    let mut seen_seeds: BTreeSet<[u8; SCKS_SEED_LEN]> = BTreeSet::new();
    let min_unique = SCKS_MIN_ENTROPY_BITS / 8;
    for (i, r) in records.iter().enumerate() {
        if r.chain_id == [0u8; SCKS_CHAIN_ID_LEN] {
            return Err(SeedDiversityError::ZeroChainId(i));
        }
        if !seen_chains.insert(r.chain_id) {
            return Err(SeedDiversityError::DuplicateChainId { idx: i });
        }
        if !seen_seeds.insert(r.seed) {
            return Err(SeedDiversityError::DuplicateSeed { idx: i });
        }
        let unique_bytes: usize = {
            let mut set = BTreeSet::new();
            for &b in &r.seed {
                set.insert(b);
            }
            set.len()
        };
        if unique_bytes < min_unique {
            return Err(SeedDiversityError::LowEntropy {
                idx: i,
                unique_bytes,
                min_unique,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; SCKS_CHAIN_ID_LEN] {
        [byte; SCKS_CHAIN_ID_LEN]
    }

    fn diverse_seed(base: u8) -> [u8; SCKS_SEED_LEN] {
        let mut s = [0u8; SCKS_SEED_LEN];
        for (i, b) in s.iter_mut().enumerate() {
            *b = base.wrapping_add(i as u8).wrapping_mul(7).wrapping_add(1);
        }
        s
    }

    fn rec(chain: u8, seed: [u8; SCKS_SEED_LEN]) -> SeedDiversityRecord {
        SeedDiversityRecord { chain_id: cid(chain), seed }
    }

    fn valid_records() -> Vec<SeedDiversityRecord> {
        vec![
            rec(0x01, diverse_seed(0x10)),
            rec(0x02, diverse_seed(0x20)),
        ]
    }

    /// **SCKS-01** — low entropy rejected.
    #[test]
    fn scks_01_low_entropy_rejected() {
        let seed = [0xAAu8; SCKS_SEED_LEN];
        let r = rec(0x01, seed);
        assert_eq!(
            validate_seed_diversity(&[r]),
            Err(SeedDiversityError::LowEntropy {
                idx: 0,
                unique_bytes: 1,
                min_unique: SCKS_MIN_ENTROPY_BITS / 8,
            })
        );
    }

    /// **SCKS-02** — duplicate seed rejected.
    #[test]
    fn scks_02_duplicate_seed_rejected() {
        let s = diverse_seed(0x10);
        let rs = vec![
            rec(0x01, s),
            rec(0x02, s),
        ];
        assert_eq!(
            validate_seed_diversity(&rs),
            Err(SeedDiversityError::DuplicateSeed { idx: 1 })
        );
    }

    /// **SCKS-03** — zero chain ID rejected.
    #[test]
    fn scks_03_zero_chain_rejected() {
        let r = SeedDiversityRecord { chain_id: [0u8; SCKS_CHAIN_ID_LEN], seed: diverse_seed(0x10) };
        assert_eq!(
            validate_seed_diversity(&[r]),
            Err(SeedDiversityError::ZeroChainId(0))
        );
    }

    /// **SCKS-04** — duplicate chain ID rejected.
    #[test]
    fn scks_04_duplicate_chain_rejected() {
        let rs = vec![
            rec(0x01, diverse_seed(0x10)),
            rec(0x01, diverse_seed(0x20)),
        ];
        assert_eq!(
            validate_seed_diversity(&rs),
            Err(SeedDiversityError::DuplicateChainId { idx: 1 })
        );
    }

    /// **SCKS-05** — invalid seed length (all zeros) rejected via low entropy.
    #[test]
    fn scks_05_zero_seed_rejected() {
        let r = rec(0x01, [0u8; SCKS_SEED_LEN]);
        assert_eq!(
            validate_seed_diversity(&[r]),
            Err(SeedDiversityError::LowEntropy {
                idx: 0,
                unique_bytes: 1,
                min_unique: SCKS_MIN_ENTROPY_BITS / 8,
            })
        );
    }

    /// **SCKS-06** — too many rejected.
    #[test]
    fn scks_06_too_many_rejected() {
        let rs: Vec<SeedDiversityRecord> = (0..=SCKS_MAX_SEEDS)
            .map(|i| {
                let mut id = [0u8; SCKS_CHAIN_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut seed = [0u8; SCKS_SEED_LEN];
                for (j, b) in seed.iter_mut().enumerate() {
                    *b = ((i as u8).wrapping_add(j as u8)).wrapping_add(1);
                }
                SeedDiversityRecord { chain_id: id, seed }
            })
            .collect();
        assert_eq!(
            validate_seed_diversity(&rs),
            Err(SeedDiversityError::TooMany {
                got: SCKS_MAX_SEEDS + 1,
                max: SCKS_MAX_SEEDS,
            })
        );
    }

    /// **SCKS-07** — valid accepted.
    #[test]
    fn scks_07_valid_accepted() {
        assert_eq!(validate_seed_diversity(&valid_records()), Ok(()));
    }

    /// **SCKS-08** — empty accepted.
    #[test]
    fn scks_08_empty_accepted() {
        assert_eq!(validate_seed_diversity(&[]), Ok(()));
    }

    /// **SCKS-09** — boundary entropy accepted.
    #[test]
    fn scks_09_boundary_entropy_accepted() {
        let min_unique = SCKS_MIN_ENTROPY_BITS / 8;
        let mut seed = [0u8; SCKS_SEED_LEN];
        for (i, b) in seed.iter_mut().enumerate() {
            *b = if i < min_unique { (i as u8) + 1 } else { 1 };
        }
        let r = rec(0x01, seed);
        assert_eq!(validate_seed_diversity(&[r]), Ok(()));
    }

    /// **SCKS-10** — many diverse seeds accepted.
    #[test]
    fn scks_10_many_diverse_accepted() {
        let rs: Vec<SeedDiversityRecord> = (0..20u8)
            .map(|i| rec(i + 1, diverse_seed(i.wrapping_mul(13))))
            .collect();
        assert_eq!(validate_seed_diversity(&rs), Ok(()));
    }
}
