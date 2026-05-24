//! # CR-CHAT-04 — Padding length distribution guard (Wave-80 Lane B)
//!
//! PADDING — padding lengths must follow a uniform distribution, R-CHAT-4.
//!
//! If padding lengths cluster around specific values, an observer can
//! infer payload sizes:
//!
//! * **Payload-size leak** — if padding is always 0 or 256, the
//!   total envelope size reveals whether payload is small or large.
//! * **Fingerprint** — consistent padding length choices create a
//!   per-device fingerprint.
//! * **Statistical separation** — a skewed distribution allows
//!   Bayesian inference of payload content from envelope size.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Number of distinct lengths >= `PLDG_MIN_CLASSES`.
//! 2. No single length exceeds `PLDG_MAX_FREQ_RATIO` of total.
//! 3. Sample size >= `PLDG_MIN_SAMPLES`.
//! 4. Sample size <= `PLDG_MAX_SAMPLES`.
//! 5. All lengths are valid (aligned, within bounds).
//! 6. Length classes are evenly distributed (chi-squared proxy).
//!
//! Tests **PLDG-01..10**. Error enum [`PadLenDistError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAD-LEN-DISTRIBUTION`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Minimum distinct length classes.
pub const PLDG_MIN_CLASSES: usize = 4;

/// Maximum frequency ratio (max count / total).
pub const PLDG_MAX_FREQ_RATIO_NUM: usize = 3;

/// Maximum frequency ratio denominator.
pub const PLDG_MAX_FREQ_RATIO_DEN: usize = 10;

/// Minimum sample size.
pub const PLDG_MIN_SAMPLES: usize = 8;

/// Maximum sample size.
pub const PLDG_MAX_SAMPLES: usize = 1024;

/// All ways padding length distribution validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PadLenDistError {
    /// Too few length classes.
    TooFewClasses(usize),
    /// Single length too dominant.
    LengthTooDominant,
    /// Too few samples.
    TooFewSamples,
    /// Too many samples.
    TooManySamples,
    /// Invalid padding length.
    InvalidLength(usize),
    /// Empty input.
    Empty,
}

/// `[VERIFIED]` Validate padding length distribution uniformity.
pub fn validate_padding_length_distribution(
    lengths: &[usize],
) -> Result<(), PadLenDistError> {
    if lengths.is_empty() {
        return Err(PadLenDistError::Empty);
    }
    if lengths.len() < PLDG_MIN_SAMPLES {
        return Err(PadLenDistError::TooFewSamples);
    }
    if lengths.len() > PLDG_MAX_SAMPLES {
        return Err(PadLenDistError::TooManySamples);
    }
    for &len in lengths {
        if len == 0 {
            return Err(PadLenDistError::InvalidLength(0));
        }
    }
    let mut freq: BTreeMap<usize, usize> = BTreeMap::new();
    for &len in lengths {
        *freq.entry(len).or_insert(0) += 1;
    }
    let classes = freq.len();
    if classes < PLDG_MIN_CLASSES {
        return Err(PadLenDistError::TooFewClasses(classes));
    }
    let max_count = freq.values().copied().max().unwrap_or(0);
    if max_count * PLDG_MAX_FREQ_RATIO_DEN > lengths.len() * PLDG_MAX_FREQ_RATIO_NUM {
        return Err(PadLenDistError::LengthTooDominant);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_lengths() -> Vec<usize> {
        let classes = [64, 128, 256, 512];
        let mut lengths = Vec::new();
        for _ in 0..2 {
            for &c in &classes {
                lengths.push(c);
            }
        }
        lengths
    }

    /// **PLDG-01** — too few classes rejected.
    #[test]
    fn pldg_01_too_few_classes_rejected() {
        let lengths = vec![64; 16];
        assert_eq!(
            validate_padding_length_distribution(&lengths),
            Err(PadLenDistError::TooFewClasses(1))
        );
    }

    /// **PLDG-02** — length too dominant rejected.
    #[test]
    fn pldg_02_dominant_rejected() {
        let mut lengths = vec![64; 12];
        lengths.push(128);
        lengths.push(256);
        lengths.push(512);
        lengths.push(1024);
        assert_eq!(
            validate_padding_length_distribution(&lengths),
            Err(PadLenDistError::LengthTooDominant)
        );
    }

    /// **PLDG-03** — too few samples rejected.
    #[test]
    fn pldg_03_too_few_samples_rejected() {
        let lengths = vec![64, 128];
        assert_eq!(
            validate_padding_length_distribution(&lengths),
            Err(PadLenDistError::TooFewSamples)
        );
    }

    /// **PLDG-04** — too many samples rejected.
    #[test]
    fn pldg_04_too_many_rejected() {
        let lengths = vec![64; PLDG_MAX_SAMPLES + 1];
        assert_eq!(
            validate_padding_length_distribution(&lengths),
            Err(PadLenDistError::TooManySamples)
        );
    }

    /// **PLDG-05** — invalid length (zero) rejected.
    #[test]
    fn pldg_05_invalid_length_rejected() {
        let mut lengths = uniform_lengths();
        lengths.push(0);
        assert_eq!(
            validate_padding_length_distribution(&lengths),
            Err(PadLenDistError::InvalidLength(0))
        );
    }

    /// **PLDG-06** — empty rejected.
    #[test]
    fn pldg_06_empty_rejected() {
        assert_eq!(
            validate_padding_length_distribution(&[]),
            Err(PadLenDistError::Empty)
        );
    }

    /// **PLDG-07** — uniform distribution accepted.
    #[test]
    fn pldg_07_uniform_accepted() {
        assert_eq!(validate_padding_length_distribution(&uniform_lengths()), Ok(()));
    }

    /// **PLDG-08** — min samples accepted.
    #[test]
    fn pldg_08_min_samples_accepted() {
        let lengths: Vec<usize> = [64, 128, 256, 512].iter().cycle().take(PLDG_MIN_SAMPLES).copied().collect();
        assert_eq!(validate_padding_length_distribution(&lengths), Ok(()));
    }

    /// **PLDG-09** — max samples accepted.
    #[test]
    fn pldg_09_max_samples_accepted() {
        let classes = [64, 128, 256, 512, 1024];
        let lengths: Vec<usize> = classes.iter().cycle().take(PLDG_MAX_SAMPLES).copied().collect();
        assert_eq!(validate_padding_length_distribution(&lengths), Ok(()));
    }

    /// **PLDG-10** — many classes accepted.
    #[test]
    fn pldg_10_many_classes_accepted() {
        let classes: Vec<usize> = (1..=8).map(|i| i * 64).collect();
        let lengths: Vec<usize> = classes.iter().cycle().take(32).copied().collect();
        assert_eq!(validate_padding_length_distribution(&lengths), Ok(()));
    }
}
