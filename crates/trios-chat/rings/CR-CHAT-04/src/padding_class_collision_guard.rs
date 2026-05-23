//! # CR-CHAT-04 — Padding class collision guard (Wave-46 Lane B)
//!
//! R-CHAT-9 — Padding class collision resistance.
//!
//! CR-CHAT-04 pads every outgoing envelope to one of a finite set of
//! length classes. An adversary who can observe that two different
//! plaintext lengths collapse to the *same* padding class can infer
//! they are "in the same bucket", narrowing the plaintext space.
//!
//! This module validates that the padding-class mapping is **injective**
//! over the expected plaintext range: no two distinct plaintext lengths
//! in `[1, PLAINTEXT_MAX]` map to the same class. If the class table has
//! fewer entries than distinct plaintext lengths, the mapping *must*
//! produce collisions — but we enforce that any two lengths that DO
//! collide differ by at most `MAX_COLLISION_SPAN` bytes.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Class table is non-empty.
//! 2. Class boundaries are strictly increasing.
//! 3. No class boundary exceeds `MAX_CLASS_SIZE`.
//! 4. Adjacent class boundaries differ by at least `MIN_CLASS_SPAN`.
//! 5. All class boundaries are multiples of `CLASS_ALIGNMENT`.
//! 6. Collision span within a class ≤ `MAX_COLLISION_SPAN`.
//!
//! Tests **PCLC-01..10**. Error enum [`PadClassError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PAD-CLASS-COLLISION`

#![forbid(unsafe_code)]

/// Maximum size of any single padding class.
pub const MAX_CLASS_SIZE: usize = 4096;

/// Minimum span between adjacent class boundaries.
pub const MIN_CLASS_SPAN: usize = 32;

/// Class boundary alignment (must be power of 2).
pub const CLASS_ALIGNMENT: usize = 32;

/// Maximum collision span within a single class.
pub const MAX_COLLISION_SPAN: usize = 512;

/// All ways padding class configuration can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PadClassError {
    /// Class table is empty.
    EmptyClasses,
    /// Boundaries not strictly increasing.
    NotStrictlyIncreasing,
    /// Class boundary exceeds max size.
    ExceedsMaxClassSize,
    /// Adjacent boundaries too close.
    SpanTooSmall,
    /// Boundary not aligned.
    NotAligned,
    /// Collision span within a class exceeds maximum.
    CollisionSpanTooLarge,
}

/// `[VERIFIED]` Validate a padding class table. The `classes` slice
/// contains the upper boundary (inclusive) of each class in ascending
/// order.
pub fn validate_padding_classes(classes: &[usize]) -> Result<(), PadClassError> {
    if classes.is_empty() {
        return Err(PadClassError::EmptyClasses);
    }
    for &c in classes {
        if c > MAX_CLASS_SIZE {
            return Err(PadClassError::ExceedsMaxClassSize);
        }
        if c % CLASS_ALIGNMENT != 0 {
            return Err(PadClassError::NotAligned);
        }
    }
    for window in classes.windows(2) {
        if window[1] <= window[0] {
            return Err(PadClassError::NotStrictlyIncreasing);
        }
        if window[1] - window[0] < MIN_CLASS_SPAN {
            return Err(PadClassError::SpanTooSmall);
        }
    }
    if classes.len() >= 2 {
        let first = if classes.len() == 1 { classes[0] } else { classes[0] };
        if first > MAX_COLLISION_SPAN {
            return Err(PadClassError::CollisionSpanTooLarge);
        }
    }
    for window in classes.windows(2) {
        let span = window[1] - window[0];
        if span > MAX_COLLISION_SPAN {
            return Err(PadClassError::CollisionSpanTooLarge);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_classes() -> Vec<usize> {
        vec![64, 128, 256, 512, 1024]
    }

    /// **PCLC-01** — empty classes rejected.
    #[test]
    fn pclc_01_empty_rejected() {
        assert_eq!(
            validate_padding_classes(&[]),
            Err(PadClassError::EmptyClasses)
        );
    }

    /// **PCLC-02** — not strictly increasing rejected.
    #[test]
    fn pclc_02_not_increasing_rejected() {
        assert_eq!(
            validate_padding_classes(&[128, 128]),
            Err(PadClassError::NotStrictlyIncreasing)
        );
    }

    /// **PCLC-03** — exceeds max class size rejected.
    #[test]
    fn pclc_03_exceeds_max_rejected() {
        assert_eq!(
            validate_padding_classes(&[MAX_CLASS_SIZE + 64]),
            Err(PadClassError::ExceedsMaxClassSize)
        );
    }

    /// **PCLC-04** — equal boundaries rejected (span = 0).
    #[test]
    fn pclc_04_equal_boundaries_rejected() {
        assert_eq!(
            validate_padding_classes(&[64, 64]),
            Err(PadClassError::NotStrictlyIncreasing)
        );
    }

    /// **PCLC-05** — not aligned rejected.
    #[test]
    fn pclc_05_not_aligned_rejected() {
        assert_eq!(
            validate_padding_classes(&[50]),
            Err(PadClassError::NotAligned)
        );
    }

    /// **PCLC-06** — collision span too large rejected.
    #[test]
    fn pclc_06_collision_span_rejected() {
        assert_eq!(
            validate_padding_classes(&[64, 640]),
            Err(PadClassError::CollisionSpanTooLarge)
        );
    }

    /// **PCLC-07** — good classes accepted.
    #[test]
    fn pclc_07_good_accepted() {
        assert_eq!(validate_padding_classes(&good_classes()), Ok(()));
    }

    /// **PCLC-08** — single class accepted.
    #[test]
    fn pclc_08_single_class_accepted() {
        assert_eq!(validate_padding_classes(&[64]), Ok(()));
    }

    /// **PCLC-09** — two-class exact boundary accepted.
    #[test]
    fn pclc_09_two_class_boundary_accepted() {
        assert_eq!(validate_padding_classes(&[64, 128]), Ok(()));
    }

    /// **PCLC-10** — descending rejected.
    #[test]
    fn pclc_10_descending_rejected() {
        assert_eq!(
            validate_padding_classes(&[128, 64]),
            Err(PadClassError::NotStrictlyIncreasing)
        );
    }
}
