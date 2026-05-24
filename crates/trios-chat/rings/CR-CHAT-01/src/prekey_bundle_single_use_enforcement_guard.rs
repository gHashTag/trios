//! # CR-CHAT-01 — Prekey bundle single use enforcement guard (Wave-148 Lane B)
//!
//! IDENTITY — prekey bundles must be single-use; reuse enables
//! key-compromise impersonation.
//!
//! Each prekey bundle is intended for a single X3DH handshake.
//! If the same bundle is used multiple times:
//!
//! * **Key-compromise impersonation** — an attacker who compromises
//!   one session's shared secret can impersonate the bundle owner
//!   to other parties who reuse the same bundle.
//! * **Forward secrecy gap** — bundle reuse means multiple sessions
//!   share the same initial key material, so compromising one
//!   compromises all.
//! * **Attribution ambiguity** — multiple sessions from the same
//!   bundle cannot be distinguished, complicating audit trails.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No bundle ID may appear more than once.
//! 2. Bundle ID must not be zero.
//! 3. Usage count must be exactly 1.
//! 4. Session ID must not be zero.
//! 5. No duplicate session IDs.
//! 6. Batch size <= `PBSU_MAX_RECORDS`.
//!
//! Tests **PBSU-01..10**. Error enum [`SingleUseError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BUNDLE-SINGLE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum records per batch.
pub const PBSU_MAX_RECORDS: usize = 512;

/// Bundle ID length.
pub const PBSU_BUNDLE_ID_LEN: usize = 32;

/// Session ID length.
pub const PBSU_SESSION_ID_LEN: usize = 32;

/// A prekey bundle usage record.
#[derive(Debug, Clone)]
pub struct BundleUsageRecord {
    /// Bundle identifier.
    pub bundle_id: [u8; PBSU_BUNDLE_ID_LEN],
    /// Session that used this bundle.
    pub session_id: [u8; PBSU_SESSION_ID_LEN],
    /// Number of times this bundle has been used.
    pub usage_count: u64,
}

/// All ways single-use validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SingleUseError {
    /// Bundle reused.
    BundleReused {
        /// Index.
        idx: usize,
    },
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Usage count not 1.
    InvalidUsageCount {
        /// Index.
        idx: usize,
        /// Declared count.
        got: u64,
    },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId {
        /// Index.
        idx: usize,
    },
    /// Too many records.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate prekey bundle single use enforcement.
pub fn validate_single_use(
    records: &[BundleUsageRecord],
) -> Result<(), SingleUseError> {
    if records.len() > PBSU_MAX_RECORDS {
        return Err(SingleUseError::TooMany {
            got: records.len(),
            max: PBSU_MAX_RECORDS,
        });
    }
    let mut seen_bundles: BTreeSet<[u8; PBSU_BUNDLE_ID_LEN]> = BTreeSet::new();
    let mut seen_sessions: BTreeSet<[u8; PBSU_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.bundle_id == [0u8; PBSU_BUNDLE_ID_LEN] {
            return Err(SingleUseError::ZeroBundleId(i));
        }
        if r.session_id == [0u8; PBSU_SESSION_ID_LEN] {
            return Err(SingleUseError::ZeroSessionId(i));
        }
        if r.usage_count != 1 {
            return Err(SingleUseError::InvalidUsageCount {
                idx: i,
                got: r.usage_count,
            });
        }
        if !seen_bundles.insert(r.bundle_id) {
            return Err(SingleUseError::BundleReused { idx: i });
        }
        if !seen_sessions.insert(r.session_id) {
            return Err(SingleUseError::DuplicateSessionId { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBSU_BUNDLE_ID_LEN] {
        [byte; PBSU_BUNDLE_ID_LEN]
    }

    fn sid(byte: u8) -> [u8; PBSU_SESSION_ID_LEN] {
        [byte; PBSU_SESSION_ID_LEN]
    }

    fn rec(bundle: u8, session: u8, count: u64) -> BundleUsageRecord {
        BundleUsageRecord { bundle_id: bid(bundle), session_id: sid(session), usage_count: count }
    }

    fn valid_records() -> Vec<BundleUsageRecord> {
        vec![
            rec(0x01, 0xA1, 1),
            rec(0x02, 0xA2, 1),
            rec(0x03, 0xA3, 1),
        ]
    }

    /// **PBSU-01** — bundle reused rejected.
    #[test]
    fn pbsu_01_bundle_reused_rejected() {
        let rs = vec![
            rec(0x01, 0xA1, 1),
            rec(0x01, 0xA2, 1),
        ];
        assert_eq!(
            validate_single_use(&rs),
            Err(SingleUseError::BundleReused { idx: 1 })
        );
    }

    /// **PBSU-02** — zero bundle ID rejected.
    #[test]
    fn pbsu_02_zero_bundle_rejected() {
        let r = BundleUsageRecord { bundle_id: [0u8; PBSU_BUNDLE_ID_LEN], session_id: sid(0xA1), usage_count: 1 };
        assert_eq!(
            validate_single_use(&[r]),
            Err(SingleUseError::ZeroBundleId(0))
        );
    }

    /// **PBSU-03** — invalid usage count rejected.
    #[test]
    fn pbsu_03_invalid_count_rejected() {
        let r = rec(0x01, 0xA1, 2);
        assert_eq!(
            validate_single_use(&[r]),
            Err(SingleUseError::InvalidUsageCount { idx: 0, got: 2 })
        );
    }

    /// **PBSU-04** — zero session ID rejected.
    #[test]
    fn pbsu_04_zero_session_rejected() {
        let r = BundleUsageRecord { bundle_id: bid(0x01), session_id: [0u8; PBSU_SESSION_ID_LEN], usage_count: 1 };
        assert_eq!(
            validate_single_use(&[r]),
            Err(SingleUseError::ZeroSessionId(0))
        );
    }

    /// **PBSU-05** — duplicate session ID rejected.
    #[test]
    fn pbsu_05_duplicate_session_rejected() {
        let rs = vec![
            rec(0x01, 0xA1, 1),
            rec(0x02, 0xA1, 1),
        ];
        assert_eq!(
            validate_single_use(&rs),
            Err(SingleUseError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **PBSU-06** — too many rejected.
    #[test]
    fn pbsu_06_too_many_rejected() {
        let rs: Vec<BundleUsageRecord> = (0..=PBSU_MAX_RECORDS)
            .map(|i| {
                let mut b = [0u8; PBSU_BUNDLE_ID_LEN];
                let mut s = [0u8; PBSU_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                b[0..8].copy_from_slice(&val.to_be_bytes());
                s[0..8].copy_from_slice(&(val + 10000).to_be_bytes());
                BundleUsageRecord { bundle_id: b, session_id: s, usage_count: 1 }
            })
            .collect();
        assert_eq!(
            validate_single_use(&rs),
            Err(SingleUseError::TooMany {
                got: PBSU_MAX_RECORDS + 1,
                max: PBSU_MAX_RECORDS,
            })
        );
    }

    /// **PBSU-07** — valid accepted.
    #[test]
    fn pbsu_07_valid_accepted() {
        assert_eq!(validate_single_use(&valid_records()), Ok(()));
    }

    /// **PBSU-08** — empty accepted.
    #[test]
    fn pbsu_08_empty_accepted() {
        assert_eq!(validate_single_use(&[]), Ok(()));
    }

    /// **PBSU-09** — zero usage count rejected.
    #[test]
    fn pbsu_09_zero_count_rejected() {
        let r = rec(0x01, 0xA1, 0);
        assert_eq!(
            validate_single_use(&[r]),
            Err(SingleUseError::InvalidUsageCount { idx: 0, got: 0 })
        );
    }

    /// **PBSU-10** — many valid single-use accepted.
    #[test]
    fn pbsu_10_many_valid_accepted() {
        let rs: Vec<BundleUsageRecord> = (0..20u8)
            .map(|i| rec(i + 1, 0xA0 + i, 1))
            .collect();
        assert_eq!(validate_single_use(&rs), Ok(()));
    }
}
