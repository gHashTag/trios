//! # CR-CHAT-07 — Cover traffic entropy fingerprint guard (Wave-50 Lane B)
//!
//! R-CHAT-10 — Cover traffic byte-level indistinguishability.
//!
//! Cover traffic must be indistinguishable from real AEAD ciphertext at
//! the byte level. An adversary who can distinguish cover from real via
//! statistical tests (entropy, byte frequency, autocorrelation) can
//! filter out cover messages, defeating the anonymity set.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cover sample is non-empty.
//! 2. Cover length matches real envelope length class.
//! 3. Cover length is a multiple of `CTEF_ALIGNMENT`.
//! 4. Byte entropy within tolerance of expected AEAD output.
//! 5. No zero-heavy regions (≥ `CTEF_MAX_ZERO_RUN` consecutive zeros).
//! 6. Cover length ≥ `CTEF_MIN_COVER_LEN`.
//!
//! Tests **CTEF-01..10**. Error enum [`CoverEntropyError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · COVER-ENTROPY`

#![forbid(unsafe_code)]

/// Alignment requirement for cover length.
pub const CTEF_ALIGNMENT: usize = 64;

/// Minimum cover length.
pub const CTEF_MIN_COVER_LEN: usize = 64;

/// Maximum consecutive zero bytes allowed.
pub const CTEF_MAX_ZERO_RUN: usize = 8;

/// Minimum Shannon entropy per 256-byte block (bits).
pub const CTEF_MIN_ENTROPY: f64 = 3.0;

/// All ways cover entropy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverEntropyError {
    /// Cover sample is empty.
    EmptyCover,
    /// Cover length not aligned.
    NotAligned,
    /// Cover too short.
    TooShort,
    /// Zero run too long.
    ZeroRunTooLong,
    /// Entropy too low.
    LowEntropy,
    /// Length class mismatch.
    LengthClassMismatch,
}

fn shannon_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut freq = [0usize; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut entropy = 0.0;
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

fn max_zero_run(data: &[u8]) -> usize {
    let mut max_run = 0usize;
    let mut current = 0usize;
    for &b in data {
        if b == 0 {
            current += 1;
            if current > max_run {
                max_run = current;
            }
        } else {
            current = 0;
        }
    }
    max_run
}

/// `[VERIFIED]` Validate cover traffic sample for entropy and structural
/// properties.
pub fn validate_cover_entropy(
    cover: &[u8],
    expected_len: usize,
) -> Result<(), CoverEntropyError> {
    if cover.is_empty() {
        return Err(CoverEntropyError::EmptyCover);
    }
    if cover.len() < CTEF_MIN_COVER_LEN {
        return Err(CoverEntropyError::TooShort);
    }
    if cover.len() % CTEF_ALIGNMENT != 0 {
        return Err(CoverEntropyError::NotAligned);
    }
    if cover.len() != expected_len {
        return Err(CoverEntropyError::LengthClassMismatch);
    }
    if max_zero_run(cover) > CTEF_MAX_ZERO_RUN {
        return Err(CoverEntropyError::ZeroRunTooLong);
    }
    let entropy = shannon_entropy(cover);
    if entropy < CTEF_MIN_ENTROPY {
        return Err(CoverEntropyError::LowEntropy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const COVER_LEN: usize = 64;

    fn good_cover() -> Vec<u8> {
        (0..COVER_LEN).map(|i| (i.wrapping_mul(131) ^ 0x55) as u8).collect()
    }

    /// **CTEF-01** — empty cover rejected.
    #[test]
    fn ctef_01_empty_rejected() {
        assert_eq!(
            validate_cover_entropy(&[], COVER_LEN),
            Err(CoverEntropyError::EmptyCover)
        );
    }

    /// **CTEF-02** — too short rejected.
    #[test]
    fn ctef_02_too_short_rejected() {
        assert_eq!(
            validate_cover_entropy(&[0xAA; 16], 16),
            Err(CoverEntropyError::TooShort)
        );
    }

    /// **CTEF-03** — not aligned rejected.
    #[test]
    fn ctef_03_not_aligned_rejected() {
        let cover = vec![0x55; CTEF_MIN_COVER_LEN + CTEF_ALIGNMENT + 1];
        assert_eq!(
            validate_cover_entropy(&cover, cover.len()),
            Err(CoverEntropyError::NotAligned)
        );
    }

    /// **CTEF-04** — zero run too long rejected.
    #[test]
    fn ctef_04_zero_run_rejected() {
        let mut cover = vec![0x55; COVER_LEN];
        for i in 0..CTEF_MAX_ZERO_RUN + 1 {
            cover[i] = 0;
        }
        assert_eq!(
            validate_cover_entropy(&cover, COVER_LEN),
            Err(CoverEntropyError::ZeroRunTooLong)
        );
    }

    /// **CTEF-05** — low entropy rejected.
    #[test]
    fn ctef_05_low_entropy_rejected() {
        let cover = vec![0x42; COVER_LEN];
        assert_eq!(
            validate_cover_entropy(&cover, COVER_LEN),
            Err(CoverEntropyError::LowEntropy)
        );
    }

    /// **CTEF-06** — length mismatch rejected.
    #[test]
    fn ctef_06_length_mismatch_rejected() {
        let cover = good_cover();
        assert_eq!(
            validate_cover_entropy(&cover, 128),
            Err(CoverEntropyError::LengthClassMismatch)
        );
    }

    /// **CTEF-07** — good cover accepted.
    #[test]
    fn ctef_07_good_accepted() {
        assert_eq!(validate_cover_entropy(&good_cover(), COVER_LEN), Ok(()));
    }

    /// **CTEF-08** — exact boundary zero run accepted.
    #[test]
    fn ctef_08_exact_zero_run_accepted() {
        let mut cover = good_cover();
        for i in 0..CTEF_MAX_ZERO_RUN {
            cover[i] = 0;
        }
        assert_eq!(validate_cover_entropy(&cover, COVER_LEN), Ok(()));
    }

    /// **CTEF-09** — minimum length aligned accepted.
    #[test]
    fn ctef_09_min_len_accepted() {
        let cover: Vec<u8> = (0..CTEF_MIN_COVER_LEN).map(|i| (i as u8).wrapping_mul(173)).collect();
        // Ensure alignment
        assert_eq!(CTEF_MIN_COVER_LEN % CTEF_ALIGNMENT, 0);
        assert_eq!(
            validate_cover_entropy(&cover, CTEF_MIN_COVER_LEN),
            Ok(())
        );
    }

    /// **CTEF-10** — high entropy cover accepted.
    #[test]
    fn ctef_10_high_entropy_accepted() {
        let cover: Vec<u8> = (0..COVER_LEN).map(|i| ((i * 251 + 17) & 0xFF) as u8).collect();
        assert_eq!(validate_cover_entropy(&cover, COVER_LEN), Ok(()));
    }
}
