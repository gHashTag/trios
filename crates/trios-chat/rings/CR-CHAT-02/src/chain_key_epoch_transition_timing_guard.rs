//! # CR-CHAT-02 — Chain key epoch transition timing guard (Wave-157 Lane A)
//!
//! RATCHET — chain key epoch transitions must happen within a maximum
//! time window; delayed transitions expose messages to compromise.
//!
//! In the Double Ratchet, epoch transitions happen when a DH step
//! occurs. If transitions are delayed beyond the allowed window:
//!
//! * **Compromise window** — messages encrypted under an old epoch
//!   are at risk if the chain key is compromised.
//! * **Forward secrecy gap** — delayed transitions extend the
//!   period where compromise affects multiple messages.
//! * **Replay vulnerability** — old epoch keys remain valid longer,
//!   increasing the replay attack surface.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Transition delay <= `CKETT_MAX_DELAY_MS`.
//! 2. Epoch must be strictly increasing.
//! 3. Transition ID must not be zero.
//! 4. No duplicate transition IDs.
//! 5. Timestamp must be > 0.
//! 6. Batch size <= `CKETT_MAX_TRANSITIONS`.
//!
//! Tests **CKETT-01..10**. Error enum [`TransitionTimingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-TIMELY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum transition delay in milliseconds (30 minutes).
pub const CKETT_MAX_DELAY_MS: u64 = 1_800_000;

/// Maximum transitions per batch.
pub const CKETT_MAX_TRANSITIONS: usize = 512;

/// Transition ID length.
pub const CKETT_TRANSITION_ID_LEN: usize = 16;

/// An epoch transition timing record.
#[derive(Debug, Clone)]
pub struct TransitionTimingRecord {
    /// Transition identifier.
    pub transition_id: [u8; CKETT_TRANSITION_ID_LEN],
    /// Epoch number.
    pub epoch: u64,
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

/// All ways transition timing validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionTimingError {
    /// Delay exceeds maximum.
    DelayTooLarge {
        idx: usize,
        delay_ms: u64,
        max_ms: u64,
    },
    /// Epoch not strictly increasing.
    NonMonotonicEpoch {
        idx: usize,
        prev: u64,
        got: u64,
    },
    /// Zero transition ID.
    ZeroTransitionId(usize),
    /// Duplicate transition ID.
    DuplicateTransitionId {
        idx: usize,
    },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Too many transitions.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate chain key epoch transition timing.
pub fn validate_transition_timing(
    transitions: &[TransitionTimingRecord],
) -> Result<(), TransitionTimingError> {
    if transitions.len() > CKETT_MAX_TRANSITIONS {
        return Err(TransitionTimingError::TooMany {
            got: transitions.len(),
            max: CKETT_MAX_TRANSITIONS,
        });
    }
    let mut seen: BTreeSet<[u8; CKETT_TRANSITION_ID_LEN]> = BTreeSet::new();
    let mut prev_epoch: Option<u64> = None;
    let mut prev_ts: Option<u64> = None;
    for (i, t) in transitions.iter().enumerate() {
        if t.transition_id == [0u8; CKETT_TRANSITION_ID_LEN] {
            return Err(TransitionTimingError::ZeroTransitionId(i));
        }
        if !seen.insert(t.transition_id) {
            return Err(TransitionTimingError::DuplicateTransitionId { idx: i });
        }
        if t.timestamp_ms == 0 {
            return Err(TransitionTimingError::ZeroTimestamp(i));
        }
        if let Some(pe) = prev_epoch {
            if t.epoch <= pe {
                return Err(TransitionTimingError::NonMonotonicEpoch {
                    idx: i,
                    prev: pe,
                    got: t.epoch,
                });
            }
        }
        if let Some(pt) = prev_ts {
            if t.timestamp_ms < pt {
                return Err(TransitionTimingError::DelayTooLarge {
                    idx: i,
                    delay_ms: u64::MAX,
                    max_ms: CKETT_MAX_DELAY_MS,
                });
            }
            let delay = t.timestamp_ms - pt;
            if delay > CKETT_MAX_DELAY_MS {
                return Err(TransitionTimingError::DelayTooLarge {
                    idx: i,
                    delay_ms: delay,
                    max_ms: CKETT_MAX_DELAY_MS,
                });
            }
        }
        prev_epoch = Some(t.epoch);
        prev_ts = Some(t.timestamp_ms);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tid(byte: u8) -> [u8; CKETT_TRANSITION_ID_LEN] {
        [byte; CKETT_TRANSITION_ID_LEN]
    }

    fn trans(id: u8, epoch: u64, ts: u64) -> TransitionTimingRecord {
        TransitionTimingRecord { transition_id: tid(id), epoch, timestamp_ms: ts }
    }

    fn valid_transitions() -> Vec<TransitionTimingRecord> {
        vec![
            trans(0x01, 1, 1_000_000),
            trans(0x02, 2, 1_200_000),
            trans(0x03, 3, 1_500_000),
        ]
    }

    /// **CKETT-01** — delay too large rejected.
    #[test]
    fn ckett_01_delay_too_large_rejected() {
        let ts = vec![
            trans(0x01, 1, 1_000_000),
            trans(0x02, 2, 1_000_000 + CKETT_MAX_DELAY_MS + 1),
        ];
        assert_eq!(
            validate_transition_timing(&ts),
            Err(TransitionTimingError::DelayTooLarge {
                idx: 1,
                delay_ms: CKETT_MAX_DELAY_MS + 1,
                max_ms: CKETT_MAX_DELAY_MS,
            })
        );
    }

    /// **CKETT-02** — non-monotonic epoch rejected.
    #[test]
    fn ckett_02_non_monotonic_rejected() {
        let ts = vec![
            trans(0x01, 2, 1_000_000),
            trans(0x02, 1, 1_200_000),
        ];
        assert_eq!(
            validate_transition_timing(&ts),
            Err(TransitionTimingError::NonMonotonicEpoch { idx: 1, prev: 2, got: 1 })
        );
    }

    /// **CKETT-03** — zero transition ID rejected.
    #[test]
    fn ckett_03_zero_id_rejected() {
        let t = TransitionTimingRecord { transition_id: [0u8; CKETT_TRANSITION_ID_LEN], epoch: 1, timestamp_ms: 1_000_000 };
        assert_eq!(
            validate_transition_timing(&[t]),
            Err(TransitionTimingError::ZeroTransitionId(0))
        );
    }

    /// **CKETT-04** — duplicate transition ID rejected.
    #[test]
    fn ckett_04_duplicate_rejected() {
        let ts = vec![
            trans(0x01, 1, 1_000_000),
            trans(0x01, 2, 1_200_000),
        ];
        assert_eq!(
            validate_transition_timing(&ts),
            Err(TransitionTimingError::DuplicateTransitionId { idx: 1 })
        );
    }

    /// **CKETT-05** — zero timestamp rejected.
    #[test]
    fn ckett_05_zero_ts_rejected() {
        let t = TransitionTimingRecord { transition_id: tid(0x01), epoch: 1, timestamp_ms: 0 };
        assert_eq!(
            validate_transition_timing(&[t]),
            Err(TransitionTimingError::ZeroTimestamp(0))
        );
    }

    /// **CKETT-06** — too many rejected.
    #[test]
    fn ckett_06_too_many_rejected() {
        let ts: Vec<TransitionTimingRecord> = (0..=CKETT_MAX_TRANSITIONS)
            .map(|i| {
                let mut id = [0u8; CKETT_TRANSITION_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                TransitionTimingRecord { transition_id: id, epoch: val, timestamp_ms: val * 100_000 }
            })
            .collect();
        assert_eq!(
            validate_transition_timing(&ts),
            Err(TransitionTimingError::TooMany {
                got: CKETT_MAX_TRANSITIONS + 1,
                max: CKETT_MAX_TRANSITIONS,
            })
        );
    }

    /// **CKETT-07** — valid accepted.
    #[test]
    fn ckett_07_valid_accepted() {
        assert_eq!(validate_transition_timing(&valid_transitions()), Ok(()));
    }

    /// **CKETT-08** — empty accepted.
    #[test]
    fn ckett_08_empty_accepted() {
        assert_eq!(validate_transition_timing(&[]), Ok(()));
    }

    /// **CKETT-09** — boundary delay accepted.
    #[test]
    fn ckett_09_boundary_delay_accepted() {
        let ts = vec![
            trans(0x01, 1, 1_000_000),
            trans(0x02, 2, 1_000_000 + CKETT_MAX_DELAY_MS),
        ];
        assert_eq!(validate_transition_timing(&ts), Ok(()));
    }

    /// **CKETT-10** — many timely accepted.
    #[test]
    fn ckett_10_many_timely_accepted() {
        let ts: Vec<TransitionTimingRecord> = (0..20u8)
            .map(|i| trans(i + 1, (i as u64) + 1, 1_000_000 + (i as u64) * 50_000))
            .collect();
        assert_eq!(validate_transition_timing(&ts), Ok(()));
    }
}
