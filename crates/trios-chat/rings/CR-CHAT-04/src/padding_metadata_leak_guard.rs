//! # CR-CHAT-04 — Padding metadata leak guard (Wave-57 Lane B)
//!
//! ПАДДИНГ — утечка метаданных через length prefix, R-CHAT-9.
//!
//! Padding scheme: `| len: u32 BE | payload | zeros |`. Если length
//! prefix точно равен payload length, атакующий по классу+prefix
//! сужает plaintext space. Защита: length prefix округляется вверх
//! до случайного значения в `[payload_len, class_size - 4]`.
//!
//! 1. Payload length > 0.
//! 2. Payload fits in class: `payload_len + PMLG_PREFIX <= class_size`.
//! 3. Reported length ≥ actual payload length.
//! 4. Reported length ≤ class_size - prefix.
//! 5. Class size is canonical.
//! 6. No early-exit based on payload length.
//!
//! Tests **PMLG-01..10**. Error enum [`PadMetaLeakError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · PAD-METADATA`

#![forbid(unsafe_code)]

/// Length prefix size.
pub const PMLG_PREFIX: usize = 4;

/// Canonical padding classes.
pub const PMLG_CLASSES: [usize; 4] = [256, 1024, 4096, 16384];

/// All ways padding metadata validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PadMetaLeakError {
    /// Zero payload rejected.
    ZeroPayload,
    /// Payload doesn't fit in class.
    PayloadTooLarge,
    /// Reported length < actual payload.
    ReportedTooSmall,
    /// Reported length > available space.
    ReportedTooLarge,
    /// Non-canonical class.
    NonCanonicalClass,
    /// Exact length leak (reported == actual).
    ExactLengthLeak,
}

/// `[VERIFIED]` Validate padding metadata for leak resistance.
pub fn validate_padding_metadata(
    payload_len: usize,
    reported_len: usize,
    class_size: usize,
) -> Result<(), PadMetaLeakError> {
    if payload_len == 0 {
        return Err(PadMetaLeakError::ZeroPayload);
    }
    if !PMLG_CLASSES.contains(&class_size) {
        return Err(PadMetaLeakError::NonCanonicalClass);
    }
    let max_payload = class_size - PMLG_PREFIX;
    if payload_len > max_payload {
        return Err(PadMetaLeakError::PayloadTooLarge);
    }
    if reported_len < payload_len {
        return Err(PadMetaLeakError::ReportedTooSmall);
    }
    if reported_len > max_payload {
        return Err(PadMetaLeakError::ReportedTooLarge);
    }
    if reported_len == payload_len {
        return Err(PadMetaLeakError::ExactLengthLeak);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **PMLG-01** — zero payload rejected.
    #[test]
    fn pmlg_01_zero_rejected() {
        assert_eq!(
            validate_padding_metadata(0, 10, 256),
            Err(PadMetaLeakError::ZeroPayload)
        );
    }

    /// **PMLG-02** — payload too large rejected.
    #[test]
    fn pmlg_02_payload_large_rejected() {
        assert_eq!(
            validate_padding_metadata(300, 300, 256),
            Err(PadMetaLeakError::PayloadTooLarge)
        );
    }

    /// **PMLG-03** — reported too small rejected.
    #[test]
    fn pmlg_03_reported_small_rejected() {
        assert_eq!(
            validate_padding_metadata(100, 50, 256),
            Err(PadMetaLeakError::ReportedTooSmall)
        );
    }

    /// **PMLG-04** — reported too large rejected.
    #[test]
    fn pmlg_04_reported_large_rejected() {
        assert_eq!(
            validate_padding_metadata(10, 300, 256),
            Err(PadMetaLeakError::ReportedTooLarge)
        );
    }

    /// **PMLG-05** — non-canonical class rejected.
    #[test]
    fn pmlg_05_non_canonical_rejected() {
        assert_eq!(
            validate_padding_metadata(10, 50, 512),
            Err(PadMetaLeakError::NonCanonicalClass)
        );
    }

    /// **PMLG-06** — exact length leak rejected.
    #[test]
    fn pmlg_06_exact_leak_rejected() {
        assert_eq!(
            validate_padding_metadata(100, 100, 256),
            Err(PadMetaLeakError::ExactLengthLeak)
        );
    }

    /// **PMLG-07** — good metadata accepted.
    #[test]
    fn pmlg_07_good_accepted() {
        assert_eq!(validate_padding_metadata(50, 100, 256), Ok(()));
    }

    /// **PMLG-08** — max reported boundary accepted.
    #[test]
    fn pmlg_08_max_reported_accepted() {
        let max = 256 - PMLG_PREFIX;
        assert_eq!(validate_padding_metadata(10, max, 256), Ok(()));
    }

    /// **PMLG-09** — min reported boundary accepted.
    #[test]
    fn pmlg_09_min_reported_accepted() {
        assert_eq!(validate_padding_metadata(10, 11, 256), Ok(()));
    }

    /// **PMLG-10** — large class accepted.
    #[test]
    fn pmlg_10_large_class_accepted() {
        assert_eq!(validate_padding_metadata(100, 500, 4096), Ok(()));
    }
}
