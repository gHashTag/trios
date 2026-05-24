//! # CR-CHAT-04 — Padding fill byte randomness guard (Wave-147 Lane A)
//!
//! PADDING — padding fill bytes must be random-looking; deterministic
//! fill (e.g. all zeros) leaks padding boundaries.
//!
//! When padding messages, the filler bytes should be
//! indistinguishable from encrypted content. If fill bytes follow
//! a predictable pattern:
//!
//! * **Padding boundary detection** — an observer who can identify
//!   the transition from encrypted payload to padding can determine
//!   the true message length.
//! * **Content size leak** — knowing the padding pattern reveals
//!   the actual payload size within the padded block.
//! * **Protocol fingerprinting** — specific fill patterns identify
//!   the protocol implementation.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Fill byte entropy (unique byte count) >= `PFBR_MIN_UNIQUE`.
//! 2. Fill byte chi-squared <= `PFBR_MAX_CHI_SQUARED`.
//! 3. Block ID must not be zero.
//! 4. No duplicate block IDs.
//! 5. Block size >= `PFBR_MIN_BLOCK`.
//! 6. Batch size <= `PFBR_MAX_BLOCKS`.
//!
//! Tests **PFBR-01..10**. Error enum [`FillRandomnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FILL-RANDOM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum unique bytes in fill region.
pub const PFBR_MIN_UNIQUE: usize = 16;

/// Maximum chi-squared for byte frequency.
pub const PFBR_MAX_CHI_SQUARED: f64 = 400.0;

/// Minimum block size.
pub const PFBR_MIN_BLOCK: usize = 32;

/// Maximum blocks per batch.
pub const PFBR_MAX_BLOCKS: usize = 128;

/// Block ID length.
pub const PFBR_BLOCK_ID_LEN: usize = 16;

/// A padding fill randomness record.
#[derive(Debug, Clone)]
pub struct FillRandomnessRecord {
    /// Block identifier.
    pub block_id: [u8; PFBR_BLOCK_ID_LEN],
    /// Block size.
    pub block_size: usize,
    /// Number of unique byte values in fill region.
    pub unique_bytes: usize,
    /// Chi-squared statistic for byte frequency.
    pub chi_squared: f64,
}

/// All ways fill randomness validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum FillRandomnessError {
    /// Too few unique bytes.
    LowEntropy { idx: usize, got: usize, min: usize },
    /// Chi-squared too high.
    HighChi { idx: usize, got: f64, max: f64 },
    /// Zero block ID.
    ZeroBlockId(usize),
    /// Duplicate block ID.
    DuplicateBlockId { idx: usize },
    /// Block too small.
    TooSmall { idx: usize, got: usize, min: usize },
    /// Too many blocks.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding fill byte randomness.
pub fn validate_fill_randomness(
    blocks: &[FillRandomnessRecord],
) -> Result<(), FillRandomnessError> {
    if blocks.len() > PFBR_MAX_BLOCKS {
        return Err(FillRandomnessError::TooMany {
            got: blocks.len(),
            max: PFBR_MAX_BLOCKS,
        });
    }
    let mut seen: BTreeSet<[u8; PFBR_BLOCK_ID_LEN]> = BTreeSet::new();
    for (i, b) in blocks.iter().enumerate() {
        if b.block_id == [0u8; PFBR_BLOCK_ID_LEN] {
            return Err(FillRandomnessError::ZeroBlockId(i));
        }
        if !seen.insert(b.block_id) {
            return Err(FillRandomnessError::DuplicateBlockId { idx: i });
        }
        if b.block_size < PFBR_MIN_BLOCK {
            return Err(FillRandomnessError::TooSmall {
                idx: i,
                got: b.block_size,
                min: PFBR_MIN_BLOCK,
            });
        }
        if b.unique_bytes < PFBR_MIN_UNIQUE {
            return Err(FillRandomnessError::LowEntropy {
                idx: i,
                got: b.unique_bytes,
                min: PFBR_MIN_UNIQUE,
            });
        }
        if b.chi_squared > PFBR_MAX_CHI_SQUARED {
            return Err(FillRandomnessError::HighChi {
                idx: i,
                got: b.chi_squared,
                max: PFBR_MAX_CHI_SQUARED,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PFBR_BLOCK_ID_LEN] {
        [byte; PFBR_BLOCK_ID_LEN]
    }

    fn block(id: u8, size: usize, unique: usize, chi: f64) -> FillRandomnessRecord {
        FillRandomnessRecord { block_id: bid(id), block_size: size, unique_bytes: unique, chi_squared: chi }
    }

    fn valid_blocks() -> Vec<FillRandomnessRecord> {
        vec![
            block(0x01, 256, 200, 280.0),
            block(0x02, 512, 220, 300.0),
        ]
    }

    /// **PFBR-01** — low entropy rejected.
    #[test]
    fn pfbr_01_low_entropy_rejected() {
        let b = block(0x01, 256, PFBR_MIN_UNIQUE - 1, 200.0);
        assert_eq!(
            validate_fill_randomness(&[b]),
            Err(FillRandomnessError::LowEntropy {
                idx: 0,
                got: PFBR_MIN_UNIQUE - 1,
                min: PFBR_MIN_UNIQUE,
            })
        );
    }

    /// **PFBR-02** — high chi rejected.
    #[test]
    fn pfbr_02_high_chi_rejected() {
        let b = block(0x01, 256, 200, PFBR_MAX_CHI_SQUARED + 50.0);
        assert_eq!(
            validate_fill_randomness(&[b]),
            Err(FillRandomnessError::HighChi {
                idx: 0,
                got: PFBR_MAX_CHI_SQUARED + 50.0,
                max: PFBR_MAX_CHI_SQUARED,
            })
        );
    }

    /// **PFBR-03** — zero block ID rejected.
    #[test]
    fn pfbr_03_zero_block_rejected() {
        let b = FillRandomnessRecord {
            block_id: [0u8; PFBR_BLOCK_ID_LEN],
            block_size: 256,
            unique_bytes: 200,
            chi_squared: 250.0,
        };
        assert_eq!(
            validate_fill_randomness(&[b]),
            Err(FillRandomnessError::ZeroBlockId(0))
        );
    }

    /// **PFBR-04** — duplicate block ID rejected.
    #[test]
    fn pfbr_04_duplicate_rejected() {
        let bs = vec![
            block(0x01, 256, 200, 250.0),
            block(0x01, 512, 220, 300.0),
        ];
        assert_eq!(
            validate_fill_randomness(&bs),
            Err(FillRandomnessError::DuplicateBlockId { idx: 1 })
        );
    }

    /// **PFBR-05** — block too small rejected.
    #[test]
    fn pfbr_05_too_small_rejected() {
        let b = block(0x01, PFBR_MIN_BLOCK - 1, 200, 250.0);
        assert_eq!(
            validate_fill_randomness(&[b]),
            Err(FillRandomnessError::TooSmall {
                idx: 0,
                got: PFBR_MIN_BLOCK - 1,
                min: PFBR_MIN_BLOCK,
            })
        );
    }

    /// **PFBR-06** — too many blocks rejected.
    #[test]
    fn pfbr_06_too_many_rejected() {
        let bs: Vec<FillRandomnessRecord> = (0..=PFBR_MAX_BLOCKS)
            .map(|i| {
                let mut id = [0u8; PFBR_BLOCK_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                FillRandomnessRecord { block_id: id, block_size: 256, unique_bytes: 200, chi_squared: 250.0 }
            })
            .collect();
        assert_eq!(
            validate_fill_randomness(&bs),
            Err(FillRandomnessError::TooMany {
                got: PFBR_MAX_BLOCKS + 1,
                max: PFBR_MAX_BLOCKS,
            })
        );
    }

    /// **PFBR-07** — valid accepted.
    #[test]
    fn pfbr_07_valid_accepted() {
        assert_eq!(validate_fill_randomness(&valid_blocks()), Ok(()));
    }

    /// **PFBR-08** — empty accepted.
    #[test]
    fn pfbr_08_empty_accepted() {
        assert_eq!(validate_fill_randomness(&[]), Ok(()));
    }

    /// **PFBR-09** — boundary entropy accepted.
    #[test]
    fn pfbr_09_boundary_entropy_accepted() {
        let b = block(0x01, 256, PFBR_MIN_UNIQUE, PFBR_MAX_CHI_SQUARED);
        assert_eq!(validate_fill_randomness(&[b]), Ok(()));
    }

    /// **PFBR-10** — many high-quality blocks accepted.
    #[test]
    fn pfbr_10_many_hq_accepted() {
        let bs: Vec<FillRandomnessRecord> = (0..20u8)
            .map(|i| block(i + 1, 128 + (i as usize) * 32, 180 + (i as usize), 250.0 + (i as f64) * 5.0))
            .collect();
        assert_eq!(validate_fill_randomness(&bs), Ok(()));
    }
}
