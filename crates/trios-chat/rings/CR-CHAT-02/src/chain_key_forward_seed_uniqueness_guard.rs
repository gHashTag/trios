//! # CR-CHAT-02 — Chain key forward-seed uniqueness guard (Wave-66 Lane B)
//!
//! RATCHET — each chain step must produce a unique forward seed, R-CHAT-2.
//!
//! A ratchet chain derives a sequence of keys from a seed. If two steps
//! produce the same seed, forward secrecy is broken:
//!
//! * **Seed collision** — two epochs share the same chain seed, so
//!   compromising one epoch reveals the other's keys.
//! * **Zero seed** — a chain step produces all-zeros, which may be a
//!   default or uninitialized value.
//! * **Short seed** — seed length below minimum, increasing collision
//!   probability.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All seeds in a chain are unique.
//! 2. No seed is all-zeros.
//! 3. Seed length >= `CKFU_MIN_SEED_LEN`.
//! 4. Seed length <= `CKFU_MAX_SEED_LEN`.
//! 5. Chain length <= `CKFU_MAX_CHAIN_LEN`.
//! 6. Chain length >= 2 (at least two seeds to compare).
//!
//! Tests **CKFU-01..10**. Error enum [`ChainSeedError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-SEED-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum seed length (bytes).
pub const CKFU_MIN_SEED_LEN: usize = 16;

/// Maximum seed length (bytes).
pub const CKFU_MAX_SEED_LEN: usize = 64;

/// Maximum chain length.
pub const CKFU_MAX_CHAIN_LEN: usize = 1024;

/// All ways chain seed uniqueness can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainSeedError {
    /// Duplicate seed in chain.
    DuplicateSeed,
    /// Zero seed found.
    ZeroSeed,
    /// Seed too short.
    SeedTooShort,
    /// Seed too long.
    SeedTooLong,
    /// Chain too long.
    ChainTooLong,
    /// Chain too short (need >= 2 seeds).
    ChainTooShort,
}

/// `[VERIFIED]` Validate that all seeds in a ratchet chain are unique and non-zero.
pub fn validate_chain_seed_uniqueness(
    seeds: &[&[u8]],
) -> Result<(), ChainSeedError> {
    if seeds.len() < 2 {
        return Err(ChainSeedError::ChainTooShort);
    }
    if seeds.len() > CKFU_MAX_CHAIN_LEN {
        return Err(ChainSeedError::ChainTooLong);
    }
    let mut seen = BTreeSet::new();
    for seed in seeds {
        if seed.len() < CKFU_MIN_SEED_LEN {
            return Err(ChainSeedError::SeedTooShort);
        }
        if seed.len() > CKFU_MAX_SEED_LEN {
            return Err(ChainSeedError::SeedTooLong);
        }
        if seed.iter().all(|&b| b == 0) {
            return Err(ChainSeedError::ZeroSeed);
        }
        if !seen.insert(seed.to_vec()) {
            return Err(ChainSeedError::DuplicateSeed);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(byte: u8) -> Vec<u8> {
        vec![byte; CKFU_MIN_SEED_LEN]
    }

    fn good_seeds() -> Vec<Vec<u8>> {
        vec![seed(0x01), seed(0x02), seed(0x03)]
    }

    /// **CKFU-01** — duplicate seed rejected.
    #[test]
    fn ckfu_01_duplicate_rejected() {
        let s = seed(0xAA);
        assert_eq!(
            validate_chain_seed_uniqueness(&[s.as_slice(), s.as_slice()]),
            Err(ChainSeedError::DuplicateSeed)
        );
    }

    /// **CKFU-02** — zero seed rejected.
    #[test]
    fn ckfu_02_zero_seed_rejected() {
        let zero = vec![0u8; CKFU_MIN_SEED_LEN];
        assert_eq!(
            validate_chain_seed_uniqueness(&[zero.as_slice(), seed(0x01).as_slice()]),
            Err(ChainSeedError::ZeroSeed)
        );
    }

    /// **CKFU-03** — seed too short rejected.
    #[test]
    fn ckfu_03_too_short_rejected() {
        let short = vec![0x01u8; 8];
        assert_eq!(
            validate_chain_seed_uniqueness(&[short.as_slice(), seed(0x02).as_slice()]),
            Err(ChainSeedError::SeedTooShort)
        );
    }

    /// **CKFU-04** — seed too long rejected.
    #[test]
    fn ckfu_04_too_long_rejected() {
        let long = vec![0x01u8; CKFU_MAX_SEED_LEN + 1];
        assert_eq!(
            validate_chain_seed_uniqueness(&[long.as_slice(), seed(0x02).as_slice()]),
            Err(ChainSeedError::SeedTooLong)
        );
    }

    /// **CKFU-05** — chain too long rejected.
    #[test]
    fn ckfu_05_too_long_rejected() {
        let seeds: Vec<Vec<u8>> = (0..=CKFU_MAX_CHAIN_LEN)
            .map(|i| {
                let mut s = vec![0u8; CKFU_MIN_SEED_LEN];
                s[0] = (i % 256) as u8;
                s[1] = ((i >> 8) % 256) as u8;
                s
            })
            .collect();
        let refs: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
        assert_eq!(
            validate_chain_seed_uniqueness(&refs),
            Err(ChainSeedError::ChainTooLong)
        );
    }

    /// **CKFU-06** — chain too short rejected.
    #[test]
    fn ckfu_06_too_short_rejected() {
        assert_eq!(
            validate_chain_seed_uniqueness(&[seed(0x01).as_slice()]),
            Err(ChainSeedError::ChainTooShort)
        );
    }

    /// **CKFU-07** — good chain accepted.
    #[test]
    fn ckfu_07_good_accepted() {
        let s = good_seeds();
        let refs: Vec<&[u8]> = s.iter().map(|x| x.as_slice()).collect();
        assert_eq!(validate_chain_seed_uniqueness(&refs), Ok(()));
    }

    /// **CKFU-08** — two distinct seeds accepted.
    #[test]
    fn ckfu_08_two_seeds_accepted() {
        assert_eq!(
            validate_chain_seed_uniqueness(&[seed(0x01).as_slice(), seed(0x02).as_slice()]),
            Ok(())
        );
    }

    /// **CKFU-09** — min seed length accepted.
    #[test]
    fn ckfu_09_min_len_accepted() {
        let a = vec![0x01u8; CKFU_MIN_SEED_LEN];
        let b = vec![0x02u8; CKFU_MIN_SEED_LEN];
        assert_eq!(
            validate_chain_seed_uniqueness(&[a.as_slice(), b.as_slice()]),
            Ok(())
        );
    }

    /// **CKFU-10** — max seed length accepted.
    #[test]
    fn ckfu_10_max_len_accepted() {
        let a = vec![0x01u8; CKFU_MAX_SEED_LEN];
        let b = vec![0x02u8; CKFU_MAX_SEED_LEN];
        assert_eq!(
            validate_chain_seed_uniqueness(&[a.as_slice(), b.as_slice()]),
            Ok(())
        );
    }
}
