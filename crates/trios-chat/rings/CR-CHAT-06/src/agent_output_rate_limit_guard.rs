//! # CR-CHAT-06 — Agent output rate limit guard (Wave-64 Lane B)
//!
//! AGENT SAFETY — output rate limiting prevents flood, R-CHAT-7.
//!
//! An agent that produces output too fast can:
//!
//! * **Flood downstream** — overwhelm UI renderer or API consumer.
//! * **Side-channel signal** — encode data in output timing for an
//!   external observer.
//! * **Resource exhaustion** — generate output faster than it can be
//!   consumed, exhausting memory buffers.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Output count in window <= `AORL_MAX_OUTPUTS`.
//! 2. Window duration <= `AORL_MAX_WINDOW_MS`.
//! 3. No zero-duration burst (all outputs at same timestamp).
//! 4. Minimum interval between outputs >= `AORL_MIN_INTERVAL_MS`.
//! 5. Total output bytes in window <= `AORL_MAX_BYTES`.
//! 6. First output timestamp <= last output timestamp.
//!
//! Tests **AORL-01..10**. Error enum [`OutputRateError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * OUTPUT-RATE-LIMIT`

#![forbid(unsafe_code)]

/// Maximum outputs per window.
pub const AORL_MAX_OUTPUTS: usize = 64;

/// Maximum window duration (ms).
pub const AORL_MAX_WINDOW_MS: u64 = 60_000;

/// Minimum interval between outputs (ms).
pub const AORL_MIN_INTERVAL_MS: u64 = 50;

/// Maximum total bytes per window.
pub const AORL_MAX_BYTES: usize = 1_048_576;

/// An output event.
#[derive(Debug, Clone)]
pub struct OutputEvent {
    /// Timestamp (ms since epoch).
    pub timestamp_ms: u64,
    /// Output size in bytes.
    pub size: usize,
}

/// All ways output rate validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OutputRateError {
    /// Too many outputs in window.
    TooManyOutputs,
    /// Window too long.
    WindowTooLong,
    /// Zero-duration burst.
    ZeroDurationBurst,
    /// Interval too short.
    IntervalTooShort,
    /// Total bytes exceeded.
    TotalBytesExceeded,
    /// Timestamps not monotonic.
    TimestampsNotMonotonic,
}

/// `[VERIFIED]` Validate output rate within a window.
pub fn validate_output_rate(
    events: &[OutputEvent],
) -> Result<(), OutputRateError> {
    if events.len() > AORL_MAX_OUTPUTS {
        return Err(OutputRateError::TooManyOutputs);
    }
    if events.is_empty() {
        return Ok(());
    }
    let total_bytes: usize = events.iter().map(|e| e.size).sum();
    if total_bytes > AORL_MAX_BYTES {
        return Err(OutputRateError::TotalBytesExceeded);
    }
    let first = events[0].timestamp_ms;
    let last = events[events.len() - 1].timestamp_ms;
    if last < first {
        return Err(OutputRateError::TimestampsNotMonotonic);
    }
    if last - first > AORL_MAX_WINDOW_MS {
        return Err(OutputRateError::WindowTooLong);
    }
    if events.len() >= 2 && first == last {
        return Err(OutputRateError::ZeroDurationBurst);
    }
    for w in events.windows(2) {
        if w[1].timestamp_ms < w[0].timestamp_ms {
            return Err(OutputRateError::TimestampsNotMonotonic);
        }
        if w[1].timestamp_ms - w[0].timestamp_ms < AORL_MIN_INTERVAL_MS {
            return Err(OutputRateError::IntervalTooShort);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(ts: u64, size: usize) -> OutputEvent {
        OutputEvent { timestamp_ms: ts, size }
    }

    fn good_events() -> Vec<OutputEvent> {
        vec![
            event(1000, 100),
            event(1100, 200),
            event(1200, 150),
        ]
    }

    /// **AORL-01** — too many outputs rejected.
    #[test]
    fn aorl_01_too_many_rejected() {
        let events: Vec<OutputEvent> = (0..=AORL_MAX_OUTPUTS)
            .map(|i| event(i as u64 * 100, 100))
            .collect();
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::TooManyOutputs)
        );
    }

    /// **AORL-02** — window too long rejected.
    #[test]
    fn aorl_02_window_long_rejected() {
        let events = vec![
            event(0, 100),
            event(AORL_MAX_WINDOW_MS + 1, 100),
        ];
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::WindowTooLong)
        );
    }

    /// **AORL-03** — zero-duration burst rejected.
    #[test]
    fn aorl_03_zero_burst_rejected() {
        let events = vec![event(1000, 100), event(1000, 200)];
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::ZeroDurationBurst)
        );
    }

    /// **AORL-04** — interval too short rejected.
    #[test]
    fn aorl_04_interval_short_rejected() {
        let events = vec![event(1000, 100), event(1010, 100)];
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::IntervalTooShort)
        );
    }

    /// **AORL-05** — total bytes exceeded rejected.
    #[test]
    fn aorl_05_bytes_exceeded_rejected() {
        let events = vec![event(1000, AORL_MAX_BYTES + 1)];
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::TotalBytesExceeded)
        );
    }

    /// **AORL-06** — timestamps not monotonic rejected.
    #[test]
    fn aorl_06_not_monotonic_rejected() {
        let events = vec![event(2000, 100), event(1000, 100)];
        assert_eq!(
            validate_output_rate(&events),
            Err(OutputRateError::TimestampsNotMonotonic)
        );
    }

    /// **AORL-07** — good events accepted.
    #[test]
    fn aorl_07_good_accepted() {
        assert_eq!(validate_output_rate(&good_events()), Ok(()));
    }

    /// **AORL-08** — single event accepted.
    #[test]
    fn aorl_08_single_accepted() {
        assert_eq!(validate_output_rate(&[event(1000, 100)]), Ok(()));
    }

    /// **AORL-09** — empty accepted.
    #[test]
    fn aorl_09_empty_accepted() {
        assert_eq!(validate_output_rate(&[]), Ok(()));
    }

    /// **AORL-10** — exact min interval accepted.
    #[test]
    fn aorl_10_exact_interval_accepted() {
        let events = vec![event(1000, 100), event(1000 + AORL_MIN_INTERVAL_MS, 100)];
        assert_eq!(validate_output_rate(&events), Ok(()));
    }
}
