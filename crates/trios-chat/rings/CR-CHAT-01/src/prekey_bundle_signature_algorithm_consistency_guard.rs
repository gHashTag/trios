//! # CR-CHAT-01 — Prekey bundle signature algorithm consistency guard (Wave-118 Lane A)
//!
//! IDENTITY — all prekey bundles in a batch must use the same signature
//! algorithm; mixing algorithms within a batch enables algorithm
//! confusion attacks.
//!
//! Algorithm confusion attacks exploit protocol implementations that
//! accept multiple signature algorithms:
//!
//! * **Cross-algorithm forgery** — an attacker crafts a bundle signed
//!   with a weak algorithm that validates against a strong key.
//! * **Downgrade via batch** — mixing Ed25519 and a hypothetical weak
//!   algo in one batch creates confusion about which algorithm the
//!   identity key actually uses.
//! * **Audit trail ambiguity** — different algorithms in the same
//!   batch make forensic analysis unreliable.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All bundles must use the same algorithm.
//! 2. Algorithm must be in `PBAC_APPROVED_ALGOS`.
//! 3. Bundle ID must not be zero.
//! 4. No duplicate bundle IDs.
//! 5. Public key length must be `PBAC_PK_LEN`.
//! 6. Total bundles <= `PBAC_MAX_BUNDLES`.
//!
//! Tests **PBAC-01..10**. Error enum [`AlgoConsistencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ALGO-CONSISTENT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Approved signature algorithms.
pub const PBAC_APPROVED_ALGOS: &[&str] = &["Ed25519", "Dilithium3", "Hybrid-Ed-Dil"];

/// Public key length.
pub const PBAC_PK_LEN: usize = 32;

/// Bundle ID length.
pub const PBAC_BUNDLE_ID_LEN: usize = 32;

/// Maximum bundles per batch.
pub const PBAC_MAX_BUNDLES: usize = 1024;

/// A prekey bundle with its signature algorithm.
#[derive(Debug, Clone)]
pub struct BundleAlgoRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBAC_BUNDLE_ID_LEN],
    /// Signature algorithm name.
    pub algorithm: String,
    /// Public key.
    pub public_key: [u8; PBAC_PK_LEN],
}

/// All ways algorithm consistency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AlgoConsistencyError {
    /// Algorithm mismatch within batch.
    AlgoMismatch { idx: usize, expected: String, found: String },
    /// Algorithm not approved.
    Unapproved { idx: usize, algo: String },
    /// Zero bundle ID.
    ZeroId(usize),
    /// Duplicate bundle ID.
    DuplicateId(usize),
    /// Too many bundles.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle signature algorithm consistency.
pub fn validate_algo_consistency(
    bundles: &[BundleAlgoRecord],
) -> Result<(), AlgoConsistencyError> {
    if bundles.len() > PBAC_MAX_BUNDLES {
        return Err(AlgoConsistencyError::TooMany {
            got: bundles.len(),
            max: PBAC_MAX_BUNDLES,
        });
    }
    let mut canonical_algo: Option<String> = None;
    let mut seen: BTreeSet<[u8; PBAC_BUNDLE_ID_LEN]> = BTreeSet::new();
    for (i, b) in bundles.iter().enumerate() {
        if b.bundle_id == [0u8; PBAC_BUNDLE_ID_LEN] {
            return Err(AlgoConsistencyError::ZeroId(i));
        }
        if !seen.insert(b.bundle_id) {
            return Err(AlgoConsistencyError::DuplicateId(i));
        }
        if !PBAC_APPROVED_ALGOS.contains(&b.algorithm.as_str()) {
            return Err(AlgoConsistencyError::Unapproved {
                idx: i,
                algo: b.algorithm.clone(),
            });
        }
        match &canonical_algo {
            None => canonical_algo = Some(b.algorithm.clone()),
            Some(expected) if expected != &b.algorithm => {
                return Err(AlgoConsistencyError::AlgoMismatch {
                    idx: i,
                    expected: expected.clone(),
                    found: b.algorithm.clone(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBAC_BUNDLE_ID_LEN] {
        [byte; PBAC_BUNDLE_ID_LEN]
    }

    fn pk(byte: u8) -> [u8; PBAC_PK_LEN] {
        [byte; PBAC_PK_LEN]
    }

    fn bundle(id: u8, algo: &str, key: u8) -> BundleAlgoRecord {
        BundleAlgoRecord { bundle_id: bid(id), algorithm: algo.to_string(), public_key: pk(key) }
    }

    fn valid_batch() -> Vec<BundleAlgoRecord> {
        vec![
            bundle(0x01, "Ed25519", 0xA1),
            bundle(0x02, "Ed25519", 0xA2),
            bundle(0x03, "Ed25519", 0xA3),
        ]
    }

    /// **PBAC-01** — algo mismatch rejected.
    #[test]
    fn pbac_01_algo_mismatch_rejected() {
        let bs = vec![
            bundle(0x01, "Ed25519", 0xA1),
            bundle(0x02, "Dilithium3", 0xA2),
        ];
        assert_eq!(
            validate_algo_consistency(&bs),
            Err(AlgoConsistencyError::AlgoMismatch {
                idx: 1,
                expected: "Ed25519".to_string(),
                found: "Dilithium3".to_string(),
            })
        );
    }

    /// **PBAC-02** — unapproved algo rejected.
    #[test]
    fn pbac_02_unapproved_rejected() {
        let bs = vec![bundle(0x01, "RSA-2048", 0xA1)];
        assert_eq!(
            validate_algo_consistency(&bs),
            Err(AlgoConsistencyError::Unapproved {
                idx: 0,
                algo: "RSA-2048".to_string(),
            })
        );
    }

    /// **PBAC-03** — zero ID rejected.
    #[test]
    fn pbac_03_zero_id_rejected() {
        let b = BundleAlgoRecord { bundle_id: [0u8; PBAC_BUNDLE_ID_LEN], algorithm: "Ed25519".to_string(), public_key: pk(0xAA) };
        assert_eq!(
            validate_algo_consistency(&[b]),
            Err(AlgoConsistencyError::ZeroId(0))
        );
    }

    /// **PBAC-04** — duplicate ID rejected.
    #[test]
    fn pbac_04_duplicate_id_rejected() {
        let bs = vec![
            bundle(0x01, "Ed25519", 0xA1),
            bundle(0x01, "Ed25519", 0xA2),
        ];
        assert_eq!(
            validate_algo_consistency(&bs),
            Err(AlgoConsistencyError::DuplicateId(1))
        );
    }

    /// **PBAC-05** — too many rejected.
    #[test]
    fn pbac_05_too_many_rejected() {
        let bs: Vec<BundleAlgoRecord> = (0..=PBAC_MAX_BUNDLES)
            .map(|i| {
                let mut id = [0u8; PBAC_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                BundleAlgoRecord { bundle_id: id, algorithm: "Ed25519".to_string(), public_key: pk(val as u8) }
            })
            .collect();
        assert_eq!(
            validate_algo_consistency(&bs),
            Err(AlgoConsistencyError::TooMany {
                got: PBAC_MAX_BUNDLES + 1,
                max: PBAC_MAX_BUNDLES,
            })
        );
    }

    /// **PBAC-06** — valid accepted.
    #[test]
    fn pbac_06_valid_accepted() {
        assert_eq!(validate_algo_consistency(&valid_batch()), Ok(()));
    }

    /// **PBAC-07** — empty accepted.
    #[test]
    fn pbac_07_empty_accepted() {
        assert_eq!(validate_algo_consistency(&[]), Ok(()));
    }

    /// **PBAC-08** — single accepted.
    #[test]
    fn pbac_08_single_accepted() {
        let bs = vec![bundle(0x01, "Ed25519", 0xAA)];
        assert_eq!(validate_algo_consistency(&bs), Ok(()));
    }

    /// **PBAC-09** — Dilithium3 batch accepted.
    #[test]
    fn pbac_09_dilithium3_accepted() {
        let bs = vec![
            bundle(0x01, "Dilithium3", 0xA1),
            bundle(0x02, "Dilithium3", 0xA2),
        ];
        assert_eq!(validate_algo_consistency(&bs), Ok(()));
    }

    /// **PBAC-10** — Hybrid-Ed-Dil batch accepted.
    #[test]
    fn pbac_10_hybrid_accepted() {
        let bs = vec![
            bundle(0x01, "Hybrid-Ed-Dil", 0xA1),
            bundle(0x02, "Hybrid-Ed-Dil", 0xA2),
        ];
        assert_eq!(validate_algo_consistency(&bs), Ok(()));
    }
}
