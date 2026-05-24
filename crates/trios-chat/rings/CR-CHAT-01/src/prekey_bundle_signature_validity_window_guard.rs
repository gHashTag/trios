//! # CR-CHAT-01 — Prekey bundle signature validity window guard (Wave-132 Lane A)
//!
//! IDENTITY — prekey bundle signatures must be verified within a
//! validity window; expired or not-yet-valid signatures indicate
//! replay or clock manipulation.
//!
//! Each prekey bundle signature has a validity window defined by
//! signed-at and expires-at timestamps:
//!
//! * **Expired signatures** — an expired signature means the bundle
//!   may have been revoked or compromised; accepting it is unsafe.
//! * **Future signatures** — a signature dated in the future
//!   indicates clock skew or manipulation.
//! * **Replay via clock** — an attacker who can manipulate clocks
//!   can replay old signatures as current.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Signature must not be expired (`now <= expires_at`).
//! 2. Signed-at must be <= `now`.
//! 3. Bundle ID must not be zero.
//! 4. No duplicate bundle IDs.
//! 5. Validity window must be <= `PBSV_MAX_WINDOW_MS`.
//! 6. Total bundles <= `PBSV_MAX_BUNDLES`.
//!
//! Tests **PBSV-01..10**. Error enum [`ValidityWindowError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * VALIDITY-WINDOW`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum validity window in milliseconds.
pub const PBSV_MAX_WINDOW_MS: u64 = 7 * 24 * 3600 * 1000;

/// Maximum bundles per batch.
pub const PBSV_MAX_BUNDLES: usize = 1024;

/// Bundle ID length.
pub const PBSV_BUNDLE_ID_LEN: usize = 32;

/// A prekey bundle signature validity record.
#[derive(Debug, Clone)]
pub struct SignatureValidityRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBSV_BUNDLE_ID_LEN],
    /// Timestamp when the signature was created.
    pub signed_at_ms: u64,
    /// Timestamp when the signature expires.
    pub expires_at_ms: u64,
    /// Current time for validation.
    pub now_ms: u64,
}

/// All ways validity window validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ValidityWindowError {
    /// Signature expired.
    Expired { idx: usize, expires_at: u64, now: u64 },
    /// Signed in the future.
    FutureSignature { idx: usize, signed_at: u64, now: u64 },
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundleId { idx: usize },
    /// Validity window too large.
    WindowTooLarge { idx: usize, window_ms: u64, max: u64 },
    /// Too many bundles.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle signature validity window.
pub fn validate_signature_validity(
    records: &[SignatureValidityRecord],
) -> Result<(), ValidityWindowError> {
    if records.len() > PBSV_MAX_BUNDLES {
        return Err(ValidityWindowError::TooMany {
            got: records.len(),
            max: PBSV_MAX_BUNDLES,
        });
    }
    let mut seen: BTreeSet<[u8; PBSV_BUNDLE_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.bundle_id == [0u8; PBSV_BUNDLE_ID_LEN] {
            return Err(ValidityWindowError::ZeroBundleId(i));
        }
        if !seen.insert(r.bundle_id) {
            return Err(ValidityWindowError::DuplicateBundleId { idx: i });
        }
        if r.signed_at_ms > r.now_ms {
            return Err(ValidityWindowError::FutureSignature {
                idx: i,
                signed_at: r.signed_at_ms,
                now: r.now_ms,
            });
        }
        if r.expires_at_ms < r.now_ms {
            return Err(ValidityWindowError::Expired {
                idx: i,
                expires_at: r.expires_at_ms,
                now: r.now_ms,
            });
        }
        let window = r.expires_at_ms.saturating_sub(r.signed_at_ms);
        if window > PBSV_MAX_WINDOW_MS {
            return Err(ValidityWindowError::WindowTooLarge {
                idx: i,
                window_ms: window,
                max: PBSV_MAX_WINDOW_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBSV_BUNDLE_ID_LEN] {
        [byte; PBSV_BUNDLE_ID_LEN]
    }

    fn record(bundle: u8, signed: u64, expires: u64, now: u64) -> SignatureValidityRecord {
        SignatureValidityRecord { bundle_id: bid(bundle), signed_at_ms: signed, expires_at_ms: expires, now_ms: now }
    }

    fn valid_records() -> Vec<SignatureValidityRecord> {
        vec![
            record(0x01, 1_000_000, 1_000_000 + PBSV_MAX_WINDOW_MS, 1_000_001),
            record(0x02, 2_000_000, 2_000_000 + PBSV_MAX_WINDOW_MS, 2_000_001),
        ]
    }

    /// **PBSV-01** — expired rejected.
    #[test]
    fn pbsv_01_expired_rejected() {
        let r = record(0x01, 100, 200, 300);
        assert_eq!(
            validate_signature_validity(&[r]),
            Err(ValidityWindowError::Expired { idx: 0, expires_at: 200, now: 300 })
        );
    }

    /// **PBSV-02** — future signature rejected.
    #[test]
    fn pbsv_02_future_rejected() {
        let r = record(0x01, 300, 400, 100);
        assert_eq!(
            validate_signature_validity(&[r]),
            Err(ValidityWindowError::FutureSignature { idx: 0, signed_at: 300, now: 100 })
        );
    }

    /// **PBSV-03** — zero bundle ID rejected.
    #[test]
    fn pbsv_03_zero_bundle_rejected() {
        let r = SignatureValidityRecord { bundle_id: [0u8; PBSV_BUNDLE_ID_LEN], signed_at_ms: 100, expires_at_ms: 200, now_ms: 150 };
        assert_eq!(
            validate_signature_validity(&[r]),
            Err(ValidityWindowError::ZeroBundleId(0))
        );
    }

    /// **PBSV-04** — duplicate bundle ID rejected.
    #[test]
    fn pbsv_04_duplicate_rejected() {
        let rs = vec![
            record(0x01, 100, 200, 150),
            record(0x01, 200, 300, 250),
        ];
        assert_eq!(
            validate_signature_validity(&rs),
            Err(ValidityWindowError::DuplicateBundleId { idx: 1 })
        );
    }

    /// **PBSV-05** — window too large rejected.
    #[test]
    fn pbsv_05_window_too_large_rejected() {
        let r = record(0x01, 100, 100 + PBSV_MAX_WINDOW_MS + 1, 200);
        assert_eq!(
            validate_signature_validity(&[r]),
            Err(ValidityWindowError::WindowTooLarge {
                idx: 0,
                window_ms: PBSV_MAX_WINDOW_MS + 1,
                max: PBSV_MAX_WINDOW_MS,
            })
        );
    }

    /// **PBSV-06** — too many rejected.
    #[test]
    fn pbsv_06_too_many_rejected() {
        let rs: Vec<SignatureValidityRecord> = (0..=PBSV_MAX_BUNDLES)
            .map(|i| {
                let mut id = [0u8; PBSV_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                SignatureValidityRecord { bundle_id: id, signed_at_ms: 100, expires_at_ms: 200, now_ms: 150 }
            })
            .collect();
        assert_eq!(
            validate_signature_validity(&rs),
            Err(ValidityWindowError::TooMany {
                got: PBSV_MAX_BUNDLES + 1,
                max: PBSV_MAX_BUNDLES,
            })
        );
    }

    /// **PBSV-07** — valid accepted.
    #[test]
    fn pbsv_07_valid_accepted() {
        assert_eq!(validate_signature_validity(&valid_records()), Ok(()));
    }

    /// **PBSV-08** — empty accepted.
    #[test]
    fn pbsv_08_empty_accepted() {
        assert_eq!(validate_signature_validity(&[]), Ok(()));
    }

    /// **PBSV-09** — exact boundary accepted.
    #[test]
    fn pbsv_09_boundary_accepted() {
        let r = record(0x01, 100, 100 + PBSV_MAX_WINDOW_MS, 200);
        assert_eq!(validate_signature_validity(&[r]), Ok(()));
    }

    /// **PBSV-10** — exact expiry moment accepted.
    #[test]
    fn pbsv_10_exact_expiry_accepted() {
        let r = record(0x01, 100, 200, 200);
        assert_eq!(validate_signature_validity(&[r]), Ok(()));
    }
}
