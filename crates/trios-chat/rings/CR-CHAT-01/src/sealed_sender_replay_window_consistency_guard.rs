//! # CR-CHAT-01 — Sealed sender replay window consistency guard (Wave-125 Lane A)
//!
//! IDENTITY — sealed sender ephemeral keys must not replay within the
//! detection window; replayed keys enable linkability attacks.
//!
//! Each sealed sender envelope uses a fresh ephemeral key. If the same
//! ephemeral public key appears in multiple envelopes:
//!
//! * **Linkability** — the observer links envelopes that share an
//!   ephemeral key, breaking sender anonymity.
//! * **Key recovery** — reusing ephemeral keys with different
//!   recipient keys can leak the static secret via lattice attacks.
//! * **Window consistency** — the replay detection window must be
//!   consistent; gaps in the window allow replay across boundaries.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate ephemeral keys within the window.
//! 2. Ephemeral key must not be zero.
//! 3. Window size must be <= `SSRW_MAX_WINDOW`.
//! 4. Timestamp must be > 0.
//! 5. Timestamps must be non-decreasing within window.
//! 6. Total entries <= `SSRW_MAX_ENTRIES`.
//!
//! Tests **SSRW-01..10**. Error enum [`ReplayWindowError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * REPLAY-FREE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum window size.
pub const SSRW_MAX_WINDOW: usize = 1024;

/// Maximum entries per batch.
pub const SSRW_MAX_ENTRIES: usize = 2048;

/// Ephemeral key length.
pub const SSRW_KEY_LEN: usize = 32;

/// A sealed sender entry in the replay window.
#[derive(Debug, Clone)]
pub struct ReplayEntry {
    /// Ephemeral public key.
    pub ephemeral_key: [u8; SSRW_KEY_LEN],
    /// Timestamp of the envelope.
    pub timestamp_ms: u64,
}

/// All ways replay window validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayWindowError {
    /// Duplicate ephemeral key in window.
    DuplicateKey { idx: usize },
    /// Zero ephemeral key.
    ZeroKey(usize),
    /// Window size exceeds maximum.
    WindowTooLarge { got: usize, max: usize },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too many entries.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate sealed sender replay window consistency.
pub fn validate_replay_window(
    entries: &[ReplayEntry],
) -> Result<(), ReplayWindowError> {
    if entries.len() > SSRW_MAX_ENTRIES {
        return Err(ReplayWindowError::TooMany {
            got: entries.len(),
            max: SSRW_MAX_ENTRIES,
        });
    }
    if entries.len() > SSRW_MAX_WINDOW {
        return Err(ReplayWindowError::WindowTooLarge {
            got: entries.len(),
            max: SSRW_MAX_WINDOW,
        });
    }
    let mut seen: BTreeSet<[u8; SSRW_KEY_LEN]> = BTreeSet::new();
    let mut prev_ts: u64 = 0;
    for (i, e) in entries.iter().enumerate() {
        if e.ephemeral_key == [0u8; SSRW_KEY_LEN] {
            return Err(ReplayWindowError::ZeroKey(i));
        }
        if e.timestamp_ms == 0 {
            return Err(ReplayWindowError::ZeroTimestamp(i));
        }
        if i > 0 && e.timestamp_ms < prev_ts {
            return Err(ReplayWindowError::NonMonotonic {
                idx: i,
                prev: prev_ts,
                current: e.timestamp_ms,
            });
        }
        if !seen.insert(e.ephemeral_key) {
            return Err(ReplayWindowError::DuplicateKey { idx: i });
        }
        prev_ts = e.timestamp_ms;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; SSRW_KEY_LEN] {
        [byte; SSRW_KEY_LEN]
    }

    fn entry(k: u8, ts: u64) -> ReplayEntry {
        ReplayEntry { ephemeral_key: key(k), timestamp_ms: ts }
    }

    fn valid_window() -> Vec<ReplayEntry> {
        vec![
            entry(0x01, 1000),
            entry(0x02, 2000),
            entry(0x03, 3000),
        ]
    }

    /// **SSRW-01** — duplicate key rejected.
    #[test]
    fn ssrw_01_duplicate_key_rejected() {
        let es = vec![
            entry(0xAA, 1000),
            entry(0xAA, 2000),
        ];
        assert_eq!(
            validate_replay_window(&es),
            Err(ReplayWindowError::DuplicateKey { idx: 1 })
        );
    }

    /// **SSRW-02** — zero key rejected.
    #[test]
    fn ssrw_02_zero_key_rejected() {
        let e = ReplayEntry { ephemeral_key: [0u8; SSRW_KEY_LEN], timestamp_ms: 1000 };
        assert_eq!(
            validate_replay_window(&[e]),
            Err(ReplayWindowError::ZeroKey(0))
        );
    }

    /// **SSRW-03** — window too large rejected.
    #[test]
    fn ssrw_03_window_too_large_rejected() {
        let es: Vec<ReplayEntry> = (0..=SSRW_MAX_WINDOW)
            .map(|i| {
                let mut k = [0u8; SSRW_KEY_LEN];
                let val = (i as u64) + 1;
                k[0..8].copy_from_slice(&val.to_be_bytes());
                ReplayEntry { ephemeral_key: k, timestamp_ms: (i as u64) + 1 }
            })
            .collect();
        assert_eq!(
            validate_replay_window(&es),
            Err(ReplayWindowError::WindowTooLarge {
                got: SSRW_MAX_WINDOW + 1,
                max: SSRW_MAX_WINDOW,
            })
        );
    }

    /// **SSRW-04** — zero timestamp rejected.
    #[test]
    fn ssrw_04_zero_timestamp_rejected() {
        let e = ReplayEntry { ephemeral_key: key(0x01), timestamp_ms: 0 };
        assert_eq!(
            validate_replay_window(&[e]),
            Err(ReplayWindowError::ZeroTimestamp(0))
        );
    }

    /// **SSRW-05** — non-monotonic rejected.
    #[test]
    fn ssrw_05_non_monotonic_rejected() {
        let es = vec![
            entry(0x01, 2000),
            entry(0x02, 1000),
        ];
        assert_eq!(
            validate_replay_window(&es),
            Err(ReplayWindowError::NonMonotonic { idx: 1, prev: 2000, current: 1000 })
        );
    }

    /// **SSRW-06** — too many rejected.
    #[test]
    fn ssrw_06_too_many_rejected() {
        let es: Vec<ReplayEntry> = (0..=SSRW_MAX_ENTRIES)
            .map(|i| {
                let mut k = [0u8; SSRW_KEY_LEN];
                let val = (i as u64) + 1;
                k[0..8].copy_from_slice(&val.to_be_bytes());
                ReplayEntry { ephemeral_key: k, timestamp_ms: (i as u64) + 1 }
            })
            .collect();
        assert_eq!(
            validate_replay_window(&es),
            Err(ReplayWindowError::TooMany {
                got: SSRW_MAX_ENTRIES + 1,
                max: SSRW_MAX_ENTRIES,
            })
        );
    }

    /// **SSRW-07** — valid accepted.
    #[test]
    fn ssrw_07_valid_accepted() {
        assert_eq!(validate_replay_window(&valid_window()), Ok(()));
    }

    /// **SSRW-08** — empty accepted.
    #[test]
    fn ssrw_08_empty_accepted() {
        assert_eq!(validate_replay_window(&[]), Ok(()));
    }

    /// **SSRW-09** — single accepted.
    #[test]
    fn ssrw_09_single_accepted() {
        assert_eq!(validate_replay_window(&[entry(0x01, 1000)]), Ok(()));
    }

    /// **SSRW-10** — equal timestamps accepted (non-decreasing).
    #[test]
    fn ssrw_10_equal_timestamps_accepted() {
        let es = vec![
            entry(0x01, 1000),
            entry(0x02, 1000),
        ];
        assert_eq!(validate_replay_window(&es), Ok(()));
    }
}
