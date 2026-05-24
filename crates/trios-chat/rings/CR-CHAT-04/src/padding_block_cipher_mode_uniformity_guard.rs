//! # CR-CHAT-04 — Padding block cipher mode uniformity guard (Wave-120 Lane B)
//!
//! PADDING — padded blocks must produce uniformly distributed
//! ciphertext; non-uniform output reveals which bytes are padding
//! vs payload.
//!
//! When a message is padded and encrypted, the ciphertext blocks must
//! be statistically uniform. Non-uniform output is exploitable:
//!
//! * **Block classification** — if padding blocks produce a different
//!   ciphertext distribution than payload blocks, the attacker can
//!   distinguish them via frequency analysis.
//! * **Mode leakage** — certain block cipher modes (e.g., ECB) produce
//!   identical ciphertext for identical plaintext blocks, creating a
//!   fingerprint for repeated padding patterns.
//! * **Statistical fingerprint** — chi-squared test on block byte
//!   frequencies reveals non-uniform encryption.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Block entropy >= `PBCU_MIN_ENTROPY` bits per byte.
//! 2. Block size must be `PBCU_BLOCK_SIZE`.
//! 3. No zero blocks.
//! 4. No duplicate block hashes.
//! 5. Block index must be unique.
//! 6. Total blocks <= `PBCU_MAX_BLOCKS`.
//!
//! Tests **PBCU-01..10**. Error enum [`BlockUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BLOCK-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Block size in bytes.
pub const PBCU_BLOCK_SIZE: usize = 16;

/// Minimum entropy per block in bits per byte.
pub const PBCU_MIN_ENTROPY: f64 = 3.0;

/// Maximum blocks per batch.
pub const PBCU_MAX_BLOCKS: usize = 4096;

/// Hash length for dedup.
pub const PBCU_HASH_LEN: usize = 32;

/// A ciphertext block record.
#[derive(Debug, Clone)]
pub struct BlockRecord {
    /// Block index.
    pub block_index: u64,
    /// Ciphertext block data (must be `PBCU_BLOCK_SIZE` bytes).
    pub block_data: Vec<u8>,
    /// Hash of the block for dedup.
    pub block_hash: [u8; PBCU_HASH_LEN],
}

/// All ways block uniformity validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum BlockUniformityError {
    /// Entropy below minimum.
    BelowMinEntropy { idx: usize, entropy: f64, min: f64 },
    /// Wrong block size.
    WrongSize { idx: usize, got: usize, expected: usize },
    /// Zero block (all zeros).
    ZeroBlock(usize),
    /// Duplicate block hash.
    DuplicateHash(usize),
    /// Duplicate block index.
    DuplicateIndex(usize),
    /// Too many blocks.
    TooMany { got: usize, max: usize },
}

fn compute_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut freq = [0usize; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut entropy = 0.0;
    for &f in &freq {
        if f > 0 {
            let p = f as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// `[VERIFIED]` Validate padding block cipher mode uniformity.
pub fn validate_block_uniformity(
    blocks: &[BlockRecord],
) -> Result<(), BlockUniformityError> {
    if blocks.len() > PBCU_MAX_BLOCKS {
        return Err(BlockUniformityError::TooMany {
            got: blocks.len(),
            max: PBCU_MAX_BLOCKS,
        });
    }
    let mut seen_hashes: BTreeSet<[u8; PBCU_HASH_LEN]> = BTreeSet::new();
    let mut seen_indices: BTreeSet<u64> = BTreeSet::new();
    for (i, b) in blocks.iter().enumerate() {
        if b.block_data.len() != PBCU_BLOCK_SIZE {
            return Err(BlockUniformityError::WrongSize {
                idx: i,
                got: b.block_data.len(),
                expected: PBCU_BLOCK_SIZE,
            });
        }
        if b.block_data.iter().all(|&byte| byte == 0) {
            return Err(BlockUniformityError::ZeroBlock(i));
        }
        let entropy = compute_entropy(&b.block_data);
        if entropy < PBCU_MIN_ENTROPY {
            return Err(BlockUniformityError::BelowMinEntropy {
                idx: i,
                entropy,
                min: PBCU_MIN_ENTROPY,
            });
        }
        if !seen_hashes.insert(b.block_hash) {
            return Err(BlockUniformityError::DuplicateHash(i));
        }
        if !seen_indices.insert(b.block_index) {
            return Err(BlockUniformityError::DuplicateIndex(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn high_entropy_block(seed: u8) -> Vec<u8> {
        (0u8..PBCU_BLOCK_SIZE as u8).map(|i| i.wrapping_mul(79).wrapping_add(seed)).collect()
    }

    fn low_entropy_block() -> Vec<u8> {
        vec![0x01u8; PBCU_BLOCK_SIZE]
    }

    fn hash(byte: u8) -> [u8; PBCU_HASH_LEN] {
        [byte; PBCU_HASH_LEN]
    }

    fn block(idx: u64, data: Vec<u8>, h: u8) -> BlockRecord {
        BlockRecord { block_index: idx, block_data: data, block_hash: hash(h) }
    }

    fn valid_blocks() -> Vec<BlockRecord> {
        vec![
            block(0, high_entropy_block(0x42), 0x01),
            block(1, high_entropy_block(0x55), 0x02),
            block(2, high_entropy_block(0x99), 0x03),
        ]
    }

    /// **PBCU-01** — low entropy rejected.
    #[test]
    fn pbcu_01_low_entropy_rejected() {
        let b = block(0, low_entropy_block(), 0x01);
        assert!(matches!(
            validate_block_uniformity(&[b]),
            Err(BlockUniformityError::BelowMinEntropy { .. })
        ));
    }

    /// **PBCU-02** — wrong size rejected.
    #[test]
    fn pbcu_02_wrong_size_rejected() {
        let b = BlockRecord { block_index: 0, block_data: vec![0xAA; 8], block_hash: hash(0x01) };
        assert_eq!(
            validate_block_uniformity(&[b]),
            Err(BlockUniformityError::WrongSize { idx: 0, got: 8, expected: PBCU_BLOCK_SIZE })
        );
    }

    /// **PBCU-03** — zero block rejected.
    #[test]
    fn pbcu_03_zero_block_rejected() {
        let b = BlockRecord { block_index: 0, block_data: vec![0u8; PBCU_BLOCK_SIZE], block_hash: hash(0x01) };
        assert_eq!(
            validate_block_uniformity(&[b]),
            Err(BlockUniformityError::ZeroBlock(0))
        );
    }

    /// **PBCU-04** — duplicate hash rejected.
    #[test]
    fn pbcu_04_duplicate_hash_rejected() {
        let bs = vec![
            block(0, high_entropy_block(0x42), 0x01),
            block(1, high_entropy_block(0x55), 0x01),
        ];
        assert_eq!(
            validate_block_uniformity(&bs),
            Err(BlockUniformityError::DuplicateHash(1))
        );
    }

    /// **PBCU-05** — duplicate index rejected.
    #[test]
    fn pbcu_05_duplicate_index_rejected() {
        let bs = vec![
            block(0, high_entropy_block(0x42), 0x01),
            block(0, high_entropy_block(0x55), 0x02),
        ];
        assert_eq!(
            validate_block_uniformity(&bs),
            Err(BlockUniformityError::DuplicateIndex(1))
        );
    }

    /// **PBCU-06** — too many rejected.
    #[test]
    fn pbcu_06_too_many_rejected() {
        let bs: Vec<BlockRecord> = (0..=PBCU_MAX_BLOCKS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                let mut h = [0u8; PBCU_HASH_LEN];
                h[0] = b;
                BlockRecord { block_index: i as u64, block_data: high_entropy_block(b), block_hash: h }
            })
            .collect();
        assert_eq!(
            validate_block_uniformity(&bs),
            Err(BlockUniformityError::TooMany {
                got: PBCU_MAX_BLOCKS + 1,
                max: PBCU_MAX_BLOCKS,
            })
        );
    }

    /// **PBCU-07** — valid accepted.
    #[test]
    fn pbcu_07_valid_accepted() {
        assert_eq!(validate_block_uniformity(&valid_blocks()), Ok(()));
    }

    /// **PBCU-08** — empty accepted.
    #[test]
    fn pbcu_08_empty_accepted() {
        assert_eq!(validate_block_uniformity(&[]), Ok(()));
    }

    /// **PBCU-09** — single block accepted.
    #[test]
    fn pbcu_09_single_accepted() {
        let b = block(0, high_entropy_block(0x42), 0x01);
        assert_eq!(validate_block_uniformity(&[b]), Ok(()));
    }

    /// **PBCU-10** — large batch accepted.
    #[test]
    fn pbcu_10_large_batch_accepted() {
        let bs: Vec<BlockRecord> = (0..256u64)
            .map(|i| {
                let mut h = [0u8; PBCU_HASH_LEN];
                h[0..8].copy_from_slice(&(i + 1).to_be_bytes());
                BlockRecord { block_index: i, block_data: high_entropy_block(i as u8), block_hash: h }
            })
            .collect();
        assert_eq!(validate_block_uniformity(&bs), Ok(()));
    }
}
