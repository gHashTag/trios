//! # CR-CHAT-07 — Replay detection window guard (Wave-76 Lane A)
//!
//! ANTI-CORRELATION — replay detector window must be correctly sized, R-CHAT-10.
//!
//! The replay detector maintains a sliding window of seen message IDs.
//! If the window is misconfigured:
//!
//! * **Window too small** — legitimately delayed packets arrive outside
//!   the window and are falsely rejected as replays.
//! * **Window too large** — an attacker can replay very old messages,
//!   and memory grows unbounded.
//! * **Window gap** — a gap in the sequence leaves holes that an
//!   attacker exploits to inject replayed messages.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Window size >= `RPDW_MIN_WINDOW`.
//! 2. Window size <= `RPDW_MAX_WINDOW`.
//! 3. All message IDs within the window are unique.
//! 4. No gaps in the sequence (contiguous range).
//! 5. Newest ID - oldest ID <= window size.
//! 6. Message count <= `RPDW_MAX_MESSAGES`.
//!
//! Tests **RPDW-01..10**. Error enum [`ReplayWindowError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * REPLAY-DETECTION-WINDOW`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum window size (messages).
pub const RPDW_MIN_WINDOW: u64 = 16;

/// Maximum window size (messages).
pub const RPDW_MAX_WINDOW: u64 = 4096;

/// Maximum messages in a batch.
pub const RPDW_MAX_MESSAGES: usize = 8192;

/// All ways replay window validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayWindowError {
    /// Window too small.
    WindowTooSmall,
    /// Window too large.
    WindowTooLarge,
    /// Duplicate message ID in window.
    DuplicateMessage(u64),
    /// Gap in sequence.
    GapInSequence,
    /// Span exceeds window.
    SpanExceedsWindow,
    /// Too many messages.
    TooManyMessages,
}

/// `[VERIFIED]` Validate replay detection window configuration and contents.
pub fn validate_replay_window(
    window_size: u64,
    message_ids: &[u64],
) -> Result<(), ReplayWindowError> {
    if window_size < RPDW_MIN_WINDOW {
        return Err(ReplayWindowError::WindowTooSmall);
    }
    if window_size > RPDW_MAX_WINDOW {
        return Err(ReplayWindowError::WindowTooLarge);
    }
    if message_ids.len() > RPDW_MAX_MESSAGES {
        return Err(ReplayWindowError::TooManyMessages);
    }
    let mut seen = BTreeSet::new();
    for &id in message_ids {
        if !seen.insert(id) {
            return Err(ReplayWindowError::DuplicateMessage(id));
        }
    }
    if !message_ids.is_empty() {
        let mut sorted = message_ids.to_vec();
        sorted.sort();
        for w in sorted.windows(2) {
            if w[1] - w[0] > 1 {
                return Err(ReplayWindowError::GapInSequence);
            }
        }
        let span = sorted[sorted.len() - 1] - sorted[0] + 1;
        if span > window_size {
            return Err(ReplayWindowError::SpanExceedsWindow);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **RPDW-01** — window too small rejected.
    #[test]
    fn rpdw_01_window_small_rejected() {
        assert_eq!(
            validate_replay_window(8, &[1, 2, 3]),
            Err(ReplayWindowError::WindowTooSmall)
        );
    }

    /// **RPDW-02** — window too large rejected.
    #[test]
    fn rpdw_02_window_large_rejected() {
        assert_eq!(
            validate_replay_window(RPDW_MAX_WINDOW + 1, &[1]),
            Err(ReplayWindowError::WindowTooLarge)
        );
    }

    /// **RPDW-03** — duplicate message rejected.
    #[test]
    fn rpdw_03_duplicate_rejected() {
        assert_eq!(
            validate_replay_window(RPDW_MIN_WINDOW, &[1, 2, 3, 2]),
            Err(ReplayWindowError::DuplicateMessage(2))
        );
    }

    /// **RPDW-04** — gap in sequence rejected.
    #[test]
    fn rpdw_04_gap_rejected() {
        assert_eq!(
            validate_replay_window(RPDW_MIN_WINDOW, &[1, 2, 5, 6]),
            Err(ReplayWindowError::GapInSequence)
        );
    }

    /// **RPDW-05** — span exceeds window rejected.
    #[test]
    fn rpdw_05_span_exceeds_rejected() {
        let ids: Vec<u64> = (1..=RPDW_MIN_WINDOW + 1).collect();
        assert_eq!(
            validate_replay_window(RPDW_MIN_WINDOW, &ids),
            Err(ReplayWindowError::SpanExceedsWindow)
        );
    }

    /// **RPDW-06** — too many messages rejected.
    #[test]
    fn rpdw_06_too_many_rejected() {
        let ids: Vec<u64> = (0..=RPDW_MAX_MESSAGES as u64).collect();
        assert_eq!(
            validate_replay_window(RPDW_MAX_WINDOW, &ids),
            Err(ReplayWindowError::TooManyMessages)
        );
    }

    /// **RPDW-07** — valid window accepted.
    #[test]
    fn rpdw_07_valid_accepted() {
        let ids: Vec<u64> = (1..=10).collect();
        assert_eq!(validate_replay_window(RPDW_MIN_WINDOW, &ids), Ok(()));
    }

    /// **RPDW-08** — empty accepted.
    #[test]
    fn rpdw_08_empty_accepted() {
        assert_eq!(validate_replay_window(RPDW_MIN_WINDOW, &[]), Ok(()));
    }

    /// **RPDW-09** — single message accepted.
    #[test]
    fn rpdw_09_single_accepted() {
        assert_eq!(validate_replay_window(RPDW_MIN_WINDOW, &[42]), Ok(()));
    }

    /// **RPDW-10** — max window accepted.
    #[test]
    fn rpdw_10_max_window_accepted() {
        let ids: Vec<u64> = (0..RPDW_MAX_WINDOW).collect();
        assert_eq!(validate_replay_window(RPDW_MAX_WINDOW, &ids), Ok(()));
    }
}
