//! # CR-CHAT-01 — Prekey bundle lifetime enforcement guard (Wave-110 Lane B)
//!
//! IDENTITY — prekey bundles must not exceed maximum lifetime.
//!
//! Each prekey bundle has a creation timestamp and a maximum lifetime.
//! If a bundle is used beyond its lifetime:
//!
//! * **Key compromise** — the longer a key is in use, the more
//!   ciphertexts are available for cryptanalysis.
//! * **Algorithm obsolescence** — old bundles may use algorithms
//!   that have been broken since their creation.
//! * **Replay window** — expired bundles have a wider replay window
//!   because more sessions may have been established with them.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Bundle age <= `PBLE_MAX_LIFETIME_MS`.
//! 2. Creation timestamp must be > 0.
//! 3. Bundle ID must not be zero.
//! 4. No duplicate bundle IDs.
//! 5. Current time must be >= creation time.
//! 6. Total bundles <= `PBLE_MAX_BUNDLES`.
//!
//! Tests **PBLE-01..10**. Error enum [`BundleLifetimeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BUNDLE-LIFETIME`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bundle lifetime in milliseconds (30 days).
pub const PBLE_MAX_LIFETIME_MS: u64 = 2_592_000_000;

/// Maximum bundles per batch.
pub const PBLE_MAX_BUNDLES: usize = 256;

/// Bundle ID length.
pub const PBLE_BUNDLE_ID_LEN: usize = 16;

/// A prekey bundle lifetime record.
#[derive(Debug, Clone)]
pub struct BundleLifetime {
    /// Bundle identifier.
    pub bundle_id: [u8; PBLE_BUNDLE_ID_LEN],
    /// Creation timestamp (ms since epoch).
    pub created_at: u64,
    /// Current timestamp (ms since epoch).
    pub now_ms: u64,
}

/// All ways bundle lifetime validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleLifetimeError {
    /// Bundle expired.
    Expired { idx: usize, age_ms: u64, max_ms: u64 },
    /// Zero creation timestamp.
    ZeroTimestamp(usize),
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundle(usize),
    /// Current time before creation.
    TimeTravel { idx: usize, created: u64, now: u64 },
    /// Too many bundles.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle lifetime enforcement.
pub fn validate_bundle_lifetimes(
    bundles: &[BundleLifetime],
) -> Result<(), BundleLifetimeError> {
    if bundles.len() > PBLE_MAX_BUNDLES {
        return Err(BundleLifetimeError::TooMany {
            got: bundles.len(),
            max: PBLE_MAX_BUNDLES,
        });
    }
    let mut seen: BTreeSet<[u8; PBLE_BUNDLE_ID_LEN]> = BTreeSet::new();
    for (i, b) in bundles.iter().enumerate() {
        if b.bundle_id == [0u8; PBLE_BUNDLE_ID_LEN] {
            return Err(BundleLifetimeError::ZeroBundleId(i));
        }
        if b.created_at == 0 {
            return Err(BundleLifetimeError::ZeroTimestamp(i));
        }
        if b.now_ms < b.created_at {
            return Err(BundleLifetimeError::TimeTravel {
                idx: i,
                created: b.created_at,
                now: b.now_ms,
            });
        }
        let age = b.now_ms - b.created_at;
        if age > PBLE_MAX_LIFETIME_MS {
            return Err(BundleLifetimeError::Expired {
                idx: i,
                age_ms: age,
                max_ms: PBLE_MAX_LIFETIME_MS,
            });
        }
        if !seen.insert(b.bundle_id) {
            return Err(BundleLifetimeError::DuplicateBundle(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBLE_BUNDLE_ID_LEN] {
        [byte; PBLE_BUNDLE_ID_LEN]
    }

    fn bundle(id: u8, created: u64, now: u64) -> BundleLifetime {
        BundleLifetime { bundle_id: bid(id), created_at: created, now_ms: now }
    }

    fn valid_bundles() -> Vec<BundleLifetime> {
        vec![
            bundle(0x01, 1000, 2000),
            bundle(0x02, 1500, 2000),
            bundle(0x03, 1800, 2000),
        ]
    }

    /// **PBLE-01** — expired rejected.
    #[test]
    fn pble_01_expired_rejected() {
        let b = bundle(0x01, 100, PBLE_MAX_LIFETIME_MS + 200);
        assert_eq!(
            validate_bundle_lifetimes(&[b]),
            Err(BundleLifetimeError::Expired {
                idx: 0,
                age_ms: PBLE_MAX_LIFETIME_MS + 100,
                max_ms: PBLE_MAX_LIFETIME_MS,
            })
        );
    }

    /// **PBLE-02** — zero timestamp rejected.
    #[test]
    fn pble_02_zero_timestamp_rejected() {
        let b = BundleLifetime { bundle_id: bid(0x01), created_at: 0, now_ms: 2000 };
        assert_eq!(
            validate_bundle_lifetimes(&[b]),
            Err(BundleLifetimeError::ZeroTimestamp(0))
        );
    }

    /// **PBLE-03** — zero bundle ID rejected.
    #[test]
    fn pble_03_zero_bundle_id_rejected() {
        let b = BundleLifetime { bundle_id: [0u8; PBLE_BUNDLE_ID_LEN], created_at: 1000, now_ms: 2000 };
        assert_eq!(
            validate_bundle_lifetimes(&[b]),
            Err(BundleLifetimeError::ZeroBundleId(0))
        );
    }

    /// **PBLE-04** — duplicate rejected.
    #[test]
    fn pble_04_duplicate_rejected() {
        let bs = vec![bundle(0x01, 1000, 2000), bundle(0x01, 1500, 2000)];
        assert_eq!(
            validate_bundle_lifetimes(&bs),
            Err(BundleLifetimeError::DuplicateBundle(1))
        );
    }

    /// **PBLE-05** — time travel rejected.
    #[test]
    fn pble_05_time_travel_rejected() {
        let b = bundle(0x01, 2000, 1000);
        assert_eq!(
            validate_bundle_lifetimes(&[b]),
            Err(BundleLifetimeError::TimeTravel {
                idx: 0,
                created: 2000,
                now: 1000,
            })
        );
    }

    /// **PBLE-06** — too many rejected.
    #[test]
    fn pble_06_too_many_rejected() {
        let bs: Vec<BundleLifetime> = (0..=PBLE_MAX_BUNDLES)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                BundleLifetime { bundle_id: bid(b), created_at: 1000, now_ms: 2000 }
            })
            .collect();
        assert!(matches!(
            validate_bundle_lifetimes(&bs),
            Err(BundleLifetimeError::TooMany { .. })
        ));
    }

    /// **PBLE-07** — valid accepted.
    #[test]
    fn pble_07_valid_accepted() {
        assert_eq!(validate_bundle_lifetimes(&valid_bundles()), Ok(()));
    }

    /// **PBLE-08** — empty accepted.
    #[test]
    fn pble_08_empty_accepted() {
        assert_eq!(validate_bundle_lifetimes(&[]), Ok(()));
    }

    /// **PBLE-09** — single accepted.
    #[test]
    fn pble_09_single_accepted() {
        let bs = vec![bundle(0x01, 1000, 2000)];
        assert_eq!(validate_bundle_lifetimes(&bs), Ok(()));
    }

    /// **PBLE-10** — boundary lifetime accepted.
    #[test]
    fn pble_10_boundary_accepted() {
        let bs = vec![bundle(0x01, 1000, 1000 + PBLE_MAX_LIFETIME_MS)];
        assert_eq!(validate_bundle_lifetimes(&bs), Ok(()));
    }
}
