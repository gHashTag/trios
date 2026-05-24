//! # CR-CHAT-06 — Agent session timeout enforcement guard (Wave-115 Lane A)
//!
//! AGENT SAFETY — sessions must not exceed configured timeout.
//!
//! Each agent session has a maximum lifetime. If a session exceeds
//! its timeout:
//!
//! * **State leakage** — long-running sessions accumulate internal
//!   state that may leak to tool outputs or subsequent prompts.
//! * **Context poisoning** — the longer a session runs, the more
//!   susceptible it is to gradual context manipulation.
//! * **Resource accumulation** — sessions hold memory, connections,
//!   and file handles that should be released promptly.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Session duration <= configured timeout.
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. Timeout must be >= `ASTE_MIN_TIMEOUT_MS`.
//! 5. Timeout must be <= `ASTE_MAX_TIMEOUT_MS`.
//! 6. Total sessions <= `ASTE_MAX_SESSIONS`.
//!
//! Tests **ASTE-01..10**. Error enum [`SessionTimeoutError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SESSION-TIMEOUT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum timeout in milliseconds.
pub const ASTE_MIN_TIMEOUT_MS: u64 = 60_000;

/// Maximum timeout in milliseconds.
pub const ASTE_MAX_TIMEOUT_MS: u64 = 86_400_000;

/// Maximum sessions per batch.
pub const ASTE_MAX_SESSIONS: usize = 256;

/// Session ID length.
pub const ASTE_SESSION_ID_LEN: usize = 16;

/// A session timeout record.
#[derive(Debug, Clone)]
pub struct SessionTimeout {
    /// Session identifier.
    pub session_id: [u8; ASTE_SESSION_ID_LEN],
    /// Session start time (ms since epoch).
    pub started_at: u64,
    /// Current time (ms since epoch).
    pub now_ms: u64,
    /// Configured timeout (ms).
    pub timeout_ms: u64,
}

/// All ways session timeout validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionTimeoutError {
    /// Session exceeded timeout.
    Exceeded { idx: usize, duration_ms: u64, timeout_ms: u64 },
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate session.
    DuplicateSession(usize),
    /// Timeout below minimum.
    BelowMin { idx: usize, timeout_ms: u64, min: u64 },
    /// Timeout above maximum.
    AboveMax { idx: usize, timeout_ms: u64, max: u64 },
    /// Too many sessions.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent session timeout enforcement.
pub fn validate_session_timeouts(
    sessions: &[SessionTimeout],
) -> Result<(), SessionTimeoutError> {
    if sessions.len() > ASTE_MAX_SESSIONS {
        return Err(SessionTimeoutError::TooMany {
            got: sessions.len(),
            max: ASTE_MAX_SESSIONS,
        });
    }
    let mut seen: BTreeSet<[u8; ASTE_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, s) in sessions.iter().enumerate() {
        if s.session_id == [0u8; ASTE_SESSION_ID_LEN] {
            return Err(SessionTimeoutError::ZeroSession(i));
        }
        if s.timeout_ms < ASTE_MIN_TIMEOUT_MS {
            return Err(SessionTimeoutError::BelowMin {
                idx: i,
                timeout_ms: s.timeout_ms,
                min: ASTE_MIN_TIMEOUT_MS,
            });
        }
        if s.timeout_ms > ASTE_MAX_TIMEOUT_MS {
            return Err(SessionTimeoutError::AboveMax {
                idx: i,
                timeout_ms: s.timeout_ms,
                max: ASTE_MAX_TIMEOUT_MS,
            });
        }
        if s.now_ms >= s.started_at {
            let duration = s.now_ms - s.started_at;
            if duration > s.timeout_ms {
                return Err(SessionTimeoutError::Exceeded {
                    idx: i,
                    duration_ms: duration,
                    timeout_ms: s.timeout_ms,
                });
            }
        }
        if !seen.insert(s.session_id) {
            return Err(SessionTimeoutError::DuplicateSession(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ASTE_SESSION_ID_LEN] {
        [byte; ASTE_SESSION_ID_LEN]
    }

    fn session(id: u8, start: u64, now: u64, timeout: u64) -> SessionTimeout {
        SessionTimeout { session_id: sid(id), started_at: start, now_ms: now, timeout_ms: timeout }
    }

    fn valid_sessions() -> Vec<SessionTimeout> {
        vec![
            session(0x01, 1000, 5000, 60_000),
            session(0x02, 2000, 3000, 120_000),
        ]
    }

    /// **ASTE-01** — exceeded rejected.
    #[test]
    fn aste_01_exceeded_rejected() {
        let s = session(0x01, 1000, 200_000, 60_000);
        assert_eq!(
            validate_session_timeouts(&[s]),
            Err(SessionTimeoutError::Exceeded {
                idx: 0,
                duration_ms: 199_000,
                timeout_ms: 60_000,
            })
        );
    }

    /// **ASTE-02** — zero session rejected.
    #[test]
    fn aste_02_zero_session_rejected() {
        let s = SessionTimeout { session_id: [0u8; ASTE_SESSION_ID_LEN], started_at: 1000, now_ms: 2000, timeout_ms: 60_000 };
        assert_eq!(
            validate_session_timeouts(&[s]),
            Err(SessionTimeoutError::ZeroSession(0))
        );
    }

    /// **ASTE-03** — duplicate rejected.
    #[test]
    fn aste_03_duplicate_rejected() {
        let ss = vec![session(0x01, 1000, 2000, 60_000), session(0x01, 2000, 3000, 60_000)];
        assert_eq!(
            validate_session_timeouts(&ss),
            Err(SessionTimeoutError::DuplicateSession(1))
        );
    }

    /// **ASTE-04** — below min rejected.
    #[test]
    fn aste_04_below_min_rejected() {
        let s = session(0x01, 1000, 2000, 10_000);
        assert_eq!(
            validate_session_timeouts(&[s]),
            Err(SessionTimeoutError::BelowMin {
                idx: 0,
                timeout_ms: 10_000,
                min: ASTE_MIN_TIMEOUT_MS,
            })
        );
    }

    /// **ASTE-05** — above max rejected.
    #[test]
    fn aste_05_above_max_rejected() {
        let s = session(0x01, 1000, 2000, ASTE_MAX_TIMEOUT_MS + 1);
        assert_eq!(
            validate_session_timeouts(&[s]),
            Err(SessionTimeoutError::AboveMax {
                idx: 0,
                timeout_ms: ASTE_MAX_TIMEOUT_MS + 1,
                max: ASTE_MAX_TIMEOUT_MS,
            })
        );
    }

    /// **ASTE-06** — too many rejected.
    #[test]
    fn aste_06_too_many_rejected() {
        let ss: Vec<SessionTimeout> = (0..=ASTE_MAX_SESSIONS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SessionTimeout { session_id: sid(b), started_at: 1000, now_ms: 2000, timeout_ms: 60_000 }
            })
            .collect();
        assert_eq!(
            validate_session_timeouts(&ss),
            Err(SessionTimeoutError::TooMany {
                got: ASTE_MAX_SESSIONS + 1,
                max: ASTE_MAX_SESSIONS,
            })
        );
    }

    /// **ASTE-07** — valid accepted.
    #[test]
    fn aste_07_valid_accepted() {
        assert_eq!(validate_session_timeouts(&valid_sessions()), Ok(()));
    }

    /// **ASTE-08** — empty accepted.
    #[test]
    fn aste_08_empty_accepted() {
        assert_eq!(validate_session_timeouts(&[]), Ok(()));
    }

    /// **ASTE-09** — boundary duration accepted.
    #[test]
    fn aste_09_boundary_accepted() {
        let s = session(0x01, 1000, 61_000, 60_000);
        assert_eq!(validate_session_timeouts(&[s]), Ok(()));
    }

    /// **ASTE-10** — max timeout accepted.
    #[test]
    fn aste_10_max_timeout_accepted() {
        let s = session(0x01, 1000, 2000, ASTE_MAX_TIMEOUT_MS);
        assert_eq!(validate_session_timeouts(&[s]), Ok(()));
    }
}
