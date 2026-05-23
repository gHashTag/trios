//! # CR-CHAT-02 — KDF label domain separation guard (Wave-63 Lane B)
//!
//! RATCHET — KDF labels must be unique per usage, R-CHAT-2.
//!
//! HKDF labels ensure domain separation: the same key material is never
//! derived for two different purposes. An attacker who finds two usages
//! with the same label can use a key from one context in another:
//!
//! * **Root key / chain key collision** — same label for both derivations
//!   means the keys are identical.
//! * **Message key reuse** — two epochs derive message keys with the
//!   same label, enabling nonce reuse.
//! * **Cross-protocol attack** — handshake and data keys share a label.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All labels in a set are unique.
//! 2. Label length >= `KLDL_MIN_LEN`.
//! 3. Label length <= `KLDL_MAX_LEN`.
//! 4. No label is empty.
//! 5. Labels are ASCII-printable.
//! 6. Max labels <= `KLDL_MAX_LABELS`.
//!
//! Tests **KLDL-01..10**. Error enum [`KdfLabelError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KDF-LABEL`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum label length.
pub const KLDL_MIN_LEN: usize = 4;

/// Maximum label length.
pub const KLDL_MAX_LEN: usize = 64;

/// Maximum labels in a set.
pub const KLDL_MAX_LABELS: usize = 32;

/// All ways KDF label validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KdfLabelError {
    /// Duplicate label.
    DuplicateLabel,
    /// Label too short.
    TooShort,
    /// Label too long.
    TooLong,
    /// Empty label.
    EmptyLabel,
    /// Non-ASCII label.
    NonAscii,
    /// Too many labels.
    TooManyLabels,
}

/// `[VERIFIED]` Validate a set of KDF labels for domain separation.
pub fn validate_kdf_labels(labels: &[&str]) -> Result<(), KdfLabelError> {
    if labels.len() > KLDL_MAX_LABELS {
        return Err(KdfLabelError::TooManyLabels);
    }
    let mut seen = BTreeSet::new();
    for label in labels {
        if label.is_empty() {
            return Err(KdfLabelError::EmptyLabel);
        }
        if label.len() < KLDL_MIN_LEN {
            return Err(KdfLabelError::TooShort);
        }
        if label.len() > KLDL_MAX_LEN {
            return Err(KdfLabelError::TooLong);
        }
        if !label.bytes().all(|b| b >= 0x20 && b <= 0x7E) {
            return Err(KdfLabelError::NonAscii);
        }
        if !seen.insert(*label) {
            return Err(KdfLabelError::DuplicateLabel);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD_LABELS: &[&str] = &[
        "trios-chat-root-key-v1",
        "trios-chat-chain-key-v1",
        "trios-chat-message-key-v1",
        "trios-chat-handshake-v1",
    ];

    /// **KLDL-01** — duplicate label rejected.
    #[test]
    fn kldl_01_duplicate_rejected() {
        assert_eq!(
            validate_kdf_labels(&["root-key", "root-key"]),
            Err(KdfLabelError::DuplicateLabel)
        );
    }

    /// **KLDL-02** — too short rejected.
    #[test]
    fn kldl_02_too_short_rejected() {
        assert_eq!(
            validate_kdf_labels(&["abc"]),
            Err(KdfLabelError::TooShort)
        );
    }

    /// **KLDL-03** — too long rejected.
    #[test]
    fn kldl_03_too_long_rejected() {
        let long = "x".repeat(KLDL_MAX_LEN + 1);
        assert_eq!(
            validate_kdf_labels(&[long.as_str()]),
            Err(KdfLabelError::TooLong)
        );
    }

    /// **KLDL-04** — empty rejected.
    #[test]
    fn kldl_04_empty_rejected() {
        assert_eq!(
            validate_kdf_labels(&[""]),
            Err(KdfLabelError::EmptyLabel)
        );
    }

    /// **KLDL-05** — non-ASCII rejected.
    #[test]
    fn kldl_05_non_ascii_rejected() {
        assert_eq!(
            validate_kdf_labels(&["key\x00v1"]),
            Err(KdfLabelError::NonAscii)
        );
    }

    /// **KLDL-06** — too many labels rejected.
    #[test]
    fn kldl_06_too_many_rejected() {
        let labels: Vec<String> = (0..=KLDL_MAX_LABELS)
            .map(|i| format!("label-{:03}", i))
            .collect();
        let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        assert_eq!(
            validate_kdf_labels(&refs),
            Err(KdfLabelError::TooManyLabels)
        );
    }

    /// **KLDL-07** — good labels accepted.
    #[test]
    fn kldl_07_good_accepted() {
        assert_eq!(validate_kdf_labels(GOOD_LABELS), Ok(()));
    }

    /// **KLDL-08** — minimum length accepted.
    #[test]
    fn kldl_08_min_len_accepted() {
        assert_eq!(validate_kdf_labels(&["root"]), Ok(()));
    }

    /// **KLDL-09** — maximum length accepted.
    #[test]
    fn kldl_09_max_len_accepted() {
        let label = "x".repeat(KLDL_MAX_LEN);
        assert_eq!(validate_kdf_labels(&[label.as_str()]), Ok(()));
    }

    /// **KLDL-10** — single label accepted.
    #[test]
    fn kldl_10_single_accepted() {
        assert_eq!(validate_kdf_labels(&["trios-chat-root-v1"]), Ok(()));
    }
}
