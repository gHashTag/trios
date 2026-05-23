//! # CR-CHAT-01 — Prekey bundle freshness guard (Wave-54 Lane A)
//!
//! ИДЕНТИФИКАЦИЯ — свежесть prekey bundle, R-CHAT-2.
//!
//! Prekey bundle имеет TTL. Если атакующий подставит старый bundle
//! (после ротации ключей), получатель зашифрует на скомпрометированный
//! ключ. Защита:
//!
//! 1. Bundle timestamp ≤ `PBFG_MAX_AGE_MS` от текущего времени.
//! 2. Bundle timestamp не из будущего (> now).
//! 3. Bundle version = `PBFG_VERSION`.
//! 4. Signature не пустой.
//! 5. Bundle ID уникален (нет replay).
//! 6. Max bundles в окне ≤ `PBFG_MAX_BUNDLES`.
//!
//! Tests **PBFG-01..10**. Error enum [`BundleFreshnessError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · BUNDLE-FRESHNESS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bundle age in milliseconds (24 hours).
pub const PBFG_MAX_AGE_MS: u64 = 86_400_000;

/// Expected bundle version.
pub const PBFG_VERSION: u32 = 1;

/// Maximum bundles in a tracking window.
pub const PBFG_MAX_BUNDLES: usize = 256;

/// All ways bundle freshness can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleFreshnessError {
    /// Bundle too old.
    Expired,
    /// Timestamp in the future.
    FutureTimestamp,
    /// Wrong version.
    WrongVersion,
    /// Empty signature.
    EmptySignature,
    /// Duplicate bundle ID.
    DuplicateBundle,
    /// Too many bundles.
    TooManyBundles,
}

/// A prekey bundle with metadata.
#[derive(Debug, Clone)]
pub struct BundleCheck {
    /// Bundle ID (hash of public key material).
    pub bundle_id: [u8; 32],
    /// Timestamp when bundle was created (ms since epoch).
    pub timestamp_ms: u64,
    /// Bundle version.
    pub version: u32,
    /// Signature length (bytes).
    pub sig_len: usize,
}

/// `[VERIFIED]` Validate a single bundle against current time.
pub fn validate_bundle_freshness(
    bundle: &BundleCheck,
    now_ms: u64,
) -> Result<(), BundleFreshnessError> {
    if bundle.sig_len == 0 {
        return Err(BundleFreshnessError::EmptySignature);
    }
    if bundle.version != PBFG_VERSION {
        return Err(BundleFreshnessError::WrongVersion);
    }
    if bundle.timestamp_ms > now_ms {
        return Err(BundleFreshnessError::FutureTimestamp);
    }
    if now_ms - bundle.timestamp_ms > PBFG_MAX_AGE_MS {
        return Err(BundleFreshnessError::Expired);
    }
    Ok(())
}

/// `[VERIFIED]` Validate a batch of bundles for freshness + uniqueness.
pub fn validate_bundle_batch(
    bundles: &[BundleCheck],
    now_ms: u64,
) -> Result<(), BundleFreshnessError> {
    if bundles.len() > PBFG_MAX_BUNDLES {
        return Err(BundleFreshnessError::TooManyBundles);
    }
    let mut seen = BTreeSet::new();
    for b in bundles {
        validate_bundle_freshness(b, now_ms)?;
        if !seen.insert(b.bundle_id) {
            return Err(BundleFreshnessError::DuplicateBundle);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(ts: u64) -> BundleCheck {
        BundleCheck {
            bundle_id: [ts as u8; 32],
            timestamp_ms: ts,
            version: PBFG_VERSION,
            sig_len: 64,
        }
    }

    const NOW: u64 = 1_000_000_000;

    /// **PBFG-01** — expired rejected.
    #[test]
    fn pbfg_01_expired_rejected() {
        let b = bundle(NOW - PBFG_MAX_AGE_MS - 1);
        assert_eq!(
            validate_bundle_freshness(&b, NOW),
            Err(BundleFreshnessError::Expired)
        );
    }

    /// **PBFG-02** — future rejected.
    #[test]
    fn pbfg_02_future_rejected() {
        let b = bundle(NOW + 1000);
        assert_eq!(
            validate_bundle_freshness(&b, NOW),
            Err(BundleFreshnessError::FutureTimestamp)
        );
    }

    /// **PBFG-03** — wrong version rejected.
    #[test]
    fn pbfg_03_wrong_version_rejected() {
        let mut b = bundle(NOW - 1000);
        b.version = 99;
        assert_eq!(
            validate_bundle_freshness(&b, NOW),
            Err(BundleFreshnessError::WrongVersion)
        );
    }

    /// **PBFG-04** — empty sig rejected.
    #[test]
    fn pbfg_04_empty_sig_rejected() {
        let mut b = bundle(NOW - 1000);
        b.sig_len = 0;
        assert_eq!(
            validate_bundle_freshness(&b, NOW),
            Err(BundleFreshnessError::EmptySignature)
        );
    }

    /// **PBFG-05** — duplicate rejected.
    #[test]
    fn pbfg_05_duplicate_rejected() {
        let b = bundle(NOW - 1000);
        assert_eq!(
            validate_bundle_batch(&[b.clone(), b], NOW),
            Err(BundleFreshnessError::DuplicateBundle)
        );
    }

    /// **PBFG-06** — fresh accepted.
    #[test]
    fn pbfg_06_fresh_accepted() {
        assert_eq!(validate_bundle_freshness(&bundle(NOW - 1000), NOW), Ok(()));
    }

    /// **PBFG-07** — exact max age accepted.
    #[test]
    fn pbfg_07_exact_age_accepted() {
        assert_eq!(
            validate_bundle_freshness(&bundle(NOW - PBFG_MAX_AGE_MS), NOW),
            Ok(())
        );
    }

    /// **PBFG-08** — batch accepted.
    #[test]
    fn pbfg_08_batch_accepted() {
        let bs: Vec<BundleCheck> = (0..10).map(|i| bundle(NOW - i * 100)).collect();
        assert_eq!(validate_bundle_batch(&bs, NOW), Ok(()));
    }

    /// **PBFG-09** — empty batch accepted.
    #[test]
    fn pbfg_09_empty_accepted() {
        assert_eq!(validate_bundle_batch(&[], NOW), Ok(()));
    }

    /// **PBFG-10** — too many bundles rejected.
    #[test]
    fn pbfg_10_too_many_rejected() {
        let bs: Vec<BundleCheck> = (0..=PBFG_MAX_BUNDLES).map(|i| {
            let mut b = bundle(NOW - i as u64);
            b.bundle_id = [(i % 256) as u8; 32];
            b
        }).collect();
        assert_eq!(
            validate_bundle_batch(&bs, NOW),
            Err(BundleFreshnessError::TooManyBundles)
        );
    }
}
