//! # CR-CHAT-04 — Padding block alignment uniformity guard (Wave-158 Lane A)
//!
//! PADDING — padded blocks must have uniform alignment; non-uniform
//! alignment leaks payload boundaries.
//!
//! When messages are padded to block boundaries, the alignment must be
//! uniform across all blocks. If alignment varies:
//!
//! * **Payload boundary detection** — varying alignment reveals where
//!   the actual payload ends and padding begins.
//! * **Size class fingerprinting** — different alignments for
//!   different message sizes create a fingerprint.
//! * **Statistical attack** — over many messages, alignment patterns
//!   leak the payload size distribution.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All block alignments must be equal.
//! 2. Block ID must not be zero.
//! 3. No duplicate block IDs.
//! 4. Alignment must be a power of 2.
//! 5. Alignment must be >= `PBAU_MIN_ALIGNMENT`.
//! 6. Batch size <= `PBAU_MAX_BLOCKS`.
//!
//! Tests **PBAU-01..10**. Error enum [`AlignmentUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ALIGN-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum blocks per batch.
pub const PBAU_MAX_BLOCKS: usize = 1024;

/// Minimum alignment (bytes).
pub const PBAU_MIN_ALIGNMENT: usize = 16;

/// Block ID length.
pub const PBAU_BLOCK_ID_LEN: usize = 16;

/// A padding block alignment record.
#[derive(Debug, Clone)]
pub struct AlignmentRecord {
    /// Block identifier.
    pub block_id: [u8; PBAU_BLOCK_ID_LEN],
    /// Alignment in bytes.
    pub alignment: usize,
}

/// All ways alignment uniformity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlignmentUniformityError {
    /// Non-uniform alignment.
    NonUniform {
        idx: usize,
        expected: usize,
        found: usize,
    },
    /// Zero block ID.
    ZeroBlockId(usize),
    /// Duplicate block ID.
    DuplicateBlockId {
        idx: usize,
    },
    /// Not a power of 2.
    NotPowerOfTwo {
        idx: usize,
        got: usize,
    },
    /// Below minimum alignment.
    BelowMinimum {
        idx: usize,
        got: usize,
        min: usize,
    },
    /// Too many blocks.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate padding block alignment uniformity.
pub fn validate_alignment_uniformity(
    blocks: &[AlignmentRecord],
) -> Result<(), AlignmentUniformityError> {
    if blocks.len() > PBAU_MAX_BLOCKS {
        return Err(AlignmentUniformityError::TooMany {
            got: blocks.len(),
            max: PBAU_MAX_BLOCKS,
        });
    }
    let mut seen: BTreeSet<[u8; PBAU_BLOCK_ID_LEN]> = BTreeSet::new();
    let mut first_alignment: Option<usize> = None;
    for (i, b) in blocks.iter().enumerate() {
        if b.block_id == [0u8; PBAU_BLOCK_ID_LEN] {
            return Err(AlignmentUniformityError::ZeroBlockId(i));
        }
        if !seen.insert(b.block_id) {
            return Err(AlignmentUniformityError::DuplicateBlockId { idx: i });
        }
        if b.alignment < PBAU_MIN_ALIGNMENT {
            return Err(AlignmentUniformityError::BelowMinimum {
                idx: i,
                got: b.alignment,
                min: PBAU_MIN_ALIGNMENT,
            });
        }
        if !b.alignment.is_power_of_two() {
            return Err(AlignmentUniformityError::NotPowerOfTwo {
                idx: i,
                got: b.alignment,
            });
        }
        match first_alignment {
            None => first_alignment = Some(b.alignment),
            Some(expected) if b.alignment != expected => {
                return Err(AlignmentUniformityError::NonUniform {
                    idx: i,
                    expected,
                    found: b.alignment,
                });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBAU_BLOCK_ID_LEN] {
        [byte; PBAU_BLOCK_ID_LEN]
    }

    fn blk(id: u8, alignment: usize) -> AlignmentRecord {
        AlignmentRecord { block_id: bid(id), alignment }
    }

    fn valid_blocks() -> Vec<AlignmentRecord> {
        vec![
            blk(0x01, 16),
            blk(0x02, 16),
            blk(0x03, 16),
        ]
    }

    /// **PBAU-01** — non-uniform rejected.
    #[test]
    fn pbau_01_non_uniform_rejected() {
        let bs = vec![
            blk(0x01, 16),
            blk(0x02, 32),
        ];
        assert_eq!(
            validate_alignment_uniformity(&bs),
            Err(AlignmentUniformityError::NonUniform { idx: 1, expected: 16, found: 32 })
        );
    }

    /// **PBAU-02** — zero block ID rejected.
    #[test]
    fn pbau_02_zero_id_rejected() {
        let b = AlignmentRecord { block_id: [0u8; PBAU_BLOCK_ID_LEN], alignment: 16 };
        assert_eq!(
            validate_alignment_uniformity(&[b]),
            Err(AlignmentUniformityError::ZeroBlockId(0))
        );
    }

    /// **PBAU-03** — duplicate block ID rejected.
    #[test]
    fn pbau_03_duplicate_rejected() {
        let bs = vec![
            blk(0x01, 16),
            blk(0x01, 16),
        ];
        assert_eq!(
            validate_alignment_uniformity(&bs),
            Err(AlignmentUniformityError::DuplicateBlockId { idx: 1 })
        );
    }

    /// **PBAU-04** — not power of two rejected.
    #[test]
    fn pbau_04_not_power_of_two_rejected() {
        let b = blk(0x01, 24);
        assert_eq!(
            validate_alignment_uniformity(&[b]),
            Err(AlignmentUniformityError::NotPowerOfTwo { idx: 0, got: 24 })
        );
    }

    /// **PBAU-05** — below minimum rejected.
    #[test]
    fn pbau_05_below_min_rejected() {
        let b = blk(0x01, 8);
        assert_eq!(
            validate_alignment_uniformity(&[b]),
            Err(AlignmentUniformityError::BelowMinimum { idx: 0, got: 8, min: PBAU_MIN_ALIGNMENT })
        );
    }

    /// **PBAU-06** — too many rejected.
    #[test]
    fn pbau_06_too_many_rejected() {
        let bs: Vec<AlignmentRecord> = (0..=PBAU_MAX_BLOCKS)
            .map(|i| {
                let mut id = [0u8; PBAU_BLOCK_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                AlignmentRecord { block_id: id, alignment: 16 }
            })
            .collect();
        assert_eq!(
            validate_alignment_uniformity(&bs),
            Err(AlignmentUniformityError::TooMany {
                got: PBAU_MAX_BLOCKS + 1,
                max: PBAU_MAX_BLOCKS,
            })
        );
    }

    /// **PBAU-07** — valid accepted.
    #[test]
    fn pbau_07_valid_accepted() {
        assert_eq!(validate_alignment_uniformity(&valid_blocks()), Ok(()));
    }

    /// **PBAU-08** — empty accepted.
    #[test]
    fn pbau_08_empty_accepted() {
        assert_eq!(validate_alignment_uniformity(&[]), Ok(()));
    }

    /// **PBAU-09** — single block accepted.
    #[test]
    fn pbau_09_single_accepted() {
        assert_eq!(validate_alignment_uniformity(&[blk(0x01, 64)]), Ok(()));
    }

    /// **PBAU-10** — many uniform accepted.
    #[test]
    fn pbau_10_many_uniform_accepted() {
        let bs: Vec<AlignmentRecord> = (0..50u8)
            .map(|i| blk(i + 1, 32))
            .collect();
        assert_eq!(validate_alignment_uniformity(&bs), Ok(()));
    }
}
