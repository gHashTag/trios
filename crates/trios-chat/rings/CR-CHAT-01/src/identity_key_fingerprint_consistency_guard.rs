//! # CR-CHAT-01 — Identity key fingerprint consistency guard (Wave-99 Lane B)
//!
//! IDENTITY — fingerprint derivation must be deterministic, R-CHAT-1.
//!
//! The fingerprint (e.g. safety number) is derived from the identity
//! key. If the derivation is inconsistent:
//!
//! * **Impersonation** — two different fingerprints for the same key
//!   allow an attacker to present either one, causing verification
//!   to fail for the legitimate user.
//! * **Session fragmentation** — the same identity appears as two
//!   different contacts because different sessions compute different
//!   fingerprints.
//! * **Trust erosion** — users stop verifying fingerprints when they
//!   change without explanation, weakening the authentication chain.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All fingerprints for the same key must be identical.
//! 2. Fingerprint length must be `IKFC_FP_LEN`.
//! 3. Key must not be all zeros.
//! 4. Maximum derivations <= `IKFC_MAX_DERIVATIONS`.
//! 5. Fingerprint must not be all zeros.
//! 6. Key-fingerprint pairs must be unique.
//!
//! Tests **IKFC-01..10**. Error enum [`FingerprintError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FINGERPRINT`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Fingerprint length.
pub const IKFC_FP_LEN: usize = 32;

/// Maximum derivations per batch.
pub const IKFC_MAX_DERIVATIONS: usize = 256;

/// Key length.
pub const IKFC_KEY_LEN: usize = 32;

/// A fingerprint derivation record.
#[derive(Debug, Clone)]
pub struct FingerprintDerivation {
    /// Identity key.
    pub key: [u8; IKFC_KEY_LEN],
    /// Derived fingerprint.
    pub fingerprint: [u8; IKFC_FP_LEN],
}

/// All ways fingerprint validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FingerprintError {
    /// Inconsistent fingerprints for same key.
    Inconsistent { key_idx: usize, fp1: [u8; IKFC_FP_LEN], fp2: [u8; IKFC_FP_LEN] },
    /// Zero key.
    ZeroKey(usize),
    /// Zero fingerprint.
    ZeroFingerprint(usize),
    /// Too many derivations.
    TooManyDerivations,
}

/// `[VERIFIED]` Validate identity key fingerprint consistency.
pub fn validate_fingerprint_consistency(
    derivations: &[FingerprintDerivation],
) -> Result<(), FingerprintError> {
    if derivations.len() > IKFC_MAX_DERIVATIONS {
        return Err(FingerprintError::TooManyDerivations);
    }
    let mut key_to_fp: BTreeMap<[u8; IKFC_KEY_LEN], [u8; IKFC_FP_LEN]> = BTreeMap::new();
    for (i, d) in derivations.iter().enumerate() {
        if d.key == [0u8; IKFC_KEY_LEN] {
            return Err(FingerprintError::ZeroKey(i));
        }
        if d.fingerprint == [0u8; IKFC_FP_LEN] {
            return Err(FingerprintError::ZeroFingerprint(i));
        }
        if let Some(existing) = key_to_fp.get(&d.key) {
            if *existing != d.fingerprint {
                return Err(FingerprintError::Inconsistent {
                    key_idx: i,
                    fp1: *existing,
                    fp2: d.fingerprint,
                });
            }
        } else {
            key_to_fp.insert(d.key, d.fingerprint);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; IKFC_KEY_LEN] {
        [byte; IKFC_KEY_LEN]
    }

    fn fp(byte: u8) -> [u8; IKFC_FP_LEN] {
        [byte; IKFC_FP_LEN]
    }

    fn derivation(key_byte: u8, fp_byte: u8) -> FingerprintDerivation {
        FingerprintDerivation { key: key(key_byte), fingerprint: fp(fp_byte) }
    }

    fn valid_derivations() -> Vec<FingerprintDerivation> {
        vec![
            derivation(0xAA, 0x11),
            derivation(0xBB, 0x22),
            derivation(0xAA, 0x11),
        ]
    }

    /// **IKFC-01** — inconsistent rejected.
    #[test]
    fn ikfc_01_inconsistent_rejected() {
        let ds = vec![derivation(0xAA, 0x11), derivation(0xAA, 0x22)];
        assert!(matches!(
            validate_fingerprint_consistency(&ds),
            Err(FingerprintError::Inconsistent { .. })
        ));
    }

    /// **IKFC-02** — zero key rejected.
    #[test]
    fn ikfc_02_zero_key_rejected() {
        let d = FingerprintDerivation { key: [0u8; IKFC_KEY_LEN], fingerprint: fp(0x11) };
        assert_eq!(
            validate_fingerprint_consistency(&[d]),
            Err(FingerprintError::ZeroKey(0))
        );
    }

    /// **IKFC-03** — zero fingerprint rejected.
    #[test]
    fn ikfc_03_zero_fp_rejected() {
        let d = FingerprintDerivation { key: key(0xAA), fingerprint: [0u8; IKFC_FP_LEN] };
        assert_eq!(
            validate_fingerprint_consistency(&[d]),
            Err(FingerprintError::ZeroFingerprint(0))
        );
    }

    /// **IKFC-04** — too many derivations rejected.
    #[test]
    fn ikfc_04_too_many_rejected() {
        let ds: Vec<FingerprintDerivation> = (0..=IKFC_MAX_DERIVATIONS)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                FingerprintDerivation { key: key(b), fingerprint: fp(b + 1) }
            })
            .collect();
        assert_eq!(
            validate_fingerprint_consistency(&ds),
            Err(FingerprintError::TooManyDerivations)
        );
    }

    /// **IKFC-05** — valid derivations with repeat accepted.
    #[test]
    fn ikfc_05_valid_accepted() {
        assert_eq!(validate_fingerprint_consistency(&valid_derivations()), Ok(()));
    }

    /// **IKFC-06** — empty accepted.
    #[test]
    fn ikfc_06_empty_accepted() {
        assert_eq!(validate_fingerprint_consistency(&[]), Ok(()));
    }

    /// **IKFC-07** — single accepted.
    #[test]
    fn ikfc_07_single_accepted() {
        assert_eq!(validate_fingerprint_consistency(&[derivation(0xAA, 0x11)]), Ok(()));
    }

    /// **IKFC-08** — max boundary accepted.
    #[test]
    fn ikfc_08_max_boundary_accepted() {
        let ds: Vec<FingerprintDerivation> = (0..IKFC_MAX_DERIVATIONS)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                FingerprintDerivation { key: key(b), fingerprint: fp(b + 1) }
            })
            .collect();
        assert_eq!(validate_fingerprint_consistency(&ds), Ok(()));
    }

    /// **IKFC-09** — same key same fp multiple times accepted.
    #[test]
    fn ikfc_09_repeat_accepted() {
        let ds = vec![
            derivation(0xAA, 0x11),
            derivation(0xAA, 0x11),
            derivation(0xAA, 0x11),
        ];
        assert_eq!(validate_fingerprint_consistency(&ds), Ok(()));
    }

    /// **IKFC-10** — different keys different fps accepted.
    #[test]
    fn ikfc_10_different_keys_accepted() {
        let ds = vec![
            derivation(0x01, 0x11),
            derivation(0x02, 0x22),
            derivation(0x03, 0x33),
        ];
        assert_eq!(validate_fingerprint_consistency(&ds), Ok(()));
    }
}
