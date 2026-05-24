//! # CR-CHAT-05 — Store session isolation verification guard (Wave-111 Lane A)
//!
//! PERSISTENCE — different sessions must be fully isolated.
//!
//! The persistence layer stores envelopes indexed by `(session,
//! counter)`. If session isolation is violated:
//!
//! * **Cross-session read** — one session can read another session's
//!   ciphertexts, breaking session-level confidentiality.
//! * **Counter collision** — two sessions writing to the same counter
//!   slot cause data loss or corruption.
//! * **Metadata leakage** — listing one session reveals the existence
//!   and counter range of other sessions.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No record may appear in multiple sessions.
//! 2. Session ID must not be zero.
//! 3. Ciphertext hash must be unique across sessions.
//! 4. No duplicate (session, counter) pairs.
//! 5. Counter must be > 0.
//! 6. Total records <= `SSIV_MAX_RECORDS`.
//!
//! Tests **SSIV-01..10**. Error enum [`IsolationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SESSION-ISOLATION`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum records per batch.
pub const SSIV_MAX_RECORDS: usize = 10_000;

/// Session ID length.
pub const SSIV_SESSION_LEN: usize = 32;

/// Hash length.
pub const SSIV_HASH_LEN: usize = 32;

/// A stored record for isolation verification.
#[derive(Debug, Clone)]
pub struct IsolationRecord {
    /// Session identifier.
    pub session_id: [u8; SSIV_SESSION_LEN],
    /// Counter value.
    pub counter: u64,
    /// Hash of the ciphertext (for cross-session duplicate detection).
    pub ct_hash: [u8; SSIV_HASH_LEN],
}

/// All ways session isolation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IsolationError {
    /// Cross-session ciphertext duplicate.
    CrossSessionDuplicate {
        /// Index of the offending record.
        idx: usize,
        /// Session that owns the original ciphertext.
        owner_session: [u8; SSIV_SESSION_LEN],
    },
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate (session, counter).
    DuplicateKey(usize),
    /// Zero counter.
    ZeroCounter(usize),
    /// Too many records.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store session isolation.
pub fn validate_session_isolation(
    records: &[IsolationRecord],
) -> Result<(), IsolationError> {
    if records.len() > SSIV_MAX_RECORDS {
        return Err(IsolationError::TooMany {
            got: records.len(),
            max: SSIV_MAX_RECORDS,
        });
    }
    let mut ct_to_session: BTreeMap<[u8; SSIV_HASH_LEN], [u8; SSIV_SESSION_LEN]> = BTreeMap::new();
    let mut keys: BTreeSet<([u8; SSIV_SESSION_LEN], u64)> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; SSIV_SESSION_LEN] {
            return Err(IsolationError::ZeroSession(i));
        }
        if r.counter == 0 {
            return Err(IsolationError::ZeroCounter(i));
        }
        if !keys.insert((r.session_id, r.counter)) {
            return Err(IsolationError::DuplicateKey(i));
        }
        if let Some(owner) = ct_to_session.get(&r.ct_hash) {
            if *owner != r.session_id {
                return Err(IsolationError::CrossSessionDuplicate {
                    idx: i,
                    owner_session: *owner,
                });
            }
        } else {
            ct_to_session.insert(r.ct_hash, r.session_id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; SSIV_SESSION_LEN] {
        [byte; SSIV_SESSION_LEN]
    }

    fn hash(byte: u8) -> [u8; SSIV_HASH_LEN] {
        [byte; SSIV_HASH_LEN]
    }

    fn record(session: u8, counter: u64, ct: u8) -> IsolationRecord {
        IsolationRecord { session_id: sid(session), counter, ct_hash: hash(ct) }
    }

    fn valid_records() -> Vec<IsolationRecord> {
        vec![
            record(0x01, 1, 0xA1),
            record(0x01, 2, 0xA2),
            record(0x02, 1, 0xB1),
            record(0x02, 2, 0xB2),
        ]
    }

    /// **SSIV-01** — cross-session duplicate rejected.
    #[test]
    fn ssiv_01_cross_session_dup_rejected() {
        let rs = vec![
            record(0x01, 1, 0xAA),
            record(0x02, 1, 0xAA),
        ];
        assert_eq!(
            validate_session_isolation(&rs),
            Err(IsolationError::CrossSessionDuplicate {
                idx: 1,
                owner_session: sid(0x01),
            })
        );
    }

    /// **SSIV-02** — zero session rejected.
    #[test]
    fn ssiv_02_zero_session_rejected() {
        let r = IsolationRecord { session_id: [0u8; SSIV_SESSION_LEN], counter: 1, ct_hash: hash(0x01) };
        assert_eq!(
            validate_session_isolation(&[r]),
            Err(IsolationError::ZeroSession(0))
        );
    }

    /// **SSIV-03** — duplicate key rejected.
    #[test]
    fn ssiv_03_duplicate_key_rejected() {
        let rs = vec![record(0x01, 5, 0xA1), record(0x01, 5, 0xA2)];
        assert_eq!(
            validate_session_isolation(&rs),
            Err(IsolationError::DuplicateKey(1))
        );
    }

    /// **SSIV-04** — zero counter rejected.
    #[test]
    fn ssiv_04_zero_counter_rejected() {
        let r = record(0x01, 0, 0x01);
        assert_eq!(
            validate_session_isolation(&[r]),
            Err(IsolationError::ZeroCounter(0))
        );
    }

    /// **SSIV-05** — too many rejected.
    #[test]
    fn ssiv_05_too_many_rejected() {
        let rs: Vec<IsolationRecord> = (0..=SSIV_MAX_RECORDS)
            .map(|i| {
                let s = (i as u8).wrapping_add(1);
                IsolationRecord { session_id: sid(s), counter: 1, ct_hash: hash(s) }
            })
            .collect();
        assert!(matches!(
            validate_session_isolation(&rs),
            Err(IsolationError::TooMany { .. })
        ));
    }

    /// **SSIV-06** — same hash same session accepted.
    #[test]
    fn ssiv_06_same_hash_same_session_accepted() {
        let rs = vec![record(0x01, 1, 0xAA), record(0x01, 2, 0xAA)];
        assert_eq!(validate_session_isolation(&rs), Ok(()));
    }

    /// **SSIV-07** — valid accepted.
    #[test]
    fn ssiv_07_valid_accepted() {
        assert_eq!(validate_session_isolation(&valid_records()), Ok(()));
    }

    /// **SSIV-08** — empty accepted.
    #[test]
    fn ssiv_08_empty_accepted() {
        assert_eq!(validate_session_isolation(&[]), Ok(()));
    }

    /// **SSIV-09** — single accepted.
    #[test]
    fn ssiv_09_single_accepted() {
        let rs = vec![record(0x01, 1, 0x01)];
        assert_eq!(validate_session_isolation(&rs), Ok(()));
    }

    /// **SSIV-10** — multiple sessions isolated accepted.
    #[test]
    fn ssiv_10_multi_session_accepted() {
        let rs: Vec<IsolationRecord> = (0..4)
            .flat_map(|s| {
                let session = (s as u8) + 1;
                (1..=3).map(move |c| {
                    IsolationRecord {
                        session_id: sid(session),
                        counter: c,
                        ct_hash: hash(session * 16 + c as u8),
                    }
                })
            })
            .collect();
        assert_eq!(validate_session_isolation(&rs), Ok(()));
    }
}
