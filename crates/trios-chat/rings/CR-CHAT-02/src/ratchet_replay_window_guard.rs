//! # CR-CHAT-02 — Ratchet replay window guard (Wave-47 Lane A)
//!
//! R-CHAT-2 — Message counter replay protection.
//!
//! Each ratchet session assigns a strictly monotonic counter to every
//! outgoing message. An adversary who can replay or reorder messages
//! can:
//!
//! * **Replay** an old ciphertext to trigger duplicate decryption with
//!   the same key (nonce reuse → AEAD catastrophic failure).
//! * **Skip** counters to force the recipient to derive unused keys,
//!   consuming CPU and potentially causing key material loss.
//! * **Fork** the ratchet by delivering different counter values to
//!   different recipients.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Counter is strictly monotonic (new > last seen).
//! 2. Gap between consecutive counters ≤ `RPL_MAX_GAP`.
//! 3. Counter never exceeds `RPL_MAX_COUNTER`.
//! 4. Replay window rejects duplicates within the sliding window.
//! 5. Counter must be non-zero.
//! 6. Window size is bounded by `RPL_MAX_WINDOW`.
//!
//! Tests **RPLA-01..10**. Error enum [`RatchetReplayError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · RATCHET-REPLAY`

#![forbid(unsafe_code)]

/// Maximum allowed gap between consecutive counters.
pub const RPL_MAX_GAP: u64 = 64;

/// Maximum counter value.
pub const RPL_MAX_COUNTER: u64 = (1u64 << 40) - 1;

/// Maximum replay window size.
pub const RPL_MAX_WINDOW: usize = 128;

/// All ways ratchet replay validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RatchetReplayError {
    /// Counter did not advance.
    NotAdvanced,
    /// Gap exceeds maximum.
    GapTooLarge,
    /// Counter exceeds maximum.
    ExceedsMaxCounter,
    /// Duplicate counter within replay window.
    DuplicateInWindow,
    /// Zero counter not allowed.
    ZeroCounter,
    /// Window size exceeds maximum.
    WindowTooLarge,
}

/// `[VERIFIED]` Validate a single counter against the last-seen value.
pub fn validate_counter_advance(
    last_seen: u64,
    proposed: u64,
) -> Result<(), RatchetReplayError> {
    if proposed == 0 {
        return Err(RatchetReplayError::ZeroCounter);
    }
    if proposed > RPL_MAX_COUNTER {
        return Err(RatchetReplayError::ExceedsMaxCounter);
    }
    if proposed <= last_seen {
        return Err(RatchetReplayError::NotAdvanced);
    }
    if proposed - last_seen > RPL_MAX_GAP {
        return Err(RatchetReplayError::GapTooLarge);
    }
    Ok(())
}

/// `[VERIFIED]` Check a counter against a replay window (sliding set of
/// recently seen counters). Returns `Ok(())` if the counter is fresh.
pub fn check_replay_window(
    window: &[u64],
    counter: u64,
) -> Result<(), RatchetReplayError> {
    if counter == 0 {
        return Err(RatchetReplayError::ZeroCounter);
    }
    if counter > RPL_MAX_COUNTER {
        return Err(RatchetReplayError::ExceedsMaxCounter);
    }
    if window.len() > RPL_MAX_WINDOW {
        return Err(RatchetReplayError::WindowTooLarge);
    }
    if window.contains(&counter) {
        return Err(RatchetReplayError::DuplicateInWindow);
    }
    if let Some(&last) = window.last() {
        if counter <= last {
            return Err(RatchetReplayError::NotAdvanced);
        }
        if counter - last > RPL_MAX_GAP {
            return Err(RatchetReplayError::GapTooLarge);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **RPLA-01** — zero counter rejected.
    #[test]
    fn rpla_01_zero_counter_rejected() {
        assert_eq!(
            validate_counter_advance(0, 0),
            Err(RatchetReplayError::ZeroCounter)
        );
    }

    /// **RPLA-02** — counter not advanced rejected.
    #[test]
    fn rpla_02_not_advanced_rejected() {
        assert_eq!(
            validate_counter_advance(5, 5),
            Err(RatchetReplayError::NotAdvanced)
        );
    }

    /// **RPLA-03** — counter rollback rejected.
    #[test]
    fn rpla_03_rollback_rejected() {
        assert_eq!(
            validate_counter_advance(10, 3),
            Err(RatchetReplayError::NotAdvanced)
        );
    }

    /// **RPLA-04** — gap too large rejected.
    #[test]
    fn rpla_04_gap_too_large_rejected() {
        assert_eq!(
            validate_counter_advance(1, 1 + RPL_MAX_GAP + 1),
            Err(RatchetReplayError::GapTooLarge)
        );
    }

    /// **RPLA-05** — exceeds max counter rejected.
    #[test]
    fn rpla_05_exceeds_max_rejected() {
        assert_eq!(
            validate_counter_advance(RPL_MAX_COUNTER - 1, RPL_MAX_COUNTER + 1),
            Err(RatchetReplayError::ExceedsMaxCounter)
        );
    }

    /// **RPLA-06** — valid advance accepted.
    #[test]
    fn rpla_06_valid_advance_accepted() {
        assert_eq!(validate_counter_advance(5, 6), Ok(()));
    }

    /// **RPLA-07** — exact max gap accepted.
    #[test]
    fn rpla_07_exact_max_gap_accepted() {
        assert_eq!(validate_counter_advance(1, 1 + RPL_MAX_GAP), Ok(()));
    }

    /// **RPLA-08** — duplicate in window rejected.
    #[test]
    fn rpla_08_duplicate_in_window_rejected() {
        assert_eq!(
            check_replay_window(&[1, 2, 3, 4, 5], 3),
            Err(RatchetReplayError::DuplicateInWindow)
        );
    }

    /// **RPLA-09** — fresh counter in window accepted.
    #[test]
    fn rpla_09_fresh_counter_accepted() {
        assert_eq!(check_replay_window(&[1, 2, 3], 4), Ok(()));
    }

    /// **RPLA-10** — empty window fresh counter accepted.
    #[test]
    fn rpla_10_empty_window_accepted() {
        assert_eq!(check_replay_window(&[], 1), Ok(()));
    }
}
