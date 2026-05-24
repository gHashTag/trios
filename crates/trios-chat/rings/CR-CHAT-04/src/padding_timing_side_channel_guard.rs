//! # CR-CHAT-04 — Padding timing side-channel guard (Wave-123 Lane A)
//!
//! PADDING — padding operations must complete in constant time; variable
//! timing leaks which messages are real vs padded.
//!
//! When a system pads messages, the time taken to pad must not vary
//! based on whether the message is real or cover traffic:
//!
//! * **Timing fingerprint** — if padding a real message takes
//!   measurably different time than padding cover traffic, the
//!   observer distinguishes them via latency.
//! * **Cache timing** — variable-time padding routines have
//!   data-dependent memory access patterns exploitable via cache
//!   side channels.
//! * **Network timing** — variable server-side processing time
//!   propagates to network latency, observable by a middlebox.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Duration must be >= `PTSC_MIN_DURATION_US`.
//! 2. Duration must be <= `PTSC_MAX_DURATION_US`.
//! 3. Duration must not be zero.
//! 4. Duration variance across batch <= `PTSC_MAX_VARIANCE_US`.
//! 5. No duplicate operation IDs.
//! 6. Total operations <= `PTSC_MAX_OPS`.
//!
//! Tests **PTSC-01..10**. Error enum [`TimingSideChannelError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONSTANT-TIME`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum allowed duration in microseconds.
pub const PTSC_MIN_DURATION_US: u64 = 100;

/// Maximum allowed duration in microseconds.
pub const PTSC_MAX_DURATION_US: u64 = 10_000;

/// Maximum variance across the batch in microseconds.
pub const PTSC_MAX_VARIANCE_US: u64 = 1000;

/// Maximum operations per batch.
pub const PTSC_MAX_OPS: usize = 1024;

/// Operation ID length.
pub const PTSC_OP_ID_LEN: usize = 32;

/// A padding timing measurement.
#[derive(Debug, Clone)]
pub struct TimingMeasurement {
    /// Operation identifier.
    pub op_id: [u8; PTSC_OP_ID_LEN],
    /// Duration of the padding operation in microseconds.
    pub duration_us: u64,
}

/// All ways timing side-channel validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TimingSideChannelError {
    /// Duration below minimum.
    TooFast { idx: usize, got: u64, min: u64 },
    /// Duration above maximum.
    TooSlow { idx: usize, got: u64, max: u64 },
    /// Zero duration.
    ZeroDuration(usize),
    /// Variance too high across batch.
    HighVariance { variance: u64, max: u64 },
    /// Duplicate operation ID.
    DuplicateOpId { idx: usize },
    /// Too many operations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding timing side-channel resistance.
pub fn validate_timing_constancy(
    measurements: &[TimingMeasurement],
) -> Result<(), TimingSideChannelError> {
    if measurements.len() > PTSC_MAX_OPS {
        return Err(TimingSideChannelError::TooMany {
            got: measurements.len(),
            max: PTSC_MAX_OPS,
        });
    }
    let mut seen: BTreeSet<[u8; PTSC_OP_ID_LEN]> = BTreeSet::new();
    let mut min_dur: u64 = u64::MAX;
    let mut max_dur: u64 = 0;
    for (i, m) in measurements.iter().enumerate() {
        if m.duration_us == 0 {
            return Err(TimingSideChannelError::ZeroDuration(i));
        }
        if m.duration_us < PTSC_MIN_DURATION_US {
            return Err(TimingSideChannelError::TooFast {
                idx: i,
                got: m.duration_us,
                min: PTSC_MIN_DURATION_US,
            });
        }
        if m.duration_us > PTSC_MAX_DURATION_US {
            return Err(TimingSideChannelError::TooSlow {
                idx: i,
                got: m.duration_us,
                max: PTSC_MAX_DURATION_US,
            });
        }
        if !seen.insert(m.op_id) {
            return Err(TimingSideChannelError::DuplicateOpId { idx: i });
        }
        min_dur = min_dur.min(m.duration_us);
        max_dur = max_dur.max(m.duration_us);
    }
    if measurements.len() >= 2 {
        let variance = max_dur.saturating_sub(min_dur);
        if variance > PTSC_MAX_VARIANCE_US {
            return Err(TimingSideChannelError::HighVariance {
                variance,
                max: PTSC_MAX_VARIANCE_US,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; PTSC_OP_ID_LEN] {
        [byte; PTSC_OP_ID_LEN]
    }

    fn meas(id: u8, dur: u64) -> TimingMeasurement {
        TimingMeasurement { op_id: oid(id), duration_us: dur }
    }

    fn valid_batch() -> Vec<TimingMeasurement> {
        vec![
            meas(0x01, 500),
            meas(0x02, 550),
            meas(0x03, 480),
            meas(0x04, 520),
        ]
    }

    /// **PTSC-01** — too fast rejected.
    #[test]
    fn ptsc_01_too_fast_rejected() {
        let ms = vec![meas(0x01, PTSC_MIN_DURATION_US - 1)];
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::TooFast {
                idx: 0,
                got: PTSC_MIN_DURATION_US - 1,
                min: PTSC_MIN_DURATION_US,
            })
        );
    }

    /// **PTSC-02** — too slow rejected.
    #[test]
    fn ptsc_02_too_slow_rejected() {
        let ms = vec![meas(0x01, PTSC_MAX_DURATION_US + 1)];
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::TooSlow {
                idx: 0,
                got: PTSC_MAX_DURATION_US + 1,
                max: PTSC_MAX_DURATION_US,
            })
        );
    }

    /// **PTSC-03** — zero duration rejected.
    #[test]
    fn ptsc_03_zero_duration_rejected() {
        let ms = vec![meas(0x01, 0)];
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::ZeroDuration(0))
        );
    }

    /// **PTSC-04** — high variance rejected.
    #[test]
    fn ptsc_04_high_variance_rejected() {
        let ms = vec![
            meas(0x01, 500),
            meas(0x02, 500 + PTSC_MAX_VARIANCE_US + 1),
        ];
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::HighVariance {
                variance: PTSC_MAX_VARIANCE_US + 1,
                max: PTSC_MAX_VARIANCE_US,
            })
        );
    }

    /// **PTSC-05** — duplicate op ID rejected.
    #[test]
    fn ptsc_05_duplicate_rejected() {
        let ms = vec![
            meas(0x01, 500),
            meas(0x01, 500),
        ];
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::DuplicateOpId { idx: 1 })
        );
    }

    /// **PTSC-06** — too many rejected.
    #[test]
    fn ptsc_06_too_many_rejected() {
        let ms: Vec<TimingMeasurement> = (0..=PTSC_MAX_OPS)
            .map(|i| {
                let mut id = [0u8; PTSC_OP_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                TimingMeasurement { op_id: id, duration_us: 500 }
            })
            .collect();
        assert_eq!(
            validate_timing_constancy(&ms),
            Err(TimingSideChannelError::TooMany {
                got: PTSC_MAX_OPS + 1,
                max: PTSC_MAX_OPS,
            })
        );
    }

    /// **PTSC-07** — valid accepted.
    #[test]
    fn ptsc_07_valid_accepted() {
        assert_eq!(validate_timing_constancy(&valid_batch()), Ok(()));
    }

    /// **PTSC-08** — empty accepted.
    #[test]
    fn ptsc_08_empty_accepted() {
        assert_eq!(validate_timing_constancy(&[]), Ok(()));
    }

    /// **PTSC-09** — single accepted (no variance check).
    #[test]
    fn ptsc_09_single_accepted() {
        let ms = vec![meas(0x01, 500)];
        assert_eq!(validate_timing_constancy(&ms), Ok(()));
    }

    /// **PTSC-10** — boundary variance accepted.
    #[test]
    fn ptsc_10_boundary_variance_accepted() {
        let ms = vec![
            meas(0x01, 500),
            meas(0x02, 500 + PTSC_MAX_VARIANCE_US),
        ];
        assert_eq!(validate_timing_constancy(&ms), Ok(()));
    }
}
