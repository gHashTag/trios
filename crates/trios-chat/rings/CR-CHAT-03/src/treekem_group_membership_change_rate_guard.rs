//! # CR-CHAT-03 — TreeKEM group membership change rate guard (Wave-115 Lane B)
//!
//! RATCHET TREE — membership changes must be rate-limited.
//!
//! Group membership changes (joins, leaves, updates) trigger TreeKEM
//! epoch transitions. If changes arrive too rapidly:
//!
//! * **Tree instability** — members cannot process epoch N before
//!   epoch N+1 arrives, causing state desynchronization.
//! * **DoS amplification** — each change requires all members to
//!   recompute the tree, so rapid changes multiply CPU cost.
//! * **State fork** — concurrent changes processed in different
//!   orders by different members produce divergent tree states.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Changes per window <= `GMCR_MAX_CHANGES`.
//! 2. Window duration >= `GMCR_MIN_WINDOW_MS`.
//! 3. Change type must be valid.
//! 4. Epoch must be strictly increasing.
//! 5. Epoch must not be zero.
//! 6. Total windows <= `GMCR_MAX_WINDOWS`.
//!
//! Tests **GMCR-01..10**. Error enum [`MembershipRateError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MEMBERSHIP-RATE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum membership changes per window.
pub const GMCR_MAX_CHANGES: usize = 20;

/// Minimum window duration (ms).
pub const GMCR_MIN_WINDOW_MS: u64 = 10_000;

/// Maximum windows per batch.
pub const GMCR_MAX_WINDOWS: usize = 256;

/// Change type codes.
pub const CHANGE_JOIN: u8 = 1;
pub const CHANGE_LEAVE: u8 = 2;
pub const CHANGE_UPDATE: u8 = 3;

/// Valid change types.
pub const GMCR_VALID_CHANGES: [u8; 3] = [CHANGE_JOIN, CHANGE_LEAVE, CHANGE_UPDATE];

/// A membership change window.
#[derive(Debug, Clone)]
pub struct MembershipWindow {
    /// Epoch number.
    pub epoch: u64,
    /// Window start time (ms since epoch).
    pub window_start: u64,
    /// Window duration (ms).
    pub window_duration: u64,
    /// Number of membership changes in this window.
    pub change_count: usize,
    /// Change types observed.
    pub change_types: Vec<u8>,
}

/// All ways membership rate validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MembershipRateError {
    /// Too many changes in window.
    TooManyChanges { idx: usize, count: usize, max: usize },
    /// Window too short.
    WindowTooShort { idx: usize, duration: u64, min: u64 },
    /// Invalid change type.
    InvalidChangeType { idx: usize, change_type: u8 },
    /// Not increasing epoch.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Too many windows.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate group membership change rate.
pub fn validate_membership_rate(
    windows: &[MembershipWindow],
) -> Result<(), MembershipRateError> {
    if windows.len() > GMCR_MAX_WINDOWS {
        return Err(MembershipRateError::TooMany {
            got: windows.len(),
            max: GMCR_MAX_WINDOWS,
        });
    }
    let mut prev_epoch: u64 = 0;
    for (i, w) in windows.iter().enumerate() {
        if w.epoch == 0 {
            return Err(MembershipRateError::ZeroEpoch(i));
        }
        if i > 0 && w.epoch <= prev_epoch {
            return Err(MembershipRateError::NotIncreasing {
                idx: i,
                prev: prev_epoch,
                current: w.epoch,
            });
        }
        if w.window_duration < GMCR_MIN_WINDOW_MS {
            return Err(MembershipRateError::WindowTooShort {
                idx: i,
                duration: w.window_duration,
                min: GMCR_MIN_WINDOW_MS,
            });
        }
        if w.change_count > GMCR_MAX_CHANGES {
            return Err(MembershipRateError::TooManyChanges {
                idx: i,
                count: w.change_count,
                max: GMCR_MAX_CHANGES,
            });
        }
        for &ct in &w.change_types {
            if !GMCR_VALID_CHANGES.contains(&ct) {
                return Err(MembershipRateError::InvalidChangeType {
                    idx: i,
                    change_type: ct,
                });
            }
        }
        prev_epoch = w.epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(epoch: u64, duration: u64, count: usize, types: Vec<u8>) -> MembershipWindow {
        MembershipWindow { epoch, window_start: 0, window_duration: duration, change_count: count, change_types: types }
    }

    fn valid_windows() -> Vec<MembershipWindow> {
        vec![
            window(1, 30_000, 5, vec![CHANGE_JOIN, CHANGE_JOIN, CHANGE_LEAVE, CHANGE_UPDATE, CHANGE_JOIN]),
            window(2, 60_000, 3, vec![CHANGE_LEAVE, CHANGE_UPDATE, CHANGE_JOIN]),
        ]
    }

    /// **GMCR-01** — too many changes rejected.
    #[test]
    fn gmcr_01_too_many_rejected() {
        let w = window(1, 30_000, GMCR_MAX_CHANGES + 1, vec![CHANGE_JOIN; GMCR_MAX_CHANGES + 1]);
        assert_eq!(
            validate_membership_rate(&[w]),
            Err(MembershipRateError::TooManyChanges {
                idx: 0,
                count: GMCR_MAX_CHANGES + 1,
                max: GMCR_MAX_CHANGES,
            })
        );
    }

    /// **GMCR-02** — window too short rejected.
    #[test]
    fn gmcr_02_window_too_short_rejected() {
        let w = window(1, 1000, 1, vec![CHANGE_JOIN]);
        assert_eq!(
            validate_membership_rate(&[w]),
            Err(MembershipRateError::WindowTooShort {
                idx: 0,
                duration: 1000,
                min: GMCR_MIN_WINDOW_MS,
            })
        );
    }

    /// **GMCR-03** — invalid change type rejected.
    #[test]
    fn gmcr_03_invalid_type_rejected() {
        let w = window(1, 30_000, 2, vec![CHANGE_JOIN, 99]);
        assert_eq!(
            validate_membership_rate(&[w]),
            Err(MembershipRateError::InvalidChangeType {
                idx: 0,
                change_type: 99,
            })
        );
    }

    /// **GMCR-04** — not increasing rejected.
    #[test]
    fn gmcr_04_not_increasing_rejected() {
        let ws = vec![window(5, 30_000, 1, vec![CHANGE_JOIN]), window(3, 30_000, 1, vec![CHANGE_JOIN])];
        assert_eq!(
            validate_membership_rate(&ws),
            Err(MembershipRateError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **GMCR-05** — zero epoch rejected.
    #[test]
    fn gmcr_05_zero_epoch_rejected() {
        let w = MembershipWindow { epoch: 0, window_start: 0, window_duration: 30_000, change_count: 1, change_types: vec![CHANGE_JOIN] };
        assert_eq!(
            validate_membership_rate(&[w]),
            Err(MembershipRateError::ZeroEpoch(0))
        );
    }

    /// **GMCR-06** — too many windows rejected.
    #[test]
    fn gmcr_06_too_many_rejected() {
        let ws: Vec<MembershipWindow> = (0..=GMCR_MAX_WINDOWS)
            .map(|i| window((i as u64) + 1, 30_000, 1, vec![CHANGE_JOIN]))
            .collect();
        assert_eq!(
            validate_membership_rate(&ws),
            Err(MembershipRateError::TooMany {
                got: GMCR_MAX_WINDOWS + 1,
                max: GMCR_MAX_WINDOWS,
            })
        );
    }

    /// **GMCR-07** — valid accepted.
    #[test]
    fn gmcr_07_valid_accepted() {
        assert_eq!(validate_membership_rate(&valid_windows()), Ok(()));
    }

    /// **GMCR-08** — empty accepted.
    #[test]
    fn gmcr_08_empty_accepted() {
        assert_eq!(validate_membership_rate(&[]), Ok(()));
    }

    /// **GMCR-09** — max changes boundary accepted.
    #[test]
    fn gmcr_09_max_changes_accepted() {
        let w = window(1, 30_000, GMCR_MAX_CHANGES, vec![CHANGE_JOIN; GMCR_MAX_CHANGES]);
        assert_eq!(validate_membership_rate(&[w]), Ok(()));
    }

    /// **GMCR-10** — all change types accepted.
    #[test]
    fn gmcr_10_all_types_accepted() {
        let w = window(1, 30_000, 3, vec![CHANGE_JOIN, CHANGE_LEAVE, CHANGE_UPDATE]);
        assert_eq!(validate_membership_rate(&[w]), Ok(()));
    }
}
