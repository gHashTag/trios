//! # CR-CHAT-01 — Prekey bundle timestamp freshness guard (Wave-138 Lane B)
//!
//! IDENTITY — prekey bundles must not be older than a maximum age;
//! stale bundles enable replay attacks.
//!
//! Each prekey bundle carries a creation timestamp. If a bundle is
//! too old:
//!
//! * **Replay attack** — an old bundle may have been compromised
//!   since its creation; using it allows the attacker to decrypt
//!   messages.
//! * **Key rotation gap** — the user may have rotated their identity
//!   since the bundle was created; the old bundle points to a
//!   deprecated key.
//! * **Clock skew abuse** — without freshness validation, an
//!   attacker can inject bundles with far-future timestamps to
//!   extend their validity window.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Bundle age <= `PBTF_MAX_AGE_MS`.
//! 2. Bundle ID must not be zero.
//! 3. No duplicate bundle IDs.
//! 4. Timestamp must not be zero.
//! 5. Timestamp must not be in the future (relative to `now_ms`).
//! 6. Batch size <= `PBTF_MAX_BATCH`.
//!
//! Tests **PBTF-01..10**. Error enum [`FreshnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FRESH-BUNDLE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bundle age in milliseconds (30 days).
pub const PBTF_MAX_AGE_MS: u64 = 30 * 24 * 60 * 60 * 1000;

/// Maximum batch size.
pub const PBTF_MAX_BATCH: usize = 256;

/// Bundle ID length.
pub const PBTF_BUNDLE_ID_LEN: usize = 32;

/// A prekey bundle timestamp record.
#[derive(Debug, Clone)]
pub struct BundleFreshnessRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBTF_BUNDLE_ID_LEN],
    /// Creation timestamp (ms since epoch).
    pub created_ms: u64,
}

/// All ways freshness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FreshnessError {
    /// Bundle too old.
    TooOld { idx: usize, age_ms: u64, max_ms: u64 },
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundleId { idx: usize },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Future timestamp.
    FutureTimestamp { idx: usize, created_ms: u64, now_ms: u64 },
    /// Batch too large.
    TooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle timestamp freshness.
pub fn validate_bundle_freshness(
    bundles: &[BundleFreshnessRecord],
    now_ms: u64,
) -> Result<(), FreshnessError> {
    if bundles.len() > PBTF_MAX_BATCH {
        return Err(FreshnessError::TooLarge {
            got: bundles.len(),
            max: PBTF_MAX_BATCH,
        });
    }
    let mut seen: BTreeSet<[u8; PBTF_BUNDLE_ID_LEN]> = BTreeSet::new();
    for (i, b) in bundles.iter().enumerate() {
        if b.bundle_id == [0u8; PBTF_BUNDLE_ID_LEN] {
            return Err(FreshnessError::ZeroBundleId(i));
        }
        if !seen.insert(b.bundle_id) {
            return Err(FreshnessError::DuplicateBundleId { idx: i });
        }
        if b.created_ms == 0 {
            return Err(FreshnessError::ZeroTimestamp(i));
        }
        if b.created_ms > now_ms {
            return Err(FreshnessError::FutureTimestamp {
                idx: i,
                created_ms: b.created_ms,
                now_ms,
            });
        }
        let age = now_ms - b.created_ms;
        if age > PBTF_MAX_AGE_MS {
            return Err(FreshnessError::TooOld {
                idx: i,
                age_ms: age,
                max_ms: PBTF_MAX_AGE_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBTF_BUNDLE_ID_LEN] {
        [byte; PBTF_BUNDLE_ID_LEN]
    }

    fn bundle(id: u8, created: u64) -> BundleFreshnessRecord {
        BundleFreshnessRecord { bundle_id: bid(id), created_ms: created }
    }

    const NOW: u64 = 10_000_000_000;

    fn valid_records() -> Vec<BundleFreshnessRecord> {
        vec![
            bundle(0x01, NOW - 1000),
            bundle(0x02, NOW - 5000),
        ]
    }

    /// **PBTF-01** — too old rejected.
    #[test]
    fn pbtf_01_too_old_rejected() {
        let b = bundle(0x01, NOW - PBTF_MAX_AGE_MS - 1);
        assert_eq!(
            validate_bundle_freshness(&[b], NOW),
            Err(FreshnessError::TooOld {
                idx: 0,
                age_ms: PBTF_MAX_AGE_MS + 1,
                max_ms: PBTF_MAX_AGE_MS,
            })
        );
    }

    /// **PBTF-02** — zero bundle ID rejected.
    #[test]
    fn pbtf_02_zero_bundle_id_rejected() {
        let b = BundleFreshnessRecord { bundle_id: [0u8; PBTF_BUNDLE_ID_LEN], created_ms: NOW - 1000 };
        assert_eq!(
            validate_bundle_freshness(&[b], NOW),
            Err(FreshnessError::ZeroBundleId(0))
        );
    }

    /// **PBTF-03** — duplicate bundle ID rejected.
    #[test]
    fn pbtf_03_duplicate_rejected() {
        let bs = vec![
            bundle(0x01, NOW - 1000),
            bundle(0x01, NOW - 2000),
        ];
        assert_eq!(
            validate_bundle_freshness(&bs, NOW),
            Err(FreshnessError::DuplicateBundleId { idx: 1 })
        );
    }

    /// **PBTF-04** — zero timestamp rejected.
    #[test]
    fn pbtf_04_zero_timestamp_rejected() {
        let b = BundleFreshnessRecord { bundle_id: bid(0x01), created_ms: 0 };
        assert_eq!(
            validate_bundle_freshness(&[b], NOW),
            Err(FreshnessError::ZeroTimestamp(0))
        );
    }

    /// **PBTF-05** — future timestamp rejected.
    #[test]
    fn pbtf_05_future_timestamp_rejected() {
        let b = bundle(0x01, NOW + 1000);
        assert_eq!(
            validate_bundle_freshness(&[b], NOW),
            Err(FreshnessError::FutureTimestamp {
                idx: 0,
                created_ms: NOW + 1000,
                now_ms: NOW,
            })
        );
    }

    /// **PBTF-06** — batch too large rejected.
    #[test]
    fn pbtf_06_too_large_rejected() {
        let bs: Vec<BundleFreshnessRecord> = (0..=PBTF_MAX_BATCH)
            .map(|i| {
                let mut id = [0u8; PBTF_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                BundleFreshnessRecord { bundle_id: id, created_ms: NOW - 1000 }
            })
            .collect();
        assert_eq!(
            validate_bundle_freshness(&bs, NOW),
            Err(FreshnessError::TooLarge {
                got: PBTF_MAX_BATCH + 1,
                max: PBTF_MAX_BATCH,
            })
        );
    }

    /// **PBTF-07** — valid accepted.
    #[test]
    fn pbtf_07_valid_accepted() {
        assert_eq!(validate_bundle_freshness(&valid_records(), NOW), Ok(()));
    }

    /// **PBTF-08** — empty accepted.
    #[test]
    fn pbtf_08_empty_accepted() {
        assert_eq!(validate_bundle_freshness(&[], NOW), Ok(()));
    }

    /// **PBTF-09** — boundary age accepted.
    #[test]
    fn pbtf_09_boundary_age_accepted() {
        let b = bundle(0x01, NOW - PBTF_MAX_AGE_MS);
        assert_eq!(validate_bundle_freshness(&[b], NOW), Ok(()));
    }

    /// **PBTF-10** — exact now accepted.
    #[test]
    fn pbtf_10_exact_now_accepted() {
        let b = bundle(0x01, NOW);
        assert_eq!(validate_bundle_freshness(&[b], NOW), Ok(()));
    }
}
