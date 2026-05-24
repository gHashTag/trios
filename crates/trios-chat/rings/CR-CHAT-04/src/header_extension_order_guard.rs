//! # CR-CHAT-04 — Header extension order guard (Wave-72 Lane B)
//!
//! PADDING — header extensions must be in canonical order, R-CHAT-4.
//!
//! Wire message headers carry typed extensions. If extensions are not
//! in canonical order (sorted by type ID):
//!
//! * **Fingerprinting** — a unique extension order becomes a device
//!   fingerprint visible to a network observer.
//! * **Parsing ambiguity** — implementations that expect sorted
//!   extensions may misparse or skip out-of-order entries.
//! * **Duplicate extension** — two extensions of the same type create
//!   ambiguity about which one is authoritative.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Extensions sorted by ascending type ID.
//! 2. No duplicate type IDs.
//! 3. Type IDs in range `[HEXO_MIN_TYPE, HEXO_MAX_TYPE]`.
//! 4. Extension count <= `HEXO_MAX_EXTENSIONS`.
//! 5. Each extension has non-empty payload.
//! 6. Extension payload length <= `HEXO_MAX_PAYLOAD`.
//!
//! Tests **HEXO-01..10**. Error enum [`HeaderExtError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * HEADER-EXT-ORDER`

#![forbid(unsafe_code)]

/// Minimum extension type ID.
pub const HEXO_MIN_TYPE: u16 = 1;

/// Maximum extension type ID.
pub const HEXO_MAX_TYPE: u16 = 65535;

/// Maximum extensions per header.
pub const HEXO_MAX_EXTENSIONS: usize = 16;

/// Maximum payload per extension.
pub const HEXO_MAX_PAYLOAD: usize = 4096;

/// A header extension.
#[derive(Debug, Clone)]
pub struct HeaderExtension {
    /// Extension type ID.
    pub type_id: u16,
    /// Extension payload.
    pub payload: Vec<u8>,
}

/// All ways header extension validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HeaderExtError {
    /// Extensions not sorted.
    NotSorted,
    /// Duplicate type ID.
    DuplicateType(u16),
    /// Type ID out of range.
    TypeOutOfRange(u16),
    /// Too many extensions.
    TooManyExtensions,
    /// Empty payload.
    EmptyPayload(u16),
    /// Payload too large.
    PayloadTooLarge(u16),
}

/// `[VERIFIED]` Validate header extensions are in canonical order.
pub fn validate_header_ext_order(
    extensions: &[HeaderExtension],
) -> Result<(), HeaderExtError> {
    if extensions.len() > HEXO_MAX_EXTENSIONS {
        return Err(HeaderExtError::TooManyExtensions);
    }
    let mut prev_type: Option<u16> = None;
    for ext in extensions {
        if ext.type_id < HEXO_MIN_TYPE {
            return Err(HeaderExtError::TypeOutOfRange(ext.type_id));
        }
        if ext.payload.is_empty() {
            return Err(HeaderExtError::EmptyPayload(ext.type_id));
        }
        if ext.payload.len() > HEXO_MAX_PAYLOAD {
            return Err(HeaderExtError::PayloadTooLarge(ext.type_id));
        }
        if let Some(pt) = prev_type {
            if ext.type_id == pt {
                return Err(HeaderExtError::DuplicateType(ext.type_id));
            }
            if ext.type_id < pt {
                return Err(HeaderExtError::NotSorted);
            }
        }
        prev_type = Some(ext.type_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(type_id: u16, size: usize) -> HeaderExtension {
        HeaderExtension { type_id, payload: vec![0xAB; size] }
    }

    fn valid_exts() -> Vec<HeaderExtension> {
        vec![ext(1, 10), ext(5, 20), ext(100, 30)]
    }

    /// **HEXO-01** — not sorted rejected.
    #[test]
    fn hexo_01_not_sorted_rejected() {
        let exts = vec![ext(5, 10), ext(1, 10)];
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::NotSorted)
        );
    }

    /// **HEXO-02** — duplicate type rejected.
    #[test]
    fn hexo_02_duplicate_rejected() {
        let exts = vec![ext(1, 10), ext(1, 20)];
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::DuplicateType(1))
        );
    }

    /// **HEXO-03** — type out of range rejected.
    #[test]
    fn hexo_03_type_oob_rejected() {
        let exts = vec![ext(0, 10)];
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::TypeOutOfRange(0))
        );
    }

    /// **HEXO-04** — too many extensions rejected.
    #[test]
    fn hexo_04_too_many_rejected() {
        let exts: Vec<HeaderExtension> = (0..=HEXO_MAX_EXTENSIONS)
            .map(|i| ext((i + 1) as u16, 10))
            .collect();
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::TooManyExtensions)
        );
    }

    /// **HEXO-05** — empty payload rejected.
    #[test]
    fn hexo_05_empty_payload_rejected() {
        let exts = vec![HeaderExtension { type_id: 1, payload: vec![] }];
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::EmptyPayload(1))
        );
    }

    /// **HEXO-06** — payload too large rejected.
    #[test]
    fn hexo_06_payload_large_rejected() {
        let exts = vec![ext(1, HEXO_MAX_PAYLOAD + 1)];
        assert_eq!(
            validate_header_ext_order(&exts),
            Err(HeaderExtError::PayloadTooLarge(1))
        );
    }

    /// **HEXO-07** — valid extensions accepted.
    #[test]
    fn hexo_07_valid_accepted() {
        assert_eq!(validate_header_ext_order(&valid_exts()), Ok(()));
    }

    /// **HEXO-08** — single extension accepted.
    #[test]
    fn hexo_08_single_accepted() {
        assert_eq!(validate_header_ext_order(&[ext(1, 10)]), Ok(()));
    }

    /// **HEXO-09** — empty accepted.
    #[test]
    fn hexo_09_empty_accepted() {
        assert_eq!(validate_header_ext_order(&[]), Ok(()));
    }

    /// **HEXO-10** — max extensions accepted.
    #[test]
    fn hexo_10_max_accepted() {
        let exts: Vec<HeaderExtension> = (0..HEXO_MAX_EXTENSIONS)
            .map(|i| ext((i + 1) as u16, 10))
            .collect();
        assert_eq!(validate_header_ext_order(&exts), Ok(()));
    }
}
