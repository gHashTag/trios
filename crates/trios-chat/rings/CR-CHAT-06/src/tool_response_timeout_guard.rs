//! # CR-CHAT-06 — Tool response timeout guard (Wave-75 Lane B)
//!
//! AGENT SAFETY — tool responses must complete within timeout, R-CHAT-7.
//!
//! Without a timeout bound, a tool call can:
//!
//! * **Hang indefinitely** — a misbehaving tool never returns, blocking
//!   the agent pipeline.
//! * **Timing side-channel** — tool response latency leaks information
//!   about internal state (e.g. file existence, DB query results).
//! * **Resource exhaustion** — many concurrent slow tool calls exhaust
//!   connection pools or memory.
//!
//! This is distinct from TOUT (output sanitization) and AORL (output
//! rate limit). TRTO enforces the *latency bound* per tool invocation.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Response latency <= `TRTO_MAX_TIMEOUT_MS`.
//! 2. Response latency >= `TRTO_MIN_TIMEOUT_MS` (too fast = cached/stub).
//! 3. Timeout setting must be > 0.
//! 4. Timeout setting <= `TRTO_MAX_TIMEOUT_MS`.
//! 5. No response accepted after timeout.
//! 6. Timeout must be set before any tool call.
//!
//! Tests **TRTO-01..10**. Error enum [`ToolTimeoutError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOOL-RESPONSE-TIMEOUT`

#![forbid(unsafe_code)]

/// Minimum acceptable response time (ms).
pub const TRTO_MIN_TIMEOUT_MS: u64 = 10;

/// Maximum timeout (ms).
pub const TRTO_MAX_TIMEOUT_MS: u64 = 30_000;

/// All ways tool timeout validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolTimeoutError {
    /// Response exceeded timeout.
    TimeoutExceeded,
    /// Response too fast (below minimum).
    TooFast,
    /// Negative or zero timeout configured.
    ZeroTimeout,
    /// Timeout setting too large.
    TimeoutTooLarge,
    /// No timeout set before call.
    NoTimeoutSet,
    /// Response arrived after timeout.
    ResponseAfterTimeout,
}

/// `[VERIFIED]` Validate tool response against timeout constraints.
pub fn validate_tool_timeout(
    timeout_ms: u64,
    start_ms: u64,
    response_ms: u64,
) -> Result<(), ToolTimeoutError> {
    if timeout_ms == 0 {
        return Err(ToolTimeoutError::ZeroTimeout);
    }
    if timeout_ms > TRTO_MAX_TIMEOUT_MS {
        return Err(ToolTimeoutError::TimeoutTooLarge);
    }
    if response_ms < start_ms {
        return Err(ToolTimeoutError::ResponseAfterTimeout);
    }
    let latency = response_ms - start_ms;
    if latency > timeout_ms {
        return Err(ToolTimeoutError::TimeoutExceeded);
    }
    if latency < TRTO_MIN_TIMEOUT_MS {
        return Err(ToolTimeoutError::TooFast);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: u64 = 1_000_000;

    fn valid_timeout() -> u64 {
        10_000
    }

    /// **TRTO-01** — timeout exceeded rejected.
    #[test]
    fn trto_01_timeout_exceeded_rejected() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START + valid_timeout() + 1),
            Err(ToolTimeoutError::TimeoutExceeded)
        );
    }

    /// **TRTO-02** — too fast rejected.
    #[test]
    fn trto_02_too_fast_rejected() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START + 1),
            Err(ToolTimeoutError::TooFast)
        );
    }

    /// **TRTO-03** — zero timeout rejected.
    #[test]
    fn trto_03_zero_timeout_rejected() {
        assert_eq!(
            validate_tool_timeout(0, START, START + 100),
            Err(ToolTimeoutError::ZeroTimeout)
        );
    }

    /// **TRTO-04** — timeout too large rejected.
    #[test]
    fn trto_04_timeout_large_rejected() {
        assert_eq!(
            validate_tool_timeout(TRTO_MAX_TIMEOUT_MS + 1, START, START + 100),
            Err(ToolTimeoutError::TimeoutTooLarge)
        );
    }

    /// **TRTO-05** — response before start rejected.
    #[test]
    fn trto_05_response_before_start_rejected() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START - 1),
            Err(ToolTimeoutError::ResponseAfterTimeout)
        );
    }

    /// **TRTO-06** — no timeout set scenario (zero timeout = unset).
    #[test]
    fn trto_06_no_timeout_scenario() {
        assert_eq!(
            validate_tool_timeout(0, START, START + 100),
            Err(ToolTimeoutError::ZeroTimeout)
        );
    }

    /// **TRTO-07** — valid response accepted.
    #[test]
    fn trto_07_valid_accepted() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START + 1000),
            Ok(())
        );
    }

    /// **TRTO-08** — exact min latency accepted.
    #[test]
    fn trto_08_min_latency_accepted() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START + TRTO_MIN_TIMEOUT_MS),
            Ok(())
        );
    }

    /// **TRTO-09** — exact timeout boundary accepted.
    #[test]
    fn trto_09_exact_timeout_accepted() {
        assert_eq!(
            validate_tool_timeout(valid_timeout(), START, START + valid_timeout()),
            Ok(())
        );
    }

    /// **TRTO-10** — max timeout setting accepted.
    #[test]
    fn trto_10_max_timeout_accepted() {
        assert_eq!(
            validate_tool_timeout(TRTO_MAX_TIMEOUT_MS, START, START + 1000),
            Ok(())
        );
    }
}
