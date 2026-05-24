//! # CR-CHAT-04 — Padding block size alignment guard (Wave-135 Lane B)
//!
//! PADDING — all padding blocks must be aligned to a common block size;
//! misaligned blocks leak protocol framing information.
//!
//! Padding in encrypted messaging adds dummy bytes to obscure message
//! lengths. If padding blocks have inconsistent alignment:
//!
//! * **Framing leak** — an observer can distinguish blocks by their
//!   misalignment, inferring which are real vs padding.
//! * **Size fingerprinting** — misaligned blocks create unique size
//!   signatures that help identify message types.
//! * **Statistical distinguisher** — consistent alignment is required
//!   for the padding to provide uniform cover traffic.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Block size must be >= `PBSA_MIN_BLOCK`.
//! 2. Block size must be <= `PBSA_MAX_BLOCK`.
//! 3. Block size must be a power of 2.
//! 4. All blocks in a batch must share the same block size.
//! 5. Block ID must not be zero.
//! 6. Batch size <= `PBSA_MAX_BATCH`.
//!
//! Tests **PBSA-01..10**. Error enum [`BlockAlignmentError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-ALIGNED`

#![forbid(unsafe_code)]

/// Minimum block size.
pub const PBSA_MIN_BLOCK: usize = 8;

/// Maximum block size.
pub const PBSA_MAX_BLOCK: usize = 4096;

/// Maximum batch size.
pub const PBSA_MAX_BATCH: usize = 256;

/// Block ID length.
pub const PBSA_BLOCK_ID_LEN: usize = 16;

/// A padding block record.
#[derive(Debug, Clone)]
pub struct PaddingBlockRecord {
    /// Block identifier.
    pub block_id: [u8; PBSA_BLOCK_ID_LEN],
    /// Block size in bytes.
    pub block_size: usize,
}

/// All ways padding block alignment validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BlockAlignmentError {
    /// Block size below minimum.
    TooSmall { idx: usize, got: usize, min: usize },
    /// Block size above maximum.
    TooLarge { idx: usize, got: usize, max: usize },
    /// Block size not a power of two.
    NotPowerOfTwo { idx: usize, got: usize },
    /// Block size mismatch within batch.
    Mismatch { idx: usize, got: usize, expected: usize },
    /// Zero block ID.
    ZeroBlockId(usize),
    /// Batch too large.
    TooLargeBatch { got: usize, max: usize },
}

fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// `[VERIFIED]` Validate padding block size alignment.
pub fn validate_padding_block_alignment(
    blocks: &[PaddingBlockRecord],
) -> Result<(), BlockAlignmentError> {
    if blocks.len() > PBSA_MAX_BATCH {
        return Err(BlockAlignmentError::TooLargeBatch {
            got: blocks.len(),
            max: PBSA_MAX_BATCH,
        });
    }
    let mut reference: Option<usize> = None;
    for (i, b) in blocks.iter().enumerate() {
        if b.block_id == [0u8; PBSA_BLOCK_ID_LEN] {
            return Err(BlockAlignmentError::ZeroBlockId(i));
        }
        if b.block_size < PBSA_MIN_BLOCK {
            return Err(BlockAlignmentError::TooSmall {
                idx: i,
                got: b.block_size,
                min: PBSA_MIN_BLOCK,
            });
        }
        if b.block_size > PBSA_MAX_BLOCK {
            return Err(BlockAlignmentError::TooLarge {
                idx: i,
                got: b.block_size,
                max: PBSA_MAX_BLOCK,
            });
        }
        if !is_power_of_two(b.block_size) {
            return Err(BlockAlignmentError::NotPowerOfTwo {
                idx: i,
                got: b.block_size,
            });
        }
        match reference {
            None => reference = Some(b.block_size),
            Some(ref expected) => {
                if b.block_size != *expected {
                    return Err(BlockAlignmentError::Mismatch {
                        idx: i,
                        got: b.block_size,
                        expected: *expected,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBSA_BLOCK_ID_LEN] {
        [byte; PBSA_BLOCK_ID_LEN]
    }

    fn block(id: u8, size: usize) -> PaddingBlockRecord {
        PaddingBlockRecord { block_id: bid(id), block_size: size }
    }

    fn valid_batch() -> Vec<PaddingBlockRecord> {
        vec![
            block(0x01, 16),
            block(0x02, 16),
            block(0x03, 16),
        ]
    }

    /// **PBSA-01** — block too small rejected.
    #[test]
    fn pbsa_01_too_small_rejected() {
        let b = block(0x01, PBSA_MIN_BLOCK - 1);
        assert_eq!(
            validate_padding_block_alignment(&[b]),
            Err(BlockAlignmentError::TooSmall {
                idx: 0,
                got: PBSA_MIN_BLOCK - 1,
                min: PBSA_MIN_BLOCK,
            })
        );
    }

    /// **PBSA-02** — block too large rejected.
    #[test]
    fn pbsa_02_too_large_rejected() {
        let b = block(0x01, PBSA_MAX_BLOCK + 1);
        assert_eq!(
            validate_padding_block_alignment(&[b]),
            Err(BlockAlignmentError::TooLarge {
                idx: 0,
                got: PBSA_MAX_BLOCK + 1,
                max: PBSA_MAX_BLOCK,
            })
        );
    }

    /// **PBSA-03** — not power of two rejected.
    #[test]
    fn pbsa_03_not_power_of_two_rejected() {
        let b = block(0x01, 24);
        assert_eq!(
            validate_padding_block_alignment(&[b]),
            Err(BlockAlignmentError::NotPowerOfTwo { idx: 0, got: 24 })
        );
    }

    /// **PBSA-04** — block size mismatch rejected.
    #[test]
    fn pbsa_04_mismatch_rejected() {
        let bs = vec![
            block(0x01, 16),
            block(0x02, 32),
        ];
        assert_eq!(
            validate_padding_block_alignment(&bs),
            Err(BlockAlignmentError::Mismatch {
                idx: 1,
                got: 32,
                expected: 16,
            })
        );
    }

    /// **PBSA-05** — zero block ID rejected.
    #[test]
    fn pbsa_05_zero_block_id_rejected() {
        let b = PaddingBlockRecord { block_id: [0u8; PBSA_BLOCK_ID_LEN], block_size: 16 };
        assert_eq!(
            validate_padding_block_alignment(&[b]),
            Err(BlockAlignmentError::ZeroBlockId(0))
        );
    }

    /// **PBSA-06** — batch too large rejected.
    #[test]
    fn pbsa_06_too_large_batch_rejected() {
        let bs: Vec<PaddingBlockRecord> = (0..=PBSA_MAX_BATCH)
            .map(|i| {
                let mut id = [0u8; PBSA_BLOCK_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                PaddingBlockRecord { block_id: id, block_size: 16 }
            })
            .collect();
        assert_eq!(
            validate_padding_block_alignment(&bs),
            Err(BlockAlignmentError::TooLargeBatch {
                got: PBSA_MAX_BATCH + 1,
                max: PBSA_MAX_BATCH,
            })
        );
    }

    /// **PBSA-07** — valid accepted.
    #[test]
    fn pbsa_07_valid_accepted() {
        assert_eq!(validate_padding_block_alignment(&valid_batch()), Ok(()));
    }

    /// **PBSA-08** — empty accepted.
    #[test]
    fn pbsa_08_empty_accepted() {
        assert_eq!(validate_padding_block_alignment(&[]), Ok(()));
    }

    /// **PBSA-09** — minimum block size accepted.
    #[test]
    fn pbsa_09_min_block_accepted() {
        assert_eq!(validate_padding_block_alignment(&[block(0x01, PBSA_MIN_BLOCK)]), Ok(()));
    }

    /// **PBSA-10** — maximum block size accepted.
    #[test]
    fn pbsa_10_max_block_accepted() {
        assert_eq!(validate_padding_block_alignment(&[block(0x01, PBSA_MAX_BLOCK)]), Ok(()));
    }
}
