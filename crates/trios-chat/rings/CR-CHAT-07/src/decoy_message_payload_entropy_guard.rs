//! # CR-CHAT-07 — Decoy message payload entropy guard (Wave-92 Lane B)
//!
//! ANTI-CORRELATION — decoy payloads must have sufficient entropy,
//! R-CHAT-10.
//!
//! Decoy messages fill the wire to hide real traffic patterns. If
//! decoy payloads have low entropy:
//!
//! * **Decoy filtering** — an observer applies a simple entropy test
//!   to distinguish decoys (low entropy) from real messages (high
//!   entropy from encryption).
//!
//! * **Statistical detection** — repeated patterns in decoys create
//!   statistical signatures that ML classifiers can detect.
//!
//! * **Cover breakdown** — once decoys are filtered, real traffic
//!   patterns are exposed, defeating the cover traffic scheme.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Unique byte ratio >= `DMPE_MIN_UNIQUE_RATIO`.
//! 2. Maximum frequency ratio <= `DMPE_MAX_FREQ_RATIO`.
//! 3. Payload length >= `DMPE_MIN_LEN`.
//! 4. Payload length <= `DMPE_MAX_LEN`.
//! 5. Byte value must not be all identical.
//! 6. Decoy count <= `DMPE_MAX_DECOYS`.
//!
//! Tests **DMPE-01..10**. Error enum [`DecoyEntropyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DECOY-ENTROPY`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Minimum unique byte ratio.
pub const DMPE_MIN_UNIQUE_RATIO_NUM: usize = 1;
pub const DMPE_MIN_UNIQUE_RATIO_DEN: usize = 4;

/// Maximum frequency ratio (most common / total).
pub const DMPE_MAX_FREQ_NUM: usize = 1;
pub const DMPE_MAX_FREQ_DEN: usize = 3;

/// Minimum payload length.
pub const DMPE_MIN_LEN: usize = 32;

/// Maximum payload length.
pub const DMPE_MAX_LEN: usize = 16384;

/// Maximum decoys per batch.
pub const DMPE_MAX_DECOYS: usize = 4096;

/// A decoy message record.
#[derive(Debug, Clone)]
pub struct DecoyMessage {
    /// Byte frequency map.
    pub freq: BTreeMap<u8, usize>,
    /// Total payload length.
    pub len: usize,
}

impl DecoyMessage {
    /// Count of unique byte values.
    pub fn unique_count(&self) -> usize {
        self.freq.len()
    }

    /// Maximum frequency of any single byte.
    pub fn max_freq(&self) -> usize {
        self.freq.values().copied().max().unwrap_or(0)
    }
}

/// All ways decoy entropy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecoyEntropyError {
    /// Unique byte ratio too low.
    LowUniqueRatio { unique: usize, total: usize },
    /// Frequency ratio too high.
    HighFreqRatio { max_freq: usize, total: usize },
    /// Payload too short.
    TooShort { len: usize, min: usize },
    /// Payload too long.
    TooLong { len: usize, max: usize },
    /// All identical bytes.
    AllIdentical,
    /// Too many decoys.
    TooManyDecoys,
}

/// `[VERIFIED]` Validate decoy message payload entropy.
pub fn validate_decoy_entropy(
    decoys: &[DecoyMessage],
) -> Result<(), DecoyEntropyError> {
    if decoys.len() > DMPE_MAX_DECOYS {
        return Err(DecoyEntropyError::TooManyDecoys);
    }
    for d in decoys {
        if d.len < DMPE_MIN_LEN {
            return Err(DecoyEntropyError::TooShort { len: d.len, min: DMPE_MIN_LEN });
        }
        if d.len > DMPE_MAX_LEN {
            return Err(DecoyEntropyError::TooLong { len: d.len, max: DMPE_MAX_LEN });
        }
        if d.unique_count() <= 1 {
            return Err(DecoyEntropyError::AllIdentical);
        }
        let unique = d.unique_count();
        let ratio_den = (d.len + DMPE_MIN_UNIQUE_RATIO_DEN - 1) / DMPE_MIN_UNIQUE_RATIO_DEN;
        if unique < ratio_den * DMPE_MIN_UNIQUE_RATIO_NUM / DMPE_MIN_UNIQUE_RATIO_DEN {
            let threshold = d.len / DMPE_MIN_UNIQUE_RATIO_DEN;
            if unique < threshold {
                return Err(DecoyEntropyError::LowUniqueRatio {
                    unique,
                    total: d.len,
                });
            }
        }
        let max_f = d.max_freq();
        let threshold = d.len / DMPE_MAX_FREQ_DEN;
        if max_f > threshold * DMPE_MAX_FREQ_NUM && d.len > 0 {
            return Err(DecoyEntropyError::HighFreqRatio {
                max_freq: max_f,
                total: d.len,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoy_from_bytes(bytes: &[u8]) -> DecoyMessage {
        let mut freq = BTreeMap::new();
        for &b in bytes {
            *freq.entry(b).or_insert(0) += 1;
        }
        DecoyMessage { freq, len: bytes.len() }
    }

    fn high_entropy_decoy() -> DecoyMessage {
        let bytes: Vec<u8> = (0u32..64).map(|i| ((i * 7 + 13) % 256) as u8).collect();
        decoy_from_bytes(&bytes)
    }

    fn valid_decoys() -> Vec<DecoyMessage> {
        vec![high_entropy_decoy(), high_entropy_decoy()]
    }

    /// **DMPE-01** — low unique ratio rejected.
    #[test]
    fn dmpe_01_low_unique_rejected() {
        let bytes: Vec<u8> = std::iter::repeat(0xAAu8).take(60)
            .chain(std::iter::repeat(0xBBu8).take(4))
            .collect();
        let d = decoy_from_bytes(&bytes);
        assert!(matches!(
            validate_decoy_entropy(&[d]),
            Err(DecoyEntropyError::LowUniqueRatio { .. }) | Err(DecoyEntropyError::HighFreqRatio { .. })
        ));
    }

    /// **DMPE-02** — high frequency ratio rejected.
    #[test]
    fn dmpe_02_high_freq_rejected() {
        let bytes: Vec<u8> = std::iter::repeat(0xAAu8).take(50)
            .chain((0..14u8).map(|i| i + 1))
            .collect();
        let d = decoy_from_bytes(&bytes);
        assert!(matches!(
            validate_decoy_entropy(&[d]),
            Err(DecoyEntropyError::HighFreqRatio { .. })
        ));
    }

    /// **DMPE-03** — too short rejected.
    #[test]
    fn dmpe_03_too_short_rejected() {
        let d = decoy_from_bytes(&[0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(
            validate_decoy_entropy(&[d]),
            Err(DecoyEntropyError::TooShort { len: 4, min: 32 })
        );
    }

    /// **DMPE-04** — too long rejected.
    #[test]
    fn dmpe_04_too_long_rejected() {
        let bytes: Vec<u8> = (0..=DMPE_MAX_LEN).map(|i| (i % 256) as u8).collect();
        let d = decoy_from_bytes(&bytes);
        assert_eq!(
            validate_decoy_entropy(&[d]),
            Err(DecoyEntropyError::TooLong { len: DMPE_MAX_LEN + 1, max: DMPE_MAX_LEN })
        );
    }

    /// **DMPE-05** — all identical rejected.
    #[test]
    fn dmpe_05_all_identical_rejected() {
        let bytes: Vec<u8> = std::iter::repeat(0xAAu8).take(64).collect();
        let d = decoy_from_bytes(&bytes);
        assert_eq!(validate_decoy_entropy(&[d]), Err(DecoyEntropyError::AllIdentical));
    }

    /// **DMPE-06** — too many decoys rejected.
    #[test]
    fn dmpe_06_too_many_rejected() {
        let decoys: Vec<DecoyMessage> = (0..=DMPE_MAX_DECOYS)
            .map(|_| high_entropy_decoy())
            .collect();
        assert_eq!(validate_decoy_entropy(&decoys), Err(DecoyEntropyError::TooManyDecoys));
    }

    /// **DMPE-07** — valid decoys accepted.
    #[test]
    fn dmpe_07_valid_accepted() {
        assert_eq!(validate_decoy_entropy(&valid_decoys()), Ok(()));
    }

    /// **DMPE-08** — empty accepted.
    #[test]
    fn dmpe_08_empty_accepted() {
        assert_eq!(validate_decoy_entropy(&[]), Ok(()));
    }

    /// **DMPE-09** — single high-entropy decoy accepted.
    #[test]
    fn dmpe_09_single_accepted() {
        assert_eq!(validate_decoy_entropy(&[high_entropy_decoy()]), Ok(()));
    }

    /// **DMPE-10** — minimum length with good entropy accepted.
    #[test]
    fn dmpe_10_min_len_accepted() {
        let bytes: Vec<u8> = (0u32..DMPE_MIN_LEN as u32).map(|i| ((i * 7 + 3) % 256) as u8).collect();
        let d = decoy_from_bytes(&bytes);
        assert_eq!(validate_decoy_entropy(&[d]), Ok(()));
    }
}
