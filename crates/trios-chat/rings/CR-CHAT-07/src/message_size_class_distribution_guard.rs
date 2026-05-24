//! # CR-CHAT-07 — Message size class distribution guard (Wave-99 Lane A)
//!
//! ANTI-CORRELATION — size class distribution must be uniform,
//! R-CHAT-10.
//!
//! Messages are padded to size classes (256, 1024, 4096, 16384). If
//! the distribution of sizes observed on the wire is skewed:
//!
//! * **Traffic analysis** — if 80% of messages are class 256, an
//!   observer infers that most messages are short (e.g. chat texts),
//!   and rare large messages are likely file transfers.
//! * **Activity fingerprinting** — the size distribution uniquely
//!   identifies a user's communication pattern across sessions.
//! * **Classification** — ML models trained on size distributions
//!   can classify message types with high accuracy.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each class count >= `MSCD_MIN_PER_CLASS`.
//! 2. No class exceeds `MSCD_MAX_RATIO` of total.
//! 3. Total observations >= `MSCD_MIN_OBSERVATIONS`.
//! 4. All classes must be present.
//! 5. Class values must be valid.
//! 6. Total <= `MSCD_MAX_OBSERVATIONS`.
//!
//! Tests **MSCD-01..10**. Error enum [`SizeDistError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIZE-DISTRIBUTION`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Minimum observations per class.
pub const MSCD_MIN_PER_CLASS: usize = 10;

/// Maximum ratio numerator for any single class.
pub const MSCD_MAX_RATIO_NUM: usize = 3;
pub const MSCD_MAX_RATIO_DEN: usize = 4;

/// Minimum total observations.
pub const MSCD_MIN_OBSERVATIONS: usize = 40;

/// Maximum observations.
pub const MSCD_MAX_OBSERVATIONS: usize = 1_000_000;

/// Valid size classes.
pub const MSCD_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// A size class observation.
#[derive(Debug, Clone)]
pub struct SizeObservation {
    /// Observed padded size.
    pub size: usize,
}

/// All ways size distribution validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SizeDistError {
    /// Class under minimum.
    UnderMinimum { class: usize, count: usize, min: usize },
    /// Dominant class.
    DominantClass { class: usize, count: usize, total: usize },
    /// Too few observations.
    TooFew { got: usize, min: usize },
    /// Missing class.
    MissingClass(usize),
    /// Invalid class.
    InvalidClass(usize),
    /// Too many observations.
    TooMany,
}

/// `[VERIFIED]` Validate message size class distribution.
pub fn validate_size_distribution(
    observations: &[SizeObservation],
) -> Result<(), SizeDistError> {
    if observations.len() > MSCD_MAX_OBSERVATIONS {
        return Err(SizeDistError::TooMany);
    }
    if observations.len() < MSCD_MIN_OBSERVATIONS {
        return Err(SizeDistError::TooFew { got: observations.len(), min: MSCD_MIN_OBSERVATIONS });
    }
    let valid: std::collections::HashSet<usize> = MSCD_CLASSES.into_iter().collect();
    let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
    for o in observations {
        if !valid.contains(&o.size) {
            return Err(SizeDistError::InvalidClass(o.size));
        }
        *counts.entry(o.size).or_insert(0) += 1;
    }
    for &c in &MSCD_CLASSES {
        if !counts.contains_key(&c) {
            return Err(SizeDistError::MissingClass(c));
        }
    }
    let total = observations.len();
    for (&class, &count) in &counts {
        if count < MSCD_MIN_PER_CLASS {
            return Err(SizeDistError::UnderMinimum { class, count, min: MSCD_MIN_PER_CLASS });
        }
        let threshold = total / MSCD_MAX_RATIO_DEN;
        if count > threshold * MSCD_MAX_RATIO_NUM {
            return Err(SizeDistError::DominantClass { class, count, total });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(size: usize) -> SizeObservation {
        SizeObservation { size }
    }

    fn balanced() -> Vec<SizeObservation> {
        let mut v = Vec::new();
        for _ in 0..15 {
            for &c in &MSCD_CLASSES {
                v.push(obs(c));
            }
        }
        v
    }

    /// **MSCD-01** — under minimum rejected.
    #[test]
    fn mscd_01_under_minimum_rejected() {
        let mut v = Vec::new();
        for &c in &MSCD_CLASSES {
            let count = if c == 256 { 3 } else { 15 };
            for _ in 0..count { v.push(obs(c)); }
        }
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistError::UnderMinimum { class: 256, count: 3, min: 10 })
        );
    }

    /// **MSCD-02** — dominant class rejected.
    #[test]
    fn mscd_02_dominant_rejected() {
        let mut v = Vec::new();
        for _ in 0..100 { v.push(obs(256)); }
        for &c in &[1024, 4096, 16384] {
            for _ in 0..10 { v.push(obs(c)); }
        }
        assert!(matches!(
            validate_size_distribution(&v),
            Err(SizeDistError::DominantClass { .. })
        ));
    }

    /// **MSCD-03** — too few rejected.
    #[test]
    fn mscd_03_too_few_rejected() {
        let v: Vec<SizeObservation> = MSCD_CLASSES.iter().map(|&c| obs(c)).collect();
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistError::TooFew { got: 4, min: 40 })
        );
    }

    /// **MSCD-04** — missing class rejected.
    #[test]
    fn mscd_04_missing_rejected() {
        let mut v = Vec::new();
        for &c in &[256, 1024, 4096] {
            for _ in 0..15 { v.push(obs(c)); }
        }
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistError::MissingClass(16384))
        );
    }

    /// **MSCD-05** — invalid class rejected.
    #[test]
    fn mscd_05_invalid_rejected() {
        let mut v = balanced();
        v.push(obs(999));
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistError::InvalidClass(999))
        );
    }

    /// **MSCD-06** — too many rejected.
    #[test]
    fn mscd_06_too_many_rejected() {
        let v: Vec<SizeObservation> = (0..=MSCD_MAX_OBSERVATIONS)
            .map(|i| obs(MSCD_CLASSES[i % 4]))
            .collect();
        assert_eq!(validate_size_distribution(&v), Err(SizeDistError::TooMany));
    }

    /// **MSCD-07** — balanced accepted.
    #[test]
    fn mscd_07_balanced_accepted() {
        assert_eq!(validate_size_distribution(&balanced()), Ok(()));
    }

    /// **MSCD-08** — minimum boundary accepted.
    #[test]
    fn mscd_08_min_boundary_accepted() {
        let mut v = Vec::new();
        for _ in 0..10 {
            for &c in &MSCD_CLASSES { v.push(obs(c)); }
        }
        assert_eq!(validate_size_distribution(&v), Ok(()));
    }

    /// **MSCD-09** — slightly imbalanced accepted.
    #[test]
    fn mscd_09_slightly_imbalanced_accepted() {
        let mut v = Vec::new();
        for &c in &MSCD_CLASSES {
            let count = if c == 256 { 20 } else { 12 };
            for _ in 0..count { v.push(obs(c)); }
        }
        assert_eq!(validate_size_distribution(&v), Ok(()));
    }

    /// **MSCD-10** — all equal accepted.
    #[test]
    fn mscd_10_equal_accepted() {
        let mut v = Vec::new();
        for _ in 0..20 {
            for &c in &MSCD_CLASSES { v.push(obs(c)); }
        }
        assert_eq!(validate_size_distribution(&v), Ok(()));
    }
}
