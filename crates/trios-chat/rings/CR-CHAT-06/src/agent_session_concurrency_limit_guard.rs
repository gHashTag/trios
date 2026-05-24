//! # CR-CHAT-06 — Agent session concurrency limit guard (Wave-102 Lane B)
//!
//! AGENT SAFETY — concurrent agent sessions must be bounded.
//!
//! Without concurrency limits on agent sessions:
//!
//! * **Resource exhaustion** — unlimited concurrent sessions consume
//!   memory, CPU, and network connections, starving other processes.
//! * **Cross-session interference** — concurrent sessions accessing
//!   shared state (tool registries, capability caches) can corrupt
//!   each other's results.
//! * **Priority inversion** — a flood of low-priority sessions blocks
//!   high-priority sessions from executing.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Concurrent sessions <= `ASCL_MAX_CONCURRENT`.
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. Per-user concurrent sessions <= `ASCL_MAX_PER_USER`.
//! 5. Session priority must be valid.
//! 6. Total sessions in batch <= `ASCL_MAX_BATCH`.
//!
//! Tests **ASCL-01..10**. Error enum [`ConcurrencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SESSION-CONCURRENCY`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Maximum total concurrent sessions.
pub const ASCL_MAX_CONCURRENT: usize = 64;

/// Maximum concurrent sessions per user.
pub const ASCL_MAX_PER_USER: usize = 8;

/// Maximum sessions per batch.
pub const ASCL_MAX_BATCH: usize = 256;

/// Session ID length.
pub const ASCL_SESSION_ID_LEN: usize = 16;

/// User ID length.
pub const ASCL_USER_ID_LEN: usize = 16;

/// Valid priority range.
pub const ASCL_MIN_PRIORITY: u8 = 0;
pub const ASCL_MAX_PRIORITY: u8 = 3;

/// A session record.
#[derive(Debug, Clone)]
pub struct SessionRecord {
    /// Session identifier.
    pub session_id: [u8; ASCL_SESSION_ID_LEN],
    /// User identifier.
    pub user_id: [u8; ASCL_USER_ID_LEN],
    /// Session priority.
    pub priority: u8,
}

/// All ways concurrency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConcurrencyError {
    /// Too many concurrent sessions.
    TooManyConcurrent { got: usize, max: usize },
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate session ID.
    DuplicateSession(usize),
    /// Per-user limit exceeded.
    PerUserExceeded { user_idx: usize, count: usize, max: usize },
    /// Invalid priority.
    InvalidPriority { idx: usize, priority: u8 },
    /// Batch too large.
    BatchTooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent session concurrency limits.
pub fn validate_session_concurrency(
    sessions: &[SessionRecord],
) -> Result<(), ConcurrencyError> {
    if sessions.len() > ASCL_MAX_BATCH {
        return Err(ConcurrencyError::BatchTooLarge {
            got: sessions.len(),
            max: ASCL_MAX_BATCH,
        });
    }
    if sessions.len() > ASCL_MAX_CONCURRENT {
        return Err(ConcurrencyError::TooManyConcurrent {
            got: sessions.len(),
            max: ASCL_MAX_CONCURRENT,
        });
    }
    let mut seen: BTreeSet<[u8; ASCL_SESSION_ID_LEN]> = BTreeSet::new();
    let mut user_counts: BTreeMap<[u8; ASCL_USER_ID_LEN], usize> = BTreeMap::new();
    for (i, s) in sessions.iter().enumerate() {
        if s.session_id == [0u8; ASCL_SESSION_ID_LEN] {
            return Err(ConcurrencyError::ZeroSession(i));
        }
        if s.priority > ASCL_MAX_PRIORITY {
            return Err(ConcurrencyError::InvalidPriority {
                idx: i,
                priority: s.priority,
            });
        }
        if !seen.insert(s.session_id) {
            return Err(ConcurrencyError::DuplicateSession(i));
        }
        let count = user_counts.entry(s.user_id).or_insert(0);
        *count += 1;
        if *count > ASCL_MAX_PER_USER {
            return Err(ConcurrencyError::PerUserExceeded {
                user_idx: i,
                count: *count,
                max: ASCL_MAX_PER_USER,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ASCL_SESSION_ID_LEN] {
        [byte; ASCL_SESSION_ID_LEN]
    }

    fn uid(byte: u8) -> [u8; ASCL_USER_ID_LEN] {
        [byte; ASCL_USER_ID_LEN]
    }

    fn session(sid_byte: u8, uid_byte: u8, priority: u8) -> SessionRecord {
        SessionRecord { session_id: sid(sid_byte), user_id: uid(uid_byte), priority }
    }

    fn valid_sessions() -> Vec<SessionRecord> {
        vec![
            session(0x01, 0xA0, 0),
            session(0x02, 0xA0, 1),
            session(0x03, 0xB0, 2),
        ]
    }

    /// **ASCL-01** — too many concurrent rejected.
    #[test]
    fn ascl_01_too_many_concurrent_rejected() {
        let ss: Vec<SessionRecord> = (0..=ASCL_MAX_CONCURRENT)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SessionRecord { session_id: sid(b), user_id: uid(b), priority: 0 }
            })
            .collect();
        assert!(matches!(
            validate_session_concurrency(&ss),
            Err(ConcurrencyError::TooManyConcurrent { .. })
        ));
    }

    /// **ASCL-02** — zero session rejected.
    #[test]
    fn ascl_02_zero_session_rejected() {
        let s = SessionRecord { session_id: [0u8; ASCL_SESSION_ID_LEN], user_id: uid(0x01), priority: 0 };
        assert_eq!(
            validate_session_concurrency(&[s]),
            Err(ConcurrencyError::ZeroSession(0))
        );
    }

    /// **ASCL-03** — duplicate session rejected.
    #[test]
    fn ascl_03_duplicate_session_rejected() {
        let ss = vec![session(0x01, 0xA0, 0), session(0x01, 0xA0, 1)];
        assert_eq!(
            validate_session_concurrency(&ss),
            Err(ConcurrencyError::DuplicateSession(1))
        );
    }

    /// **ASCL-04** — per-user exceeded rejected.
    #[test]
    fn ascl_04_per_user_exceeded_rejected() {
        let ss: Vec<SessionRecord> = (0..=ASCL_MAX_PER_USER)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SessionRecord { session_id: sid(b), user_id: uid(0xA0), priority: 0 }
            })
            .collect();
        assert!(matches!(
            validate_session_concurrency(&ss),
            Err(ConcurrencyError::PerUserExceeded { .. })
        ));
    }

    /// **ASCL-05** — invalid priority rejected.
    #[test]
    fn ascl_05_invalid_priority_rejected() {
        let s = SessionRecord { session_id: sid(0x01), user_id: uid(0xA0), priority: ASCL_MAX_PRIORITY + 1 };
        assert_eq!(
            validate_session_concurrency(&[s]),
            Err(ConcurrencyError::InvalidPriority { idx: 0, priority: ASCL_MAX_PRIORITY + 1 })
        );
    }

    /// **ASCL-06** — batch too large rejected.
    #[test]
    fn ascl_06_batch_too_large_rejected() {
        let ss: Vec<SessionRecord> = (0..=ASCL_MAX_BATCH)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SessionRecord { session_id: sid(b), user_id: uid(b), priority: 0 }
            })
            .collect();
        assert!(matches!(
            validate_session_concurrency(&ss),
            Err(ConcurrencyError::BatchTooLarge { .. })
        ));
    }

    /// **ASCL-07** — valid accepted.
    #[test]
    fn ascl_07_valid_accepted() {
        assert_eq!(validate_session_concurrency(&valid_sessions()), Ok(()));
    }

    /// **ASCL-08** — empty accepted.
    #[test]
    fn ascl_08_empty_accepted() {
        assert_eq!(validate_session_concurrency(&[]), Ok(()));
    }

    /// **ASCL-09** — single accepted.
    #[test]
    fn ascl_09_single_accepted() {
        let ss = vec![session(0x01, 0xA0, 0)];
        assert_eq!(validate_session_concurrency(&ss), Ok(()));
    }

    /// **ASCL-10** — per-user boundary accepted.
    #[test]
    fn ascl_10_per_user_boundary_accepted() {
        let ss: Vec<SessionRecord> = (0..ASCL_MAX_PER_USER)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SessionRecord { session_id: sid(b), user_id: uid(0xA0), priority: 0 }
            })
            .collect();
        assert_eq!(validate_session_concurrency(&ss), Ok(()));
    }
}
