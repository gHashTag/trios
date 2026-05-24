//! # CR-CHAT-02 — Root key rotation frequency bound guard (Wave-154 Lane A)
//!
//! RATCHET — root keys must be rotated within a maximum interval;
//! too-infrequent rotations weaken forward secrecy.
//!
//! In the Double Ratchet, root key rotations happen via DH steps. If
//! the interval between rotations is too large:
//!
//! * **Forward secrecy degradation** — more messages are encrypted
//!   under the same root key, increasing exposure if compromised.
//! * **Key lifetime excess** — a root key used beyond its intended
//!   lifetime increases the attack window.
//! * **Post-compromise recovery delay** — infrequent rotations delay
//!   recovery from a key compromise event.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rotation interval <= `RKRF_MAX_INTERVAL_MS`.
//! 2. Epoch must be strictly increasing.
//! 3. Rotation ID must not be zero.
//! 4. No duplicate rotation IDs.
//! 5. Timestamp must be > 0.
//! 6. Batch size <= `RKRF_MAX_ROTATIONS`.
//!
//! Tests **RKRF-01..10**. Error enum [`RotationFrequencyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ROTATION-FREQ`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum interval between rotations in milliseconds (1 hour).
pub const RKRF_MAX_INTERVAL_MS: u64 = 3_600_000;

/// Maximum rotations per batch.
pub const RKRF_MAX_ROTATIONS: usize = 512;

/// Rotation ID length.
pub const RKRF_ROTATION_ID_LEN: usize = 16;

/// A root key rotation record.
#[derive(Debug, Clone)]
pub struct RotationRecord {
    /// Rotation identifier.
    pub rotation_id: [u8; RKRF_ROTATION_ID_LEN],
    /// Epoch number.
    pub epoch: u64,
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
}

/// All ways rotation frequency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RotationFrequencyError {
    /// Interval exceeds maximum.
    IntervalTooLarge {
        idx: usize,
        gap_ms: u64,
        max_ms: u64,
    },
    /// Epoch not strictly increasing.
    NonMonotonicEpoch {
        idx: usize,
        prev: u64,
        got: u64,
    },
    /// Zero rotation ID.
    ZeroRotationId(usize),
    /// Duplicate rotation ID.
    DuplicateRotationId {
        idx: usize,
    },
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Too many rotations.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate root key rotation frequency bound.
pub fn validate_rotation_frequency(
    rotations: &[RotationRecord],
) -> Result<(), RotationFrequencyError> {
    if rotations.len() > RKRF_MAX_ROTATIONS {
        return Err(RotationFrequencyError::TooMany {
            got: rotations.len(),
            max: RKRF_MAX_ROTATIONS,
        });
    }
    let mut seen: BTreeSet<[u8; RKRF_ROTATION_ID_LEN]> = BTreeSet::new();
    let mut prev_epoch: Option<u64> = None;
    let mut prev_ts: Option<u64> = None;
    for (i, r) in rotations.iter().enumerate() {
        if r.rotation_id == [0u8; RKRF_ROTATION_ID_LEN] {
            return Err(RotationFrequencyError::ZeroRotationId(i));
        }
        if !seen.insert(r.rotation_id) {
            return Err(RotationFrequencyError::DuplicateRotationId { idx: i });
        }
        if r.timestamp_ms == 0 {
            return Err(RotationFrequencyError::ZeroTimestamp(i));
        }
        if let Some(pe) = prev_epoch {
            if r.epoch <= pe {
                return Err(RotationFrequencyError::NonMonotonicEpoch {
                    idx: i,
                    prev: pe,
                    got: r.epoch,
                });
            }
        }
        if let Some(pt) = prev_ts {
            let gap = r.timestamp_ms.saturating_sub(pt);
            if gap > RKRF_MAX_INTERVAL_MS {
                return Err(RotationFrequencyError::IntervalTooLarge {
                    idx: i,
                    gap_ms: gap,
                    max_ms: RKRF_MAX_INTERVAL_MS,
                });
            }
        }
        prev_epoch = Some(r.epoch);
        prev_ts = Some(r.timestamp_ms);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; RKRF_ROTATION_ID_LEN] {
        [byte; RKRF_ROTATION_ID_LEN]
    }

    fn rot(id: u8, epoch: u64, ts: u64) -> RotationRecord {
        RotationRecord { rotation_id: rid(id), epoch, timestamp_ms: ts }
    }

    fn valid_rotations() -> Vec<RotationRecord> {
        vec![
            rot(0x01, 1, 1_000_000),
            rot(0x02, 2, 1_600_000),
            rot(0x03, 3, 2_200_000),
        ]
    }

    /// **RKRF-01** — interval too large rejected.
    #[test]
    fn rkrf_01_interval_too_large_rejected() {
        let rs = vec![
            rot(0x01, 1, 1_000_000),
            rot(0x02, 2, 5_000_000),
        ];
        assert_eq!(
            validate_rotation_frequency(&rs),
            Err(RotationFrequencyError::IntervalTooLarge {
                idx: 1,
                gap_ms: 4_000_000,
                max_ms: RKRF_MAX_INTERVAL_MS,
            })
        );
    }

    /// **RKRF-02** — non-monotonic epoch rejected.
    #[test]
    fn rkrf_02_non_monotonic_rejected() {
        let rs = vec![
            rot(0x01, 2, 1_000_000),
            rot(0x02, 1, 1_600_000),
        ];
        assert_eq!(
            validate_rotation_frequency(&rs),
            Err(RotationFrequencyError::NonMonotonicEpoch { idx: 1, prev: 2, got: 1 })
        );
    }

    /// **RKRF-03** — zero rotation ID rejected.
    #[test]
    fn rkrf_03_zero_id_rejected() {
        let r = RotationRecord { rotation_id: [0u8; RKRF_ROTATION_ID_LEN], epoch: 1, timestamp_ms: 1_000_000 };
        assert_eq!(
            validate_rotation_frequency(&[r]),
            Err(RotationFrequencyError::ZeroRotationId(0))
        );
    }

    /// **RKRF-04** — duplicate rotation ID rejected.
    #[test]
    fn rkrf_04_duplicate_rejected() {
        let rs = vec![
            rot(0x01, 1, 1_000_000),
            rot(0x01, 2, 1_600_000),
        ];
        assert_eq!(
            validate_rotation_frequency(&rs),
            Err(RotationFrequencyError::DuplicateRotationId { idx: 1 })
        );
    }

    /// **RKRF-05** — zero timestamp rejected.
    #[test]
    fn rkrf_05_zero_ts_rejected() {
        let r = RotationRecord { rotation_id: rid(0x01), epoch: 1, timestamp_ms: 0 };
        assert_eq!(
            validate_rotation_frequency(&[r]),
            Err(RotationFrequencyError::ZeroTimestamp(0))
        );
    }

    /// **RKRF-06** — too many rejected.
    #[test]
    fn rkrf_06_too_many_rejected() {
        let rs: Vec<RotationRecord> = (0..=RKRF_MAX_ROTATIONS)
            .map(|i| {
                let mut id = [0u8; RKRF_ROTATION_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                RotationRecord { rotation_id: id, epoch: val, timestamp_ms: val * 600_000 }
            })
            .collect();
        assert_eq!(
            validate_rotation_frequency(&rs),
            Err(RotationFrequencyError::TooMany {
                got: RKRF_MAX_ROTATIONS + 1,
                max: RKRF_MAX_ROTATIONS,
            })
        );
    }

    /// **RKRF-07** — valid accepted.
    #[test]
    fn rkrf_07_valid_accepted() {
        assert_eq!(validate_rotation_frequency(&valid_rotations()), Ok(()));
    }

    /// **RKRF-08** — empty accepted.
    #[test]
    fn rkrf_08_empty_accepted() {
        assert_eq!(validate_rotation_frequency(&[]), Ok(()));
    }

    /// **RKRF-09** — single rotation accepted.
    #[test]
    fn rkrf_09_single_accepted() {
        assert_eq!(validate_rotation_frequency(&[rot(0x01, 1, 1_000_000)]), Ok(()));
    }

    /// **RKRF-10** — boundary interval accepted.
    #[test]
    fn rkrf_10_boundary_interval_accepted() {
        let rs = vec![
            rot(0x01, 1, 1_000_000),
            rot(0x02, 2, 1_000_000 + RKRF_MAX_INTERVAL_MS),
        ];
        assert_eq!(validate_rotation_frequency(&rs), Ok(()));
    }
}
