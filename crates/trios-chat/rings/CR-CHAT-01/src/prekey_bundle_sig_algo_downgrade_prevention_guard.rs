//! # CR-CHAT-01 — Prekey bundle signature algorithm downgrade prevention guard (Wave-103 Lane B)
//!
//! IDENTITY — signature algorithms must not be downgraded across bundles.
//!
//! When a client updates their prekey bundle, the signature algorithm
//! must not be weaker than the previous bundle's:
//!
//! * **Algorithm downgrade attack** — an attacker replaces Ed25519
//!   with a weaker algorithm in a new bundle, then forges signatures
//!   to impersonate the user.
//! * **Protocol confusion** — different bundles for the same identity
//!   using different algorithms cause verification failures in
//!   multi-device setups.
//! * **Rollback** — downgrading from post-quantum to classical-only
//!   signatures exposes the bundle to future quantum attacks.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Algorithm strength must be non-decreasing across bundles.
//! 2. Bundle index must be strictly increasing.
//! 3. Bundle index must not be zero.
//! 4. No duplicate bundle indices.
//! 5. Algorithm must be in approved set.
//! 6. Total bundles <= `SADP_MAX_BUNDLES`.
//!
//! Tests **SADP-01..10**. Error enum [`SigDowngradeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * NO-SIG-DOWNGRADE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bundles per batch.
pub const SADP_MAX_BUNDLES: usize = 256;

/// Algorithm strength levels (higher = stronger).
pub const ALGO_ED25519: u8 = 1;
pub const ALGO_DILITHIUM3: u8 = 2;
pub const ALGO_HYBRID_ED_DIL: u8 = 3;

/// Approved algorithms.
pub const SADP_APPROVED: [u8; 3] = [ALGO_ED25519, ALGO_DILITHIUM3, ALGO_HYBRID_ED_DIL];

/// A prekey bundle's signature algorithm record.
#[derive(Debug, Clone)]
pub struct BundleAlgo {
    /// Bundle index (monotonically increasing).
    pub index: u64,
    /// Signature algorithm strength level.
    pub algo: u8,
}

/// All ways signature downgrade prevention can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SigDowngradeError {
    /// Algorithm downgraded.
    Downgrade { idx: usize, prev: u8, current: u8 },
    /// Not increasing index.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero index.
    ZeroIndex(usize),
    /// Duplicate index.
    DuplicateIndex(usize),
    /// Unapproved algorithm.
    UnapprovedAlgo { idx: usize, algo: u8 },
    /// Too many bundles.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle signature algorithm downgrade prevention.
pub fn validate_sig_algo_downgrade_prevention(
    bundles: &[BundleAlgo],
) -> Result<(), SigDowngradeError> {
    if bundles.len() > SADP_MAX_BUNDLES {
        return Err(SigDowngradeError::TooMany {
            got: bundles.len(),
            max: SADP_MAX_BUNDLES,
        });
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut prev_algo: u8 = 0;
    let mut prev_index: u64 = 0;
    for (i, b) in bundles.iter().enumerate() {
        if b.index == 0 {
            return Err(SigDowngradeError::ZeroIndex(i));
        }
        if !SADP_APPROVED.contains(&b.algo) {
            return Err(SigDowngradeError::UnapprovedAlgo { idx: i, algo: b.algo });
        }
        if !seen.insert(b.index) {
            return Err(SigDowngradeError::DuplicateIndex(i));
        }
        if i > 0 {
            if b.index <= prev_index {
                return Err(SigDowngradeError::NotIncreasing {
                    idx: i,
                    prev: prev_index,
                    current: b.index,
                });
            }
            if b.algo < prev_algo {
                return Err(SigDowngradeError::Downgrade {
                    idx: i,
                    prev: prev_algo,
                    current: b.algo,
                });
            }
        }
        prev_index = b.index;
        prev_algo = b.algo;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(index: u64, algo: u8) -> BundleAlgo {
        BundleAlgo { index, algo }
    }

    fn valid_bundles() -> Vec<BundleAlgo> {
        vec![
            bundle(1, ALGO_ED25519),
            bundle(2, ALGO_ED25519),
            bundle(3, ALGO_DILITHIUM3),
        ]
    }

    /// **SADP-01** — downgrade rejected.
    #[test]
    fn sadp_01_downgrade_rejected() {
        let bs = vec![bundle(1, ALGO_DILITHIUM3), bundle(2, ALGO_ED25519)];
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&bs),
            Err(SigDowngradeError::Downgrade {
                idx: 1,
                prev: ALGO_DILITHIUM3,
                current: ALGO_ED25519,
            })
        );
    }

    /// **SADP-02** — not increasing rejected.
    #[test]
    fn sadp_02_not_increasing_rejected() {
        let bs = vec![bundle(5, ALGO_ED25519), bundle(3, ALGO_ED25519)];
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&bs),
            Err(SigDowngradeError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **SADP-03** — zero index rejected.
    #[test]
    fn sadp_03_zero_index_rejected() {
        let b = BundleAlgo { index: 0, algo: ALGO_ED25519 };
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&[b]),
            Err(SigDowngradeError::ZeroIndex(0))
        );
    }

    /// **SADP-04** — duplicate index rejected.
    #[test]
    fn sadp_04_duplicate_index_rejected() {
        let bs = vec![bundle(1, ALGO_ED25519), bundle(1, ALGO_DILITHIUM3)];
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&bs),
            Err(SigDowngradeError::DuplicateIndex(1))
        );
    }

    /// **SADP-05** — unapproved algorithm rejected.
    #[test]
    fn sadp_05_unapproved_rejected() {
        let b = BundleAlgo { index: 1, algo: 99 };
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&[b]),
            Err(SigDowngradeError::UnapprovedAlgo { idx: 0, algo: 99 })
        );
    }

    /// **SADP-06** — too many rejected.
    #[test]
    fn sadp_06_too_many_rejected() {
        let bs: Vec<BundleAlgo> = (0..=SADP_MAX_BUNDLES)
            .map(|i| BundleAlgo { index: (i as u64) + 1, algo: ALGO_ED25519 })
            .collect();
        assert_eq!(
            validate_sig_algo_downgrade_prevention(&bs),
            Err(SigDowngradeError::TooMany {
                got: SADP_MAX_BUNDLES + 1,
                max: SADP_MAX_BUNDLES,
            })
        );
    }

    /// **SADP-07** — valid accepted.
    #[test]
    fn sadp_07_valid_accepted() {
        assert_eq!(validate_sig_algo_downgrade_prevention(&valid_bundles()), Ok(()));
    }

    /// **SADP-08** — empty accepted.
    #[test]
    fn sadp_08_empty_accepted() {
        assert_eq!(validate_sig_algo_downgrade_prevention(&[]), Ok(()));
    }

    /// **SADP-09** — upgrade accepted (same algo).
    #[test]
    fn sadp_09_same_algo_accepted() {
        let bs = vec![bundle(1, ALGO_ED25519), bundle(2, ALGO_ED25519)];
        assert_eq!(validate_sig_algo_downgrade_prevention(&bs), Ok(()));
    }

    /// **SADP-10** — hybrid upgrade accepted.
    #[test]
    fn sadp_10_hybrid_upgrade_accepted() {
        let bs = vec![
            bundle(1, ALGO_ED25519),
            bundle(2, ALGO_DILITHIUM3),
            bundle(3, ALGO_HYBRID_ED_DIL),
        ];
        assert_eq!(validate_sig_algo_downgrade_prevention(&bs), Ok(()));
    }
}
