//! # CR-CHAT-06 — Agent rate limit burst detection guard (Wave-156 Lane B)
//!
//! AGENT SAFETY — agent actions must respect rate limits; burst
//! patterns indicate automation or abuse.
//!
//! When agents perform actions, they must respect configured rate
//! limits. If actions come in bursts:
//!
//! * **Automation detection** — a compromised agent may flood the
//!   system with automated actions.
//! * **Resource exhaustion** — bursty action patterns consume
//!   disproportionate resources.
//! * **Abuse vector** — rate limit violations are a strong signal
//!   of compromised or malicious agents.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Actions per window <= `ARLB_MAX_ACTIONS`.
//! 2. Window duration <= `ARLB_MAX_WINDOW_MS`.
//! 3. Session ID must not be zero.
//! 4. No duplicate session IDs.
//! 5. Action count must be > 0.
//! 6. Batch size <= `ARLB_MAX_SESSIONS`.
//!
//! Tests **ARLB-01..10**. Error enum [`RateLimitBurstError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATE-LIMITED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum actions per window.
pub const ARLB_MAX_ACTIONS: usize = 100;

/// Maximum window duration in milliseconds.
pub const ARLB_MAX_WINDOW_MS: u64 = 60_000;

/// Maximum sessions per batch.
pub const ARLB_MAX_SESSIONS: usize = 256;

/// Session ID length.
pub const ARLB_SESSION_ID_LEN: usize = 32;

/// A rate limit session record.
#[derive(Debug, Clone)]
pub struct RateSession {
    /// Session identifier.
    pub session_id: [u8; ARLB_SESSION_ID_LEN],
    /// Number of actions in the window.
    pub action_count: usize,
    /// Window duration in milliseconds.
    pub window_ms: u64,
}

/// All ways rate limit burst validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RateLimitBurstError {
    /// Too many actions in window.
    TooManyActions {
        idx: usize,
        got: usize,
        max: usize,
    },
    /// Window too large.
    WindowTooLarge {
        idx: usize,
        got: u64,
        max: u64,
    },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId {
        idx: usize,
    },
    /// Zero action count.
    ZeroActionCount(usize),
    /// Too many sessions.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate agent rate limit burst.
pub fn validate_rate_limit_burst(
    sessions: &[RateSession],
) -> Result<(), RateLimitBurstError> {
    if sessions.len() > ARLB_MAX_SESSIONS {
        return Err(RateLimitBurstError::TooMany {
            got: sessions.len(),
            max: ARLB_MAX_SESSIONS,
        });
    }
    let mut seen: BTreeSet<[u8; ARLB_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, s) in sessions.iter().enumerate() {
        if s.session_id == [0u8; ARLB_SESSION_ID_LEN] {
            return Err(RateLimitBurstError::ZeroSessionId(i));
        }
        if !seen.insert(s.session_id) {
            return Err(RateLimitBurstError::DuplicateSessionId { idx: i });
        }
        if s.action_count == 0 {
            return Err(RateLimitBurstError::ZeroActionCount(i));
        }
        if s.action_count > ARLB_MAX_ACTIONS {
            return Err(RateLimitBurstError::TooManyActions {
                idx: i,
                got: s.action_count,
                max: ARLB_MAX_ACTIONS,
            });
        }
        if s.window_ms > ARLB_MAX_WINDOW_MS {
            return Err(RateLimitBurstError::WindowTooLarge {
                idx: i,
                got: s.window_ms,
                max: ARLB_MAX_WINDOW_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ARLB_SESSION_ID_LEN] {
        [byte; ARLB_SESSION_ID_LEN]
    }

    fn session(id: u8, actions: usize, window_ms: u64) -> RateSession {
        RateSession { session_id: sid(id), action_count: actions, window_ms }
    }

    fn valid_sessions() -> Vec<RateSession> {
        vec![
            session(0x01, 10, 10_000),
            session(0x02, 50, 30_000),
            session(0x03, 80, 60_000),
        ]
    }

    /// **ARLB-01** — too many actions rejected.
    #[test]
    fn arlb_01_too_many_actions_rejected() {
        let s = session(0x01, ARLB_MAX_ACTIONS + 1, 10_000);
        assert_eq!(
            validate_rate_limit_burst(&[s]),
            Err(RateLimitBurstError::TooManyActions {
                idx: 0,
                got: ARLB_MAX_ACTIONS + 1,
                max: ARLB_MAX_ACTIONS,
            })
        );
    }

    /// **ARLB-02** — window too large rejected.
    #[test]
    fn arlb_02_window_too_large_rejected() {
        let s = session(0x01, 10, ARLB_MAX_WINDOW_MS + 1);
        assert_eq!(
            validate_rate_limit_burst(&[s]),
            Err(RateLimitBurstError::WindowTooLarge {
                idx: 0,
                got: ARLB_MAX_WINDOW_MS + 1,
                max: ARLB_MAX_WINDOW_MS,
            })
        );
    }

    /// **ARLB-03** — zero session ID rejected.
    #[test]
    fn arlb_03_zero_session_rejected() {
        let s = RateSession { session_id: [0u8; ARLB_SESSION_ID_LEN], action_count: 10, window_ms: 10_000 };
        assert_eq!(
            validate_rate_limit_burst(&[s]),
            Err(RateLimitBurstError::ZeroSessionId(0))
        );
    }

    /// **ARLB-04** — duplicate session rejected.
    #[test]
    fn arlb_04_duplicate_rejected() {
        let ss = vec![
            session(0x01, 10, 10_000),
            session(0x01, 20, 20_000),
        ];
        assert_eq!(
            validate_rate_limit_burst(&ss),
            Err(RateLimitBurstError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **ARLB-05** — zero action count rejected.
    #[test]
    fn arlb_05_zero_actions_rejected() {
        let s = session(0x01, 0, 10_000);
        assert_eq!(
            validate_rate_limit_burst(&[s]),
            Err(RateLimitBurstError::ZeroActionCount(0))
        );
    }

    /// **ARLB-06** — too many sessions rejected.
    #[test]
    fn arlb_06_too_many_rejected() {
        let ss: Vec<RateSession> = (0..=ARLB_MAX_SESSIONS)
            .map(|i| {
                let mut id = [0u8; ARLB_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                RateSession { session_id: id, action_count: 10, window_ms: 10_000 }
            })
            .collect();
        assert_eq!(
            validate_rate_limit_burst(&ss),
            Err(RateLimitBurstError::TooMany {
                got: ARLB_MAX_SESSIONS + 1,
                max: ARLB_MAX_SESSIONS,
            })
        );
    }

    /// **ARLB-07** — valid accepted.
    #[test]
    fn arlb_07_valid_accepted() {
        assert_eq!(validate_rate_limit_burst(&valid_sessions()), Ok(()));
    }

    /// **ARLB-08** — empty accepted.
    #[test]
    fn arlb_08_empty_accepted() {
        assert_eq!(validate_rate_limit_burst(&[]), Ok(()));
    }

    /// **ARLB-09** — boundary actions accepted.
    #[test]
    fn arlb_09_boundary_actions_accepted() {
        let s = session(0x01, ARLB_MAX_ACTIONS, ARLB_MAX_WINDOW_MS);
        assert_eq!(validate_rate_limit_burst(&[s]), Ok(()));
    }

    /// **ARLB-10** — many valid accepted.
    #[test]
    fn arlb_10_many_valid_accepted() {
        let ss: Vec<RateSession> = (0..20u8)
            .map(|i| session(i + 1, (i as usize) * 4 + 1, (i as u64) * 2000 + 1000))
            .collect();
        assert_eq!(validate_rate_limit_burst(&ss), Ok(()));
    }
}
