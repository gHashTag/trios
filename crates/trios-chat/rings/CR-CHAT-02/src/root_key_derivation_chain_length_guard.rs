//! # CR-CHAT-02 — Root key derivation chain length guard (Wave-94 Lane A)
//!
//! RATCHET — root key derivation chain must be bounded, R-CHAT-2.
//!
//! Each DH ratchet step extends the root key derivation chain. Without
//! a length bound:
//!
//! * **Memory DoS** — an adversary forces an unbounded number of DH
//!   steps, each adding a link to the chain, exhausting memory.
//! * **CPU exhaustion** — verifying a chain with millions of links
//!   consumes excessive CPU during session setup.
//! * **State bloat** — the root key history grows without bound,
//!   eventually causing OOM or disk exhaustion in persistence.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Chain length <= `RKCL_MAX_CHAIN_LEN`.
//! 2. Each link must have a valid epoch (>= 1).
//! 3. Epochs must be strictly increasing.
//! 4. No duplicate epochs.
//! 5. Root key hash must not be all zeros.
//! 6. Chain must have at least 1 link (if non-empty).
//!
//! Tests **RKCL-01..10**. Error enum [`ChainLengthError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ROOT-KEY-CHAIN-LEN`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chain length.
pub const RKCL_MAX_CHAIN_LEN: usize = 512;

/// Root key hash length.
pub const RKCL_HASH_LEN: usize = 32;

/// A root key derivation chain link.
#[derive(Debug, Clone)]
pub struct RootKeyLink {
    /// Epoch of this derivation.
    pub epoch: u64,
    /// Hash of the root key at this step.
    pub root_key_hash: [u8; RKCL_HASH_LEN],
}

/// All ways chain length validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainLengthError {
    /// Chain too long.
    ChainTooLong,
    /// Zero epoch.
    ZeroEpoch(u64),
    /// Epoch not increasing.
    EpochNotIncreasing(u64),
    /// Duplicate epoch.
    DuplicateEpoch(u64),
    /// Zero root key hash.
    ZeroHash(u64),
}

/// `[VERIFIED]` Validate root key derivation chain length.
pub fn validate_root_key_chain_length(
    chain: &[RootKeyLink],
) -> Result<(), ChainLengthError> {
    if chain.len() > RKCL_MAX_CHAIN_LEN {
        return Err(ChainLengthError::ChainTooLong);
    }
    let mut seen = BTreeSet::new();
    for (i, link) in chain.iter().enumerate() {
        if link.epoch == 0 {
            return Err(ChainLengthError::ZeroEpoch(link.epoch));
        }
        if link.root_key_hash == [0u8; RKCL_HASH_LEN] {
            return Err(ChainLengthError::ZeroHash(link.epoch));
        }
        if !seen.insert(link.epoch) {
            return Err(ChainLengthError::DuplicateEpoch(link.epoch));
        }
        if i > 0 && link.epoch <= chain[i - 1].epoch {
            return Err(ChainLengthError::EpochNotIncreasing(link.epoch));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; RKCL_HASH_LEN] {
        [byte; RKCL_HASH_LEN]
    }

    fn link(epoch: u64, hash_byte: u8) -> RootKeyLink {
        RootKeyLink { epoch, root_key_hash: hash(hash_byte) }
    }

    fn valid_chain() -> Vec<RootKeyLink> {
        vec![link(1, 0xAA), link(2, 0xBB), link(3, 0xCC)]
    }

    /// **RKCL-01** — chain too long rejected.
    #[test]
    fn rkcl_01_too_long_rejected() {
        let chain: Vec<RootKeyLink> = (1..=RKCL_MAX_CHAIN_LEN as u64 + 1)
            .map(|i| link(i, (i % 254 + 1) as u8))
            .collect();
        assert_eq!(
            validate_root_key_chain_length(&chain),
            Err(ChainLengthError::ChainTooLong)
        );
    }

    /// **RKCL-02** — zero epoch rejected.
    #[test]
    fn rkcl_02_zero_epoch_rejected() {
        let l = RootKeyLink { epoch: 0, root_key_hash: hash(0xAA) };
        assert_eq!(
            validate_root_key_chain_length(&[l]),
            Err(ChainLengthError::ZeroEpoch(0))
        );
    }

    /// **RKCL-03** — epoch not increasing rejected.
    #[test]
    fn rkcl_03_epoch_not_increasing_rejected() {
        let chain = vec![link(2, 0xAA), link(1, 0xBB)];
        assert_eq!(
            validate_root_key_chain_length(&chain),
            Err(ChainLengthError::EpochNotIncreasing(1))
        );
    }

    /// **RKCL-04** — duplicate epoch rejected.
    #[test]
    fn rkcl_04_duplicate_epoch_rejected() {
        let chain = vec![link(1, 0xAA), link(2, 0xBB), link(1, 0xCC)];
        assert_eq!(
            validate_root_key_chain_length(&chain),
            Err(ChainLengthError::DuplicateEpoch(1))
        );
    }

    /// **RKCL-05** — zero hash rejected.
    #[test]
    fn rkcl_05_zero_hash_rejected() {
        let l = RootKeyLink { epoch: 1, root_key_hash: [0u8; RKCL_HASH_LEN] };
        assert_eq!(
            validate_root_key_chain_length(&[l]),
            Err(ChainLengthError::ZeroHash(1))
        );
    }

    /// **RKCL-06** — valid chain accepted.
    #[test]
    fn rkcl_06_valid_accepted() {
        assert_eq!(validate_root_key_chain_length(&valid_chain()), Ok(()));
    }

    /// **RKCL-07** — empty accepted.
    #[test]
    fn rkcl_07_empty_accepted() {
        assert_eq!(validate_root_key_chain_length(&[]), Ok(()));
    }

    /// **RKCL-08** — single link accepted.
    #[test]
    fn rkcl_08_single_accepted() {
        assert_eq!(validate_root_key_chain_length(&[link(1, 0xFF)]), Ok(()));
    }

    /// **RKCL-09** — max chain length boundary accepted.
    #[test]
    fn rkcl_09_max_boundary_accepted() {
        let chain: Vec<RootKeyLink> = (1..=RKCL_MAX_CHAIN_LEN as u64)
            .map(|i| {
                let mut h = [0u8; RKCL_HASH_LEN];
                let bytes = i.to_le_bytes();
                h[..8].copy_from_slice(&bytes);
                h[8] = 0x01;
                RootKeyLink { epoch: i, root_key_hash: h }
            })
            .collect();
        assert_eq!(validate_root_key_chain_length(&chain), Ok(()));
    }

    /// **RKCL-10** — long monotone chain accepted.
    #[test]
    fn rkcl_10_long_monotone_accepted() {
        let chain: Vec<RootKeyLink> = (1..=100u64)
            .map(|i| link(i, (i % 254 + 1) as u8))
            .collect();
        assert_eq!(validate_root_key_chain_length(&chain), Ok(()));
    }
}
