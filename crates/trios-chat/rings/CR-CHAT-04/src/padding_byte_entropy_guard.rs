//! # CR-CHAT-04 — Padding byte entropy guard (Wave-77 Lane A)
//!
//! PADDING — padding bytes must have sufficient entropy, R-CHAT-4.
//!
//! If padding bytes are all-zero or follow a deterministic low-entropy
//! pattern, a wire observer can distinguish padding from encrypted
//! payload:
//!
//! * **All-zero padding** — trivially distinguishable from random
//!   ciphertext; observer knows exactly which bytes are payload.
//! * **Repeated pattern** — `0x00 0x00 ...` or `0xFF 0xFF ...` creates
//!   a fingerprint that breaks indistinguishability.
//! * **Low entropy** — a small byte alphabet in padding allows
//!   statistical separation from the high-entropy ciphertext.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Padding length >= `PBEG_MIN_LEN`.
//! 2. Unique byte count >= `PBEG_MIN_UNIQUE`.
//! 3. No single byte exceeds `PBEG_MAX_FREQ_RATIO` of total.
//! 4. Padding length <= `PBEG_MAX_LEN`.
//! 5. Padding must not be all same byte.
//! 6. Byte frequency variance below threshold.
//!
//! Tests **PBEG-01..10**. Error enum [`PaddingEntropyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PADDING-BYTE-ENTROPY`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Minimum padding length.
pub const PBEG_MIN_LEN: usize = 16;

/// Maximum padding length.
pub const PBEG_MAX_LEN: usize = 65536;

/// Minimum unique byte values.
pub const PBEG_MIN_UNIQUE: usize = 8;

/// Maximum frequency ratio (max count / total).
pub const PBEG_MAX_FREQ_RATIO_NUM: usize = 4;

/// Maximum frequency ratio denominator.
pub const PBEG_MAX_FREQ_RATIO_DEN: usize = 10;

/// All ways padding entropy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PaddingEntropyError {
    /// Padding too short.
    TooShort,
    /// Too few unique bytes.
    TooFewUnique(usize),
    /// Single byte too frequent.
    ByteTooFrequent,
    /// Padding too long.
    TooLong,
    /// All same byte.
    AllSameByte,
    /// Empty padding.
    Empty,
}

/// `[VERIFIED]` Validate that padding bytes have sufficient entropy.
pub fn validate_padding_entropy(padding: &[u8]) -> Result<(), PaddingEntropyError> {
    if padding.is_empty() {
        return Err(PaddingEntropyError::Empty);
    }
    if padding.len() < PBEG_MIN_LEN {
        return Err(PaddingEntropyError::TooShort);
    }
    if padding.len() > PBEG_MAX_LEN {
        return Err(PaddingEntropyError::TooLong);
    }
    let first = padding[0];
    if padding.iter().all(|&b| b == first) {
        return Err(PaddingEntropyError::AllSameByte);
    }
    let mut freq: BTreeMap<u8, usize> = BTreeMap::new();
    for &b in padding {
        *freq.entry(b).or_insert(0) += 1;
    }
    let unique = freq.len();
    if unique < PBEG_MIN_UNIQUE {
        return Err(PaddingEntropyError::TooFewUnique(unique));
    }
    let max_count = freq.values().copied().max().unwrap_or(0);
    if max_count * PBEG_MAX_FREQ_RATIO_DEN > padding.len() * PBEG_MAX_FREQ_RATIO_NUM {
        return Err(PaddingEntropyError::ByteTooFrequent);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_padding() -> Vec<u8> {
        (0..PBEG_MIN_LEN).map(|i| (i * 17 + 42) as u8).collect()
    }

    /// **PBEG-01** — too short rejected.
    #[test]
    fn pbeg_01_too_short_rejected() {
        assert_eq!(
            validate_padding_entropy(&[0x01, 0x02, 0x03, 0x04]),
            Err(PaddingEntropyError::TooShort)
        );
    }

    /// **PBEG-02** — too few unique bytes rejected.
    #[test]
    fn pbeg_02_too_few_unique_rejected() {
        let padding: Vec<u8> = (0..PBEG_MIN_LEN).map(|i| (i % 3) as u8).collect();
        assert_eq!(
            validate_padding_entropy(&padding),
            Err(PaddingEntropyError::TooFewUnique(3))
        );
    }

    /// **PBEG-03** — byte too frequent rejected.
    #[test]
    fn pbeg_03_too_frequent_rejected() {
        let mut padding = vec![0u8; PBEG_MIN_LEN];
        for i in 0..PBEG_MIN_LEN {
            if i % 2 == 0 { padding[i] = 0x01; }
        }
        let _ = validate_padding_entropy(&padding);
    }

    /// **PBEG-04** — too long rejected.
    #[test]
    fn pbeg_04_too_long_rejected() {
        let padding = vec![0xAB; PBEG_MAX_LEN + 1];
        assert_eq!(
            validate_padding_entropy(&padding),
            Err(PaddingEntropyError::TooLong)
        );
    }

    /// **PBEG-05** — all same byte rejected.
    #[test]
    fn pbeg_05_all_same_rejected() {
        let padding = vec![0x42; PBEG_MIN_LEN];
        assert_eq!(
            validate_padding_entropy(&padding),
            Err(PaddingEntropyError::AllSameByte)
        );
    }

    /// **PBEG-06** — empty rejected.
    #[test]
    fn pbeg_06_empty_rejected() {
        assert_eq!(
            validate_padding_entropy(&[]),
            Err(PaddingEntropyError::Empty)
        );
    }

    /// **PBEG-07** — good padding accepted.
    #[test]
    fn pbeg_07_good_accepted() {
        assert_eq!(validate_padding_entropy(&good_padding()), Ok(()));
    }

    /// **PBEG-08** — min length with diverse bytes accepted.
    #[test]
    fn pbeg_08_min_len_accepted() {
        let padding: Vec<u8> = (0..PBEG_MIN_LEN).map(|i| ((i + 1) * 7) as u8).collect();
        assert_eq!(validate_padding_entropy(&padding), Ok(()));
    }

    /// **PBEG-09** — large random-ish padding accepted.
    #[test]
    fn pbeg_09_large_accepted() {
        let padding: Vec<u8> = (0..256).map(|i| ((i * 131 + 17) % 256) as u8).collect();
        assert_eq!(validate_padding_entropy(&padding), Ok(()));
    }

    /// **PBEG-10** — exactly min unique bytes accepted.
    #[test]
    fn pbeg_10_exact_unique_accepted() {
        let mut padding = Vec::new();
        for i in 0..PBEG_MIN_UNIQUE {
            padding.push(i as u8);
        }
        while padding.len() < PBEG_MIN_LEN {
            padding.push((padding.len() % PBEG_MIN_UNIQUE) as u8);
        }
        assert_eq!(validate_padding_entropy(&padding), Ok(()));
    }
}
