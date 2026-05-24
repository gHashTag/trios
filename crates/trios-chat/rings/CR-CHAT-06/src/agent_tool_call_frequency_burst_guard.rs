//! # CR-CHAT-06 — Agent tool call frequency burst guard (Wave-138 Lane A)
//!
//! AGENT SAFETY — rate of tool calls per session must be bounded;
//! excessive rate indicates automated exploitation.
//!
//! An agent session making tool calls at an excessive rate may be
//! under automated control (e.g. a compromised agent executing a
//! rapid-fire attack chain):
//!
//! * **Resource exhaustion** — rapid tool calls consume server-side
//!   resources (API calls, compute, storage).
//! * **Automated exploitation** — an attacker scripting tool calls
//!   can probe many attack vectors in a short time.
//! * **Rate evasion** — without a burst limit, an attacker can
//!   stay under a per-minute average while sending damaging bursts.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Calls in window <= `ATCF_MAX_BURST`.
//! 2. Window duration <= `ATCF_MAX_WINDOW_MS`.
//! 3. Session ID must not be zero.
//! 4. No duplicate session IDs.
//! 5. Timestamp must be > 0.
//! 6. Batch size <= `ATCF_MAX_ENTRIES`.
//!
//! Tests **ATCF-01..10**. Error enum [`BurstError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATE-LIMITED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum calls in burst window.
pub const ATCF_MAX_BURST: usize = 30;

/// Maximum window duration in milliseconds.
pub const ATCF_MAX_WINDOW_MS: u64 = 60_000;

/// Maximum entries per batch.
pub const ATCF_MAX_ENTRIES: usize = 512;

/// Session ID length.
pub const ATCF_SESSION_ID_LEN: usize = 32;

/// A tool call burst record.
#[derive(Debug, Clone)]
pub struct ToolCallBurst {
    /// Session identifier.
    pub session_id: [u8; ATCF_SESSION_ID_LEN],
    /// Timestamp of the tool call (ms since epoch).
    pub timestamp_ms: u64,
    /// Number of calls in this burst window.
    pub call_count: usize,
    /// Window duration in ms.
    pub window_ms: u64,
}

/// All ways tool call burst validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BurstError {
    /// Burst count exceeded.
    BurstExceeded { idx: usize, got: usize, max: usize },
    /// Window too large.
    WindowTooLarge { idx: usize, got: u64, max: u64 },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session ID.
    DuplicateSessionId { idx: usize },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Batch too large.
    TooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate agent tool call frequency burst.
pub fn validate_tool_call_burst(
    records: &[ToolCallBurst],
) -> Result<(), BurstError> {
    if records.len() > ATCF_MAX_ENTRIES {
        return Err(BurstError::TooLarge {
            got: records.len(),
            max: ATCF_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; ATCF_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; ATCF_SESSION_ID_LEN] {
            return Err(BurstError::ZeroSessionId(i));
        }
        if r.timestamp_ms == 0 {
            return Err(BurstError::ZeroTimestamp(i));
        }
        if !seen.insert(r.session_id) {
            return Err(BurstError::DuplicateSessionId { idx: i });
        }
        if r.window_ms > ATCF_MAX_WINDOW_MS {
            return Err(BurstError::WindowTooLarge {
                idx: i,
                got: r.window_ms,
                max: ATCF_MAX_WINDOW_MS,
            });
        }
        if r.call_count > ATCF_MAX_BURST {
            return Err(BurstError::BurstExceeded {
                idx: i,
                got: r.call_count,
                max: ATCF_MAX_BURST,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ATCF_SESSION_ID_LEN] {
        [byte; ATCF_SESSION_ID_LEN]
    }

    fn burst(session: u8, ts: u64, count: usize, window: u64) -> ToolCallBurst {
        ToolCallBurst { session_id: sid(session), timestamp_ms: ts, call_count: count, window_ms: window }
    }

    fn valid_records() -> Vec<ToolCallBurst> {
        vec![
            burst(0x01, 1000, 10, 5000),
            burst(0x02, 2000, 20, 10000),
        ]
    }

    /// **ATCF-01** — burst exceeded rejected.
    #[test]
    fn atcf_01_burst_exceeded_rejected() {
        let r = burst(0x01, 1000, ATCF_MAX_BURST + 1, 5000);
        assert_eq!(
            validate_tool_call_burst(&[r]),
            Err(BurstError::BurstExceeded {
                idx: 0,
                got: ATCF_MAX_BURST + 1,
                max: ATCF_MAX_BURST,
            })
        );
    }

    /// **ATCF-02** — window too large rejected.
    #[test]
    fn atcf_02_window_too_large_rejected() {
        let r = burst(0x01, 1000, 5, ATCF_MAX_WINDOW_MS + 1);
        assert_eq!(
            validate_tool_call_burst(&[r]),
            Err(BurstError::WindowTooLarge {
                idx: 0,
                got: ATCF_MAX_WINDOW_MS + 1,
                max: ATCF_MAX_WINDOW_MS,
            })
        );
    }

    /// **ATCF-03** — zero session ID rejected.
    #[test]
    fn atcf_03_zero_session_rejected() {
        let r = ToolCallBurst {
            session_id: [0u8; ATCF_SESSION_ID_LEN],
            timestamp_ms: 1000,
            call_count: 5,
            window_ms: 5000,
        };
        assert_eq!(
            validate_tool_call_burst(&[r]),
            Err(BurstError::ZeroSessionId(0))
        );
    }

    /// **ATCF-04** — duplicate session ID rejected.
    #[test]
    fn atcf_04_duplicate_rejected() {
        let rs = vec![
            burst(0x01, 1000, 5, 5000),
            burst(0x01, 2000, 10, 5000),
        ];
        assert_eq!(
            validate_tool_call_burst(&rs),
            Err(BurstError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **ATCF-05** — zero timestamp rejected.
    #[test]
    fn atcf_05_zero_timestamp_rejected() {
        let r = ToolCallBurst { session_id: sid(0x01), timestamp_ms: 0, call_count: 5, window_ms: 5000 };
        assert_eq!(
            validate_tool_call_burst(&[r]),
            Err(BurstError::ZeroTimestamp(0))
        );
    }

    /// **ATCF-06** — batch too large rejected.
    #[test]
    fn atcf_06_too_large_rejected() {
        let rs: Vec<ToolCallBurst> = (0..=ATCF_MAX_ENTRIES)
            .map(|i| {
                let mut s = [0u8; ATCF_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                ToolCallBurst { session_id: s, timestamp_ms: val, call_count: 1, window_ms: 1000 }
            })
            .collect();
        assert_eq!(
            validate_tool_call_burst(&rs),
            Err(BurstError::TooLarge {
                got: ATCF_MAX_ENTRIES + 1,
                max: ATCF_MAX_ENTRIES,
            })
        );
    }

    /// **ATCF-07** — valid accepted.
    #[test]
    fn atcf_07_valid_accepted() {
        assert_eq!(validate_tool_call_burst(&valid_records()), Ok(()));
    }

    /// **ATCF-08** — empty accepted.
    #[test]
    fn atcf_08_empty_accepted() {
        assert_eq!(validate_tool_call_burst(&[]), Ok(()));
    }

    /// **ATCF-09** — boundary burst accepted.
    #[test]
    fn atcf_09_boundary_burst_accepted() {
        let r = burst(0x01, 1000, ATCF_MAX_BURST, ATCF_MAX_WINDOW_MS);
        assert_eq!(validate_tool_call_burst(&[r]), Ok(()));
    }

    /// **ATCF-10** — single call accepted.
    #[test]
    fn atcf_10_single_call_accepted() {
        let r = burst(0x01, 1000, 1, 1000);
        assert_eq!(validate_tool_call_burst(&[r]), Ok(()));
    }
}
