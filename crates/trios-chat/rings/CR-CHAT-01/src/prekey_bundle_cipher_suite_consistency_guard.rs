//! # CR-CHAT-01 — Prekey bundle cipher suite consistency guard (Wave-135 Lane A)
//!
//! IDENTITY — all prekey bundles in a batch must declare the same
//! cipher suite; mixing cipher suites enables downgrade attacks.
//!
//! Each prekey bundle carries a cipher suite identifier (e.g. X25519-
//! AES128GCM-SHA256). If a batch contains bundles with different
//! cipher suites:
//!
//! * **Downgrade attack** — an attacker could inject a weak cipher
//!   suite bundle, forcing the initiator to negotiate a weaker
//!   algorithm.
//! * **Interoperability failure** — mixed cipher suites may cause
//!   decryption failures in heterogeneous clients.
//! * **Audit gap** — inconsistent cipher suites complicate
//!   post-compromise auditing of which algorithm protected which
//!   message.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All bundles must share the same cipher suite ID.
//! 2. Cipher suite ID must not be zero.
//! 3. Bundle ID must not be zero.
//! 4. No duplicate bundle IDs.
//! 5. Batch size <= `PBCC_MAX_BATCH`.
//! 6. Reference cipher suite = first bundle's cipher suite.
//!
//! Tests **PBCC-01..10**. Error enum [`CipherSuiteConsistencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CIPHER-CONSISTENT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum batch size.
pub const PBCC_MAX_BATCH: usize = 512;

/// Bundle ID length.
pub const PBCC_BUNDLE_ID_LEN: usize = 32;

/// A prekey bundle cipher suite record.
#[derive(Debug, Clone)]
pub struct BundleCipherRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBCC_BUNDLE_ID_LEN],
    /// Cipher suite identifier.
    pub cipher_suite_id: u16,
}

/// All ways cipher suite consistency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CipherSuiteConsistencyError {
    /// Cipher suite mismatch.
    Mismatch { idx: usize, got: u16, expected: u16 },
    /// Zero cipher suite ID.
    ZeroCipherSuite(usize),
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundleId { idx: usize },
    /// Batch too large.
    TooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle cipher suite consistency.
pub fn validate_cipher_suite_consistency(
    bundles: &[BundleCipherRecord],
) -> Result<(), CipherSuiteConsistencyError> {
    if bundles.len() > PBCC_MAX_BATCH {
        return Err(CipherSuiteConsistencyError::TooLarge {
            got: bundles.len(),
            max: PBCC_MAX_BATCH,
        });
    }
    let mut seen: BTreeSet<[u8; PBCC_BUNDLE_ID_LEN]> = BTreeSet::new();
    let mut reference: Option<u16> = None;
    for (i, b) in bundles.iter().enumerate() {
        if b.cipher_suite_id == 0 {
            return Err(CipherSuiteConsistencyError::ZeroCipherSuite(i));
        }
        if b.bundle_id == [0u8; PBCC_BUNDLE_ID_LEN] {
            return Err(CipherSuiteConsistencyError::ZeroBundleId(i));
        }
        if !seen.insert(b.bundle_id) {
            return Err(CipherSuiteConsistencyError::DuplicateBundleId { idx: i });
        }
        match reference {
            None => reference = Some(b.cipher_suite_id),
            Some(ref expected) => {
                if b.cipher_suite_id != *expected {
                    return Err(CipherSuiteConsistencyError::Mismatch {
                        idx: i,
                        got: b.cipher_suite_id,
                        expected: *expected,
                    });
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBCC_BUNDLE_ID_LEN] {
        [byte; PBCC_BUNDLE_ID_LEN]
    }

    fn bundle(id: u8, cipher: u16) -> BundleCipherRecord {
        BundleCipherRecord { bundle_id: bid(id), cipher_suite_id: cipher }
    }

    fn valid_batch() -> Vec<BundleCipherRecord> {
        vec![
            bundle(0x01, 0x0001),
            bundle(0x02, 0x0001),
            bundle(0x03, 0x0001),
        ]
    }

    /// **PBCC-01** — cipher suite mismatch rejected.
    #[test]
    fn pbcc_01_mismatch_rejected() {
        let bs = vec![
            bundle(0x01, 0x0001),
            bundle(0x02, 0x0002),
        ];
        assert_eq!(
            validate_cipher_suite_consistency(&bs),
            Err(CipherSuiteConsistencyError::Mismatch {
                idx: 1,
                got: 0x0002,
                expected: 0x0001,
            })
        );
    }

    /// **PBCC-02** — zero cipher suite rejected.
    #[test]
    fn pbcc_02_zero_cipher_rejected() {
        let b = BundleCipherRecord { bundle_id: bid(0x01), cipher_suite_id: 0 };
        assert_eq!(
            validate_cipher_suite_consistency(&[b]),
            Err(CipherSuiteConsistencyError::ZeroCipherSuite(0))
        );
    }

    /// **PBCC-03** — zero bundle ID rejected.
    #[test]
    fn pbcc_03_zero_bundle_id_rejected() {
        let b = BundleCipherRecord { bundle_id: [0u8; PBCC_BUNDLE_ID_LEN], cipher_suite_id: 1 };
        assert_eq!(
            validate_cipher_suite_consistency(&[b]),
            Err(CipherSuiteConsistencyError::ZeroBundleId(0))
        );
    }

    /// **PBCC-04** — duplicate bundle ID rejected.
    #[test]
    fn pbcc_04_duplicate_rejected() {
        let bs = vec![
            bundle(0x01, 0x0001),
            bundle(0x01, 0x0001),
        ];
        assert_eq!(
            validate_cipher_suite_consistency(&bs),
            Err(CipherSuiteConsistencyError::DuplicateBundleId { idx: 1 })
        );
    }

    /// **PBCC-05** — batch too large rejected.
    #[test]
    fn pbcc_05_too_large_rejected() {
        let bs: Vec<BundleCipherRecord> = (0..=PBCC_MAX_BATCH)
            .map(|i| {
                let mut id = [0u8; PBCC_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                BundleCipherRecord { bundle_id: id, cipher_suite_id: 1 }
            })
            .collect();
        assert_eq!(
            validate_cipher_suite_consistency(&bs),
            Err(CipherSuiteConsistencyError::TooLarge {
                got: PBCC_MAX_BATCH + 1,
                max: PBCC_MAX_BATCH,
            })
        );
    }

    /// **PBCC-06** — valid accepted.
    #[test]
    fn pbcc_06_valid_accepted() {
        assert_eq!(validate_cipher_suite_consistency(&valid_batch()), Ok(()));
    }

    /// **PBCC-07** — empty accepted.
    #[test]
    fn pbcc_07_empty_accepted() {
        assert_eq!(validate_cipher_suite_consistency(&[]), Ok(()));
    }

    /// **PBCC-08** — single bundle accepted.
    #[test]
    fn pbcc_08_single_accepted() {
        assert_eq!(validate_cipher_suite_consistency(&[bundle(0x01, 0x0001)]), Ok(()));
    }

    /// **PBCC-09** — many same cipher suite accepted.
    #[test]
    fn pbcc_09_many_same_accepted() {
        let bs: Vec<BundleCipherRecord> = (0..10u8)
            .map(|i| bundle(i + 1, 0x0003))
            .collect();
        assert_eq!(validate_cipher_suite_consistency(&bs), Ok(()));
    }

    /// **PBCC-10** — mismatch detected at last element.
    #[test]
    fn pbcc_10_mismatch_at_last() {
        let mut bs: Vec<BundleCipherRecord> = (0..9u8)
            .map(|i| bundle(i + 1, 0x0001))
            .collect();
        bs.push(bundle(0x0B, 0x0099));
        assert_eq!(
            validate_cipher_suite_consistency(&bs),
            Err(CipherSuiteConsistencyError::Mismatch {
                idx: 9,
                got: 0x0099,
                expected: 0x0001,
            })
        );
    }
}
