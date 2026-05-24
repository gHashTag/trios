//! # CR-CHAT-04 — Padding byte pattern uniformity guard (Wave-139 Lane A)
//!
//! PADDING — padding bytes must be uniformly distributed; structured
//! padding leaks implementation fingerprints.
//!
//! When padding messages, the filler bytes must not follow a
//! predictable pattern (e.g. all zeros, incrementing sequence).
//! An observer who can distinguish padding from real content via
//! byte patterns gains:
//!
//! * **Content discrimination** — structured padding is easily
//!   separated from encrypted payload, reducing effective cover.
//! * **Implementation fingerprint** — specific padding patterns
//!   identify the client software version.
//! * **Statistical attack** — non-uniform padding creates a
//!   distinguisher that reduces the effective padding budget.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Byte frequency chi-squared <= `PBPU_MAX_CHI_SQUARED`.
//! 2. Minimum samples >= `PBPU_MIN_SAMPLES`.
//! 3. Maximum samples <= `PBPU_MAX_SAMPLES`.
//! 4. No duplicate block IDs.
//! 5. Block size must be >= `PBPU_MIN_BLOCK`.
//! 6. Batch size <= `PBPU_MAX_BLOCKS`.
//!
//! Tests **PBPU-01..10**. Error enum [`PatternUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * UNIFORM-PAD`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chi-squared statistic for byte frequency uniformity.
pub const PBPU_MAX_CHI_SQUARED: f64 = 350.0;

/// Minimum number of byte samples.
pub const PBPU_MIN_SAMPLES: usize = 64;

/// Maximum number of byte samples per block.
pub const PBPU_MAX_SAMPLES: usize = 65536;

/// Minimum block size.
pub const PBPU_MIN_BLOCK: usize = 32;

/// Maximum blocks per batch.
pub const PBPU_MAX_BLOCKS: usize = 128;

/// Block ID length.
pub const PBPU_BLOCK_ID_LEN: usize = 16;

/// A padding block uniformity record.
#[derive(Debug, Clone)]
pub struct PaddingPatternRecord {
    /// Block identifier.
    pub block_id: [u8; PBPU_BLOCK_ID_LEN],
    /// Number of byte samples analyzed.
    pub sample_count: usize,
    /// Chi-squared statistic of byte frequency distribution.
    pub chi_squared: f64,
}

/// All ways pattern uniformity validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum PatternUniformityError {
    /// Chi-squared too high (non-uniform).
    HighChiSquared { idx: usize, got: f64, max: f64 },
    /// Too few samples.
    TooFewSamples { idx: usize, got: usize, min: usize },
    /// Too many samples.
    TooManySamples { idx: usize, got: usize, max: usize },
    /// Duplicate block ID.
    DuplicateBlockId { idx: usize },
    /// Block size below minimum.
    TooSmall { idx: usize, got: usize, min: usize },
    /// Batch too large.
    TooLargeBatch { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding byte pattern uniformity.
pub fn validate_pattern_uniformity(
    blocks: &[PaddingPatternRecord],
) -> Result<(), PatternUniformityError> {
    if blocks.len() > PBPU_MAX_BLOCKS {
        return Err(PatternUniformityError::TooLargeBatch {
            got: blocks.len(),
            max: PBPU_MAX_BLOCKS,
        });
    }
    let mut seen: BTreeSet<[u8; PBPU_BLOCK_ID_LEN]> = BTreeSet::new();
    for (i, b) in blocks.iter().enumerate() {
        if !seen.insert(b.block_id) {
            return Err(PatternUniformityError::DuplicateBlockId { idx: i });
        }
        if b.sample_count < PBPU_MIN_BLOCK {
            return Err(PatternUniformityError::TooSmall {
                idx: i,
                got: b.sample_count,
                min: PBPU_MIN_BLOCK,
            });
        }
        if b.sample_count < PBPU_MIN_SAMPLES {
            return Err(PatternUniformityError::TooFewSamples {
                idx: i,
                got: b.sample_count,
                min: PBPU_MIN_SAMPLES,
            });
        }
        if b.sample_count > PBPU_MAX_SAMPLES {
            return Err(PatternUniformityError::TooManySamples {
                idx: i,
                got: b.sample_count,
                max: PBPU_MAX_SAMPLES,
            });
        }
        if b.chi_squared > PBPU_MAX_CHI_SQUARED {
            return Err(PatternUniformityError::HighChiSquared {
                idx: i,
                got: b.chi_squared,
                max: PBPU_MAX_CHI_SQUARED,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBPU_BLOCK_ID_LEN] {
        [byte; PBPU_BLOCK_ID_LEN]
    }

    fn block(id: u8, samples: usize, chi: f64) -> PaddingPatternRecord {
        PaddingPatternRecord { block_id: bid(id), sample_count: samples, chi_squared: chi }
    }

    fn valid_blocks() -> Vec<PaddingPatternRecord> {
        vec![
            block(0x01, 256, 200.0),
            block(0x02, 512, 280.0),
        ]
    }

    /// **PBPU-01** — high chi-squared rejected.
    #[test]
    fn pbpu_01_high_chi_rejected() {
        let b = block(0x01, 256, PBPU_MAX_CHI_SQUARED + 50.0);
        assert_eq!(
            validate_pattern_uniformity(&[b]),
            Err(PatternUniformityError::HighChiSquared {
                idx: 0,
                got: PBPU_MAX_CHI_SQUARED + 50.0,
                max: PBPU_MAX_CHI_SQUARED,
            })
        );
    }

    /// **PBPU-02** — too few samples rejected.
    #[test]
    fn pbpu_02_too_few_rejected() {
        let b = block(0x01, PBPU_MIN_BLOCK, 200.0);
        assert_eq!(
            validate_pattern_uniformity(&[b]),
            Err(PatternUniformityError::TooFewSamples {
                idx: 0,
                got: PBPU_MIN_BLOCK,
                min: PBPU_MIN_SAMPLES,
            })
        );
    }

    /// **PBPU-03** — too many samples rejected.
    #[test]
    fn pbpu_03_too_many_rejected() {
        let b = block(0x01, PBPU_MAX_SAMPLES + 1, 200.0);
        assert_eq!(
            validate_pattern_uniformity(&[b]),
            Err(PatternUniformityError::TooManySamples {
                idx: 0,
                got: PBPU_MAX_SAMPLES + 1,
                max: PBPU_MAX_SAMPLES,
            })
        );
    }

    /// **PBPU-04** — duplicate block ID rejected.
    #[test]
    fn pbpu_04_duplicate_rejected() {
        let bs = vec![
            block(0x01, 256, 200.0),
            block(0x01, 512, 250.0),
        ];
        assert_eq!(
            validate_pattern_uniformity(&bs),
            Err(PatternUniformityError::DuplicateBlockId { idx: 1 })
        );
    }

    /// **PBPU-05** — block too small rejected.
    #[test]
    fn pbpu_05_too_small_rejected() {
        let b = block(0x01, PBPU_MIN_BLOCK - 1, 200.0);
        assert_eq!(
            validate_pattern_uniformity(&[b]),
            Err(PatternUniformityError::TooSmall {
                idx: 0,
                got: PBPU_MIN_BLOCK - 1,
                min: PBPU_MIN_BLOCK,
            })
        );
    }

    /// **PBPU-06** — batch too large rejected.
    #[test]
    fn pbpu_06_too_large_batch_rejected() {
        let bs: Vec<PaddingPatternRecord> = (0..=PBPU_MAX_BLOCKS)
            .map(|i| {
                let mut id = [0u8; PBPU_BLOCK_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                PaddingPatternRecord { block_id: id, sample_count: 256, chi_squared: 200.0 }
            })
            .collect();
        assert_eq!(
            validate_pattern_uniformity(&bs),
            Err(PatternUniformityError::TooLargeBatch {
                got: PBPU_MAX_BLOCKS + 1,
                max: PBPU_MAX_BLOCKS,
            })
        );
    }

    /// **PBPU-07** — valid accepted.
    #[test]
    fn pbpu_07_valid_accepted() {
        assert_eq!(validate_pattern_uniformity(&valid_blocks()), Ok(()));
    }

    /// **PBPU-08** — empty accepted.
    #[test]
    fn pbpu_08_empty_accepted() {
        assert_eq!(validate_pattern_uniformity(&[]), Ok(()));
    }

    /// **PBPU-09** — boundary chi-squared accepted.
    #[test]
    fn pbpu_09_boundary_chi_accepted() {
        let b = block(0x01, 256, PBPU_MAX_CHI_SQUARED);
        assert_eq!(validate_pattern_uniformity(&[b]), Ok(()));
    }

    /// **PBPU-10** — many uniform blocks accepted.
    #[test]
    fn pbpu_10_many_uniform_accepted() {
        let bs: Vec<PaddingPatternRecord> = (0..20u8)
            .map(|i| block(i + 1, 128, 150.0 + (i as f64) * 5.0))
            .collect();
        assert_eq!(validate_pattern_uniformity(&bs), Ok(()));
    }
}
