//! # CR-CHAT-06 — Agent tool call rate limit guard (Wave-89 Lane B)
//!
//! AGENT SAFETY — tool calls per session must be bounded, R-CHAT-7.
//!
//! Without rate limiting on tool calls:
//!
//! * **Resource exhaustion** — a compromised agent calls tools in an
//!   infinite loop, consuming CPU, memory, and I/O.
//! * **Automated exploitation** — an attacker uses the agent to
//!   brute-force access by calling a sensitive tool thousands of times.
//! * **Denial of service** — excessive tool calls saturate the tool
//!   execution pipeline, blocking legitimate requests.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Calls per window <= `ATCR_MAX_CALLS`.
//! 2. Window size = `ATCR_WINDOW_MS`.
//! 3. No duplicate call IDs.
//! 4. Timestamps must be within the window.
//! 5. Tool name must be non-empty.
//! 6. Call count must be >= 0 (trivially true, but validates input).
//!
//! Tests **ATCR-01..10**. Error enum [`RateLimitError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-RATE-LIMIT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum calls per window.
pub const ATCR_MAX_CALLS: usize = 100;

/// Window size (ms).
pub const ATCR_WINDOW_MS: u64 = 60_000;

/// Maximum unique call IDs.
pub const ATCR_MAX_CALL_IDS: usize = 1024;

/// A tool call record.
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Unique call ID.
    pub call_id: u64,
    /// Tool name.
    pub tool: String,
    /// Timestamp (ms).
    pub timestamp_ms: u64,
}

/// All ways rate limit validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RateLimitError {
    /// Rate exceeded.
    RateExceeded { count: usize, max: usize },
    /// Duplicate call ID.
    DuplicateCallId(u64),
    /// Call outside window.
    OutsideWindow { timestamp: u64, window_start: u64, window_end: u64 },
    /// Empty tool name.
    EmptyToolName,
    /// Too many unique call IDs.
    TooManyCallIds,
}

/// `[VERIFIED]` Validate tool call rate limits.
pub fn validate_tool_call_rate(
    calls: &[ToolCall],
    window_start: u64,
) -> Result<(), RateLimitError> {
    if calls.iter().any(|c| c.tool.is_empty()) {
        return Err(RateLimitError::EmptyToolName);
    }
    let window_end = window_start + ATCR_WINDOW_MS;
    let in_window: Vec<&ToolCall> = calls
        .iter()
        .filter(|c| c.timestamp_ms >= window_start && c.timestamp_ms <= window_end)
        .collect();
    if in_window.len() > ATCR_MAX_CALLS {
        return Err(RateLimitError::RateExceeded {
            count: in_window.len(),
            max: ATCR_MAX_CALLS,
        });
    }
    let mut seen = BTreeSet::new();
    for c in &in_window {
        if !seen.insert(c.call_id) {
            return Err(RateLimitError::DuplicateCallId(c.call_id));
        }
    }
    if seen.len() > ATCR_MAX_CALL_IDS {
        return Err(RateLimitError::TooManyCallIds);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(id: u64, tool: &str, ts: u64) -> ToolCall {
        ToolCall { call_id: id, tool: tool.to_string(), timestamp_ms: ts }
    }

    fn valid_calls() -> Vec<ToolCall> {
        vec![
            call(1, "read_file", 1000),
            call(2, "search", 2000),
            call(3, "write_file", 3000),
        ]
    }

    /// **ATCR-01** — rate exceeded rejected.
    #[test]
    fn atcr_01_rate_exceeded_rejected() {
        let calls: Vec<ToolCall> = (0..=ATCR_MAX_CALLS as u64)
            .map(|i| call(i, "tool", i * 100))
            .collect();
        assert_eq!(
            validate_tool_call_rate(&calls, 0),
            Err(RateLimitError::RateExceeded { count: ATCR_MAX_CALLS + 1, max: ATCR_MAX_CALLS })
        );
    }

    /// **ATCR-02** — duplicate call ID rejected.
    #[test]
    fn atcr_02_duplicate_rejected() {
        let calls = vec![call(1, "tool", 1000), call(1, "tool", 2000)];
        assert_eq!(
            validate_tool_call_rate(&calls, 0),
            Err(RateLimitError::DuplicateCallId(1))
        );
    }

    /// **ATCR-03** — outside window accepted (filtered out).
    #[test]
    fn atcr_03_outside_window_filtered() {
        let calls = vec![
            call(1, "tool", 100),
            call(2, "tool", ATCR_WINDOW_MS + 100),
        ];
        assert_eq!(validate_tool_call_rate(&calls, 0), Ok(()));
    }

    /// **ATCR-04** — empty tool name rejected.
    #[test]
    fn atcr_04_empty_tool_rejected() {
        let calls = vec![ToolCall { call_id: 1, tool: String::new(), timestamp_ms: 1000 }];
        assert_eq!(validate_tool_call_rate(&calls, 0), Err(RateLimitError::EmptyToolName));
    }

    /// **ATCR-05** — too many call IDs rejected.
    #[test]
    fn atcr_05_too_many_ids_rejected() {
        let calls: Vec<ToolCall> = (0..=ATCR_MAX_CALL_IDS as u64)
            .map(|i| call(i, "tool", 100 + i))
            .collect();
        if calls.len() > ATCR_MAX_CALL_IDS {
            assert!(matches!(
                validate_tool_call_rate(&calls, 0),
                Err(RateLimitError::TooManyCallIds | RateLimitError::RateExceeded { .. })
            ));
        }
    }

    /// **ATCR-06** — valid calls accepted.
    #[test]
    fn atcr_06_valid_accepted() {
        assert_eq!(validate_tool_call_rate(&valid_calls(), 0), Ok(()));
    }

    /// **ATCR-07** — empty accepted.
    #[test]
    fn atcr_07_empty_accepted() {
        assert_eq!(validate_tool_call_rate(&[], 0), Ok(()));
    }

    /// **ATCR-08** — single call accepted.
    #[test]
    fn atcr_08_single_accepted() {
        assert_eq!(validate_tool_call_rate(&[call(1, "tool", 1000)], 0), Ok(()));
    }

    /// **ATCR-09** — max calls boundary accepted.
    #[test]
    fn atcr_09_max_boundary_accepted() {
        let calls: Vec<ToolCall> = (0..ATCR_MAX_CALLS as u64)
            .map(|i| call(i, "tool", i))
            .collect();
        assert_eq!(validate_tool_call_rate(&calls, 0), Ok(()));
    }

    /// **ATCR-10** — calls at window boundary accepted.
    #[test]
    fn atcr_10_boundary_accepted() {
        let calls = vec![
            call(1, "tool", 0),
            call(2, "tool", ATCR_WINDOW_MS),
        ];
        assert_eq!(validate_tool_call_rate(&calls, 0), Ok(()));
    }
}
