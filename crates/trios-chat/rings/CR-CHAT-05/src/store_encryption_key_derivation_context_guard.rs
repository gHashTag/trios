//! # CR-CHAT-05 — Store encryption key derivation context guard (Wave-114 Lane B)
//!
//! PERSISTENCE — store encryption keys must use unique derivation contexts.
//!
//! Each session's store encryption key is derived via HKDF with a
//! context-specific info parameter. If contexts are reused:
//!
//! * **Key collision** — two sessions derive the same encryption key,
//!   so compromising one session's key compromises the other.
//! * **Cross-session decryption** — if contexts match, a key from
//!   session A can decrypt session B's data.
//! * **Audit failure** — it becomes impossible to prove which session
//!   produced a given ciphertext.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate derivation contexts.
//! 2. Context must not be zero.
//! 3. Session ID must not be zero.
//! 4. No duplicate session IDs.
//! 5. Key hash must not be zero.
//! 6. Total records <= `SEKC_MAX_RECORDS`.
//!
//! Tests **SEKC-01..10**. Error enum [`KeyContextError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KEY-CONTEXT-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum records per batch.
pub const SEKC_MAX_RECORDS: usize = 1024;

/// Context length.
pub const SEKC_CONTEXT_LEN: usize = 32;

/// Session ID length.
pub const SEKC_SESSION_LEN: usize = 32;

/// Key hash length.
pub const SEKC_KEY_HASH_LEN: usize = 32;

/// A key derivation context record.
#[derive(Debug, Clone)]
pub struct KeyContextRecord {
    /// Session identifier.
    pub session_id: [u8; SEKC_SESSION_LEN],
    /// Derivation context.
    pub context: [u8; SEKC_CONTEXT_LEN],
    /// Hash of the derived key.
    pub key_hash: [u8; SEKC_KEY_HASH_LEN],
}

/// All ways key context validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyContextError {
    /// Duplicate derivation context.
    DuplicateContext(usize),
    /// Zero context.
    ZeroContext(usize),
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate session ID.
    DuplicateSession(usize),
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Too many records.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store encryption key derivation context.
pub fn validate_key_contexts(
    records: &[KeyContextRecord],
) -> Result<(), KeyContextError> {
    if records.len() > SEKC_MAX_RECORDS {
        return Err(KeyContextError::TooMany {
            got: records.len(),
            max: SEKC_MAX_RECORDS,
        });
    }
    let mut contexts: BTreeSet<[u8; SEKC_CONTEXT_LEN]> = BTreeSet::new();
    let mut sessions: BTreeSet<[u8; SEKC_SESSION_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; SEKC_SESSION_LEN] {
            return Err(KeyContextError::ZeroSession(i));
        }
        if r.context == [0u8; SEKC_CONTEXT_LEN] {
            return Err(KeyContextError::ZeroContext(i));
        }
        if r.key_hash == [0u8; SEKC_KEY_HASH_LEN] {
            return Err(KeyContextError::ZeroKeyHash(i));
        }
        if !sessions.insert(r.session_id) {
            return Err(KeyContextError::DuplicateSession(i));
        }
        if !contexts.insert(r.context) {
            return Err(KeyContextError::DuplicateContext(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arr32(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn record(session: u8, context: u8, key: u8) -> KeyContextRecord {
        KeyContextRecord {
            session_id: arr32(session),
            context: arr32(context),
            key_hash: arr32(key),
        }
    }

    fn valid_records() -> Vec<KeyContextRecord> {
        vec![
            record(0x01, 0xA1, 0x11),
            record(0x02, 0xA2, 0x22),
            record(0x03, 0xA3, 0x33),
        ]
    }

    /// **SEKC-01** — duplicate context rejected.
    #[test]
    fn sekc_01_duplicate_context_rejected() {
        let rs = vec![record(0x01, 0xAA, 0x11), record(0x02, 0xAA, 0x22)];
        assert_eq!(
            validate_key_contexts(&rs),
            Err(KeyContextError::DuplicateContext(1))
        );
    }

    /// **SEKC-02** — zero context rejected.
    #[test]
    fn sekc_02_zero_context_rejected() {
        let r = KeyContextRecord { session_id: arr32(0x01), context: [0u8; SEKC_CONTEXT_LEN], key_hash: arr32(0x11) };
        assert_eq!(
            validate_key_contexts(&[r]),
            Err(KeyContextError::ZeroContext(0))
        );
    }

    /// **SEKC-03** — zero session rejected.
    #[test]
    fn sekc_03_zero_session_rejected() {
        let r = KeyContextRecord { session_id: [0u8; SEKC_SESSION_LEN], context: arr32(0xAA), key_hash: arr32(0x11) };
        assert_eq!(
            validate_key_contexts(&[r]),
            Err(KeyContextError::ZeroSession(0))
        );
    }

    /// **SEKC-04** — duplicate session rejected.
    #[test]
    fn sekc_04_duplicate_session_rejected() {
        let rs = vec![record(0x01, 0xA1, 0x11), record(0x01, 0xA2, 0x22)];
        assert_eq!(
            validate_key_contexts(&rs),
            Err(KeyContextError::DuplicateSession(1))
        );
    }

    /// **SEKC-05** — zero key hash rejected.
    #[test]
    fn sekc_05_zero_key_rejected() {
        let r = KeyContextRecord { session_id: arr32(0x01), context: arr32(0xAA), key_hash: [0u8; SEKC_KEY_HASH_LEN] };
        assert_eq!(
            validate_key_contexts(&[r]),
            Err(KeyContextError::ZeroKeyHash(0))
        );
    }

    /// **SEKC-06** — too many rejected.
    #[test]
    fn sekc_06_too_many_rejected() {
        let rs: Vec<KeyContextRecord> = (0..=SEKC_MAX_RECORDS)
            .map(|i| {
                let mut session = [0u8; SEKC_SESSION_LEN];
                let mut context = [0u8; SEKC_CONTEXT_LEN];
                let mut key = [0u8; SEKC_KEY_HASH_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                context[0..8].copy_from_slice(&val.to_be_bytes());
                key[0..8].copy_from_slice(&val.to_be_bytes());
                KeyContextRecord { session_id: session, context, key_hash: key }
            })
            .collect();
        assert_eq!(
            validate_key_contexts(&rs),
            Err(KeyContextError::TooMany {
                got: SEKC_MAX_RECORDS + 1,
                max: SEKC_MAX_RECORDS,
            })
        );
    }

    /// **SEKC-07** — valid accepted.
    #[test]
    fn sekc_07_valid_accepted() {
        assert_eq!(validate_key_contexts(&valid_records()), Ok(()));
    }

    /// **SEKC-08** — empty accepted.
    #[test]
    fn sekc_08_empty_accepted() {
        assert_eq!(validate_key_contexts(&[]), Ok(()));
    }

    /// **SEKC-09** — single accepted.
    #[test]
    fn sekc_09_single_accepted() {
        let rs = vec![record(0x01, 0xA1, 0x11)];
        assert_eq!(validate_key_contexts(&rs), Ok(()));
    }

    /// **SEKC-10** — same key hash different session/context accepted.
    #[test]
    fn sekc_10_same_key_diff_session_accepted() {
        let rs = vec![record(0x01, 0xA1, 0x11), record(0x02, 0xA2, 0x11)];
        assert_eq!(validate_key_contexts(&rs), Ok(()));
    }
}
