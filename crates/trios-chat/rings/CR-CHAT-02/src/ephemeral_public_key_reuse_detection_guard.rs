//! # CR-CHAT-02 — Ephemeral public key reuse detection guard (Wave-116 Lane A)
//!
//! RATCHET — ephemeral DH public keys must never be reused.
//!
//! Each DH ratchet step generates a fresh ephemeral key pair. If the
//! same ephemeral public key appears in multiple sessions:
//!
//! * **Key-compromise impersonation** — if the ephemeral secret is
//!   compromised in one session, the attacker can impersonate the
//!   sender in all sessions that reused that key.
//! * **Session linkage** — the same ephemeral key links sessions,
//!   defeating forward secrecy across session boundaries.
//! * **Biometric fingerprint** — unique ephemeral keys per session
//!   are a privacy requirement; reuse creates a persistent identity.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate ephemeral public keys.
//! 2. Key must not be zero.
//! 3. Session ID must not be zero.
//! 4. Key length must be `EPKR_KEY_LEN`.
//! 5. No duplicate session IDs.
//! 6. Total records <= `EPKR_MAX_RECORDS`.
//!
//! Tests **EPKR-01..10**. Error enum [`EphemeralReuseError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPHEMERAL-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Key length.
pub const EPKR_KEY_LEN: usize = 32;

/// Session ID length.
pub const EPKR_SESSION_LEN: usize = 32;

/// Maximum records per batch.
pub const EPKR_MAX_RECORDS: usize = 1024;

/// An ephemeral key usage record.
#[derive(Debug, Clone)]
pub struct EphemeralKeyRecord {
    /// Session identifier.
    pub session_id: [u8; EPKR_SESSION_LEN],
    /// Ephemeral public key.
    pub ephemeral_key: [u8; EPKR_KEY_LEN],
}

/// All ways ephemeral key reuse validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EphemeralReuseError {
    /// Duplicate ephemeral key.
    DuplicateKey(usize),
    /// Zero key.
    ZeroKey(usize),
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate session ID.
    DuplicateSession(usize),
    /// Key matches another session's key.
    CrossSessionReuse { idx: usize, owner_session: [u8; EPKR_SESSION_LEN] },
    /// Too many records.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate ephemeral public key reuse detection.
pub fn validate_ephemeral_key_reuse(
    records: &[EphemeralKeyRecord],
) -> Result<(), EphemeralReuseError> {
    if records.len() > EPKR_MAX_RECORDS {
        return Err(EphemeralReuseError::TooMany {
            got: records.len(),
            max: EPKR_MAX_RECORDS,
        });
    }
    let mut keys: BTreeSet<[u8; EPKR_KEY_LEN]> = BTreeSet::new();
    let mut key_to_session: std::collections::BTreeMap<[u8; EPKR_KEY_LEN], [u8; EPKR_SESSION_LEN]> = std::collections::BTreeMap::new();
    let mut sessions: BTreeSet<[u8; EPKR_SESSION_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; EPKR_SESSION_LEN] {
            return Err(EphemeralReuseError::ZeroSession(i));
        }
        if r.ephemeral_key == [0u8; EPKR_KEY_LEN] {
            return Err(EphemeralReuseError::ZeroKey(i));
        }
        if !sessions.insert(r.session_id) {
            return Err(EphemeralReuseError::DuplicateSession(i));
        }
        if let Some(owner) = key_to_session.get(&r.ephemeral_key) {
            return Err(EphemeralReuseError::CrossSessionReuse {
                idx: i,
                owner_session: *owner,
            });
        }
        if !keys.insert(r.ephemeral_key) {
            return Err(EphemeralReuseError::DuplicateKey(i));
        }
        key_to_session.insert(r.ephemeral_key, r.session_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; EPKR_SESSION_LEN] {
        [byte; EPKR_SESSION_LEN]
    }

    fn key(byte: u8) -> [u8; EPKR_KEY_LEN] {
        [byte; EPKR_KEY_LEN]
    }

    fn record(session: u8, epk: u8) -> EphemeralKeyRecord {
        EphemeralKeyRecord { session_id: sid(session), ephemeral_key: key(epk) }
    }

    fn valid_records() -> Vec<EphemeralKeyRecord> {
        vec![
            record(0x01, 0xA1),
            record(0x02, 0xA2),
            record(0x03, 0xA3),
        ]
    }

    /// **EPKR-01** — cross-session reuse rejected.
    #[test]
    fn epkr_01_cross_session_rejected() {
        let rs = vec![record(0x01, 0xAA), record(0x02, 0xAA)];
        assert_eq!(
            validate_ephemeral_key_reuse(&rs),
            Err(EphemeralReuseError::CrossSessionReuse {
                idx: 1,
                owner_session: sid(0x01),
            })
        );
    }

    /// **EPKR-02** — zero key rejected.
    #[test]
    fn epkr_02_zero_key_rejected() {
        let r = EphemeralKeyRecord { session_id: sid(0x01), ephemeral_key: [0u8; EPKR_KEY_LEN] };
        assert_eq!(
            validate_ephemeral_key_reuse(&[r]),
            Err(EphemeralReuseError::ZeroKey(0))
        );
    }

    /// **EPKR-03** — zero session rejected.
    #[test]
    fn epkr_03_zero_session_rejected() {
        let r = EphemeralKeyRecord { session_id: [0u8; EPKR_SESSION_LEN], ephemeral_key: key(0xAA) };
        assert_eq!(
            validate_ephemeral_key_reuse(&[r]),
            Err(EphemeralReuseError::ZeroSession(0))
        );
    }

    /// **EPKR-04** — duplicate session rejected.
    #[test]
    fn epkr_04_duplicate_session_rejected() {
        let rs = vec![record(0x01, 0xA1), record(0x01, 0xA2)];
        assert_eq!(
            validate_ephemeral_key_reuse(&rs),
            Err(EphemeralReuseError::DuplicateSession(1))
        );
    }

    /// **EPKR-05** — too many rejected.
    #[test]
    fn epkr_05_too_many_rejected() {
        let rs: Vec<EphemeralKeyRecord> = (0..=EPKR_MAX_RECORDS)
            .map(|i| {
                let mut session = [0u8; EPKR_SESSION_LEN];
                let mut epk = [0u8; EPKR_KEY_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                epk[0..8].copy_from_slice(&val.to_be_bytes());
                EphemeralKeyRecord { session_id: session, ephemeral_key: epk }
            })
            .collect();
        assert_eq!(
            validate_ephemeral_key_reuse(&rs),
            Err(EphemeralReuseError::TooMany {
                got: EPKR_MAX_RECORDS + 1,
                max: EPKR_MAX_RECORDS,
            })
        );
    }

    /// **EPKR-06** — valid accepted.
    #[test]
    fn epkr_06_valid_accepted() {
        assert_eq!(validate_ephemeral_key_reuse(&valid_records()), Ok(()));
    }

    /// **EPKR-07** — empty accepted.
    #[test]
    fn epkr_07_empty_accepted() {
        assert_eq!(validate_ephemeral_key_reuse(&[]), Ok(()));
    }

    /// **EPKR-08** — single accepted.
    #[test]
    fn epkr_08_single_accepted() {
        let rs = vec![record(0x01, 0xAA)];
        assert_eq!(validate_ephemeral_key_reuse(&rs), Ok(()));
    }

    /// **EPKR-09** — many sessions unique keys accepted.
    #[test]
    fn epkr_09_many_unique_accepted() {
        let rs: Vec<EphemeralKeyRecord> = (0..100)
            .map(|i| {
                let mut session = [0u8; EPKR_SESSION_LEN];
                let mut epk = [0u8; EPKR_KEY_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                epk[0..8].copy_from_slice(&(val + 1000).to_be_bytes());
                EphemeralKeyRecord { session_id: session, ephemeral_key: epk }
            })
            .collect();
        assert_eq!(validate_ephemeral_key_reuse(&rs), Ok(()));
    }

    /// **EPKR-10** — max boundary accepted.
    #[test]
    fn epkr_10_max_boundary_accepted() {
        let rs: Vec<EphemeralKeyRecord> = (0..EPKR_MAX_RECORDS)
            .map(|i| {
                let mut session = [0u8; EPKR_SESSION_LEN];
                let mut epk = [0u8; EPKR_KEY_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                epk[0..8].copy_from_slice(&(val + 5000).to_be_bytes());
                EphemeralKeyRecord { session_id: session, ephemeral_key: epk }
            })
            .collect();
        assert_eq!(validate_ephemeral_key_reuse(&rs), Ok(()));
    }
}
