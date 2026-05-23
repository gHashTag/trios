//! # CR-CHAT-05 — Envelope size distribution uniformity guard (Wave-59 Lane A)
//!
//! PERSISTENCE — stored envelope sizes must not leak active/idle, R-CHAT-1.
//!
//! If real envelopes are always 1024 bytes and cover are 256, an attacker
//! with read access to the store can determine active/idle via a size
//! histogram. All stored envelopes must share a single padding class.
//!
//! 1. All envelopes share one class.
//! 2. Class is in the canonical set.
//! 3. Min envelopes for statistics >= `ESDU_MIN_ENVELOPES`.
//! 4. No envelope shorter than `ESDU_MIN_SIZE`.
//! 5. Standard deviation of sizes = 0 (all identical).
//! 6. Store size <= `ESDU_MAX_STORE`.
//!
//! Tests **ESDU-01..10**. Error enum [`SizeDistributionError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SIZE-UNIFORMITY`

#![forbid(unsafe_code)]

/// Canonical envelope sizes.
pub const ESDU_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// Minimum envelopes for statistical analysis.
pub const ESDU_MIN_ENVELOPES: usize = 4;

/// Minimum single envelope size.
pub const ESDU_MIN_SIZE: usize = 256;

/// Maximum store size.
pub const ESDU_MAX_STORE: usize = 4096;

/// All ways size distribution validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SizeDistributionError {
    /// Multiple size classes detected.
    MultipleClasses,
    /// Non-canonical size.
    NonCanonicalSize,
    /// Too few envelopes.
    TooFewEnvelopes,
    /// Envelope too small.
    EnvelopeTooSmall,
    /// Store too large.
    StoreTooLarge,
    /// Zero-size envelope.
    ZeroSize,
}

/// `[VERIFIED]` Validate that all stored envelopes share a single
/// padding class.
pub fn validate_size_uniformity(sizes: &[usize]) -> Result<(), SizeDistributionError> {
    if sizes.len() > ESDU_MAX_STORE {
        return Err(SizeDistributionError::StoreTooLarge);
    }
    if sizes.is_empty() {
        return Ok(());
    }
    let mut class: Option<usize> = None;
    for &s in sizes {
        if s == 0 {
            return Err(SizeDistributionError::ZeroSize);
        }
        if s < ESDU_MIN_SIZE {
            return Err(SizeDistributionError::EnvelopeTooSmall);
        }
        if !ESDU_CLASSES.contains(&s) {
            return Err(SizeDistributionError::NonCanonicalSize);
        }
        match class {
            None => class = Some(s),
            Some(c) if c != s => return Err(SizeDistributionError::MultipleClasses),
            _ => {}
        }
    }
    if sizes.len() < ESDU_MIN_ENVELOPES {
        return Err(SizeDistributionError::TooFewEnvelopes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **ESDU-01** — multiple classes rejected.
    #[test]
    fn esdu_01_multiple_classes_rejected() {
        assert_eq!(
            validate_size_uniformity(&[1024, 1024, 256, 1024]),
            Err(SizeDistributionError::MultipleClasses)
        );
    }

    /// **ESDU-02** — non-canonical rejected.
    #[test]
    fn esdu_02_non_canonical_rejected() {
        assert_eq!(
            validate_size_uniformity(&[512, 512, 512, 512]),
            Err(SizeDistributionError::NonCanonicalSize)
        );
    }

    /// **ESDU-03** — too few rejected.
    #[test]
    fn esdu_03_too_few_rejected() {
        assert_eq!(
            validate_size_uniformity(&[1024, 1024, 1024]),
            Err(SizeDistributionError::TooFewEnvelopes)
        );
    }

    /// **ESDU-04** — too small rejected.
    #[test]
    fn esdu_04_too_small_rejected() {
        assert_eq!(
            validate_size_uniformity(&[64, 64, 64, 64]),
            Err(SizeDistributionError::EnvelopeTooSmall)
        );
    }

    /// **ESDU-05** — store too large rejected.
    #[test]
    fn esdu_05_store_large_rejected() {
        let s = vec![1024; ESDU_MAX_STORE + 1];
        assert_eq!(
            validate_size_uniformity(&s),
            Err(SizeDistributionError::StoreTooLarge)
        );
    }

    /// **ESDU-06** — zero size rejected.
    #[test]
    fn esdu_06_zero_rejected() {
        assert_eq!(
            validate_size_uniformity(&[0, 1024, 1024, 1024]),
            Err(SizeDistributionError::ZeroSize)
        );
    }

    /// **ESDU-07** — uniform accepted.
    #[test]
    fn esdu_07_uniform_accepted() {
        assert_eq!(validate_size_uniformity(&[1024; 8]), Ok(()));
    }

    /// **ESDU-08** — empty accepted.
    #[test]
    fn esdu_08_empty_accepted() {
        assert_eq!(validate_size_uniformity(&[]), Ok(()));
    }

    /// **ESDU-09** — exact min count accepted.
    #[test]
    fn esdu_09_exact_min_accepted() {
        assert_eq!(validate_size_uniformity(&[256; ESDU_MIN_ENVELOPES]), Ok(()));
    }

    /// **ESDU-10** — large class accepted.
    #[test]
    fn esdu_10_large_class_accepted() {
        assert_eq!(validate_size_uniformity(&[16384; 10]), Ok(()));
    }
}
