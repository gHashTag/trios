//! # CR-CHAT-01 — Prekey bundle rotation rate limit guard (Wave-122 Lane A)
//!
//! IDENTITY — prekey bundles must not be rotated too frequently; rapid
//! rotation can indicate key compromise or enable denial-of-service.
//!
//! Rotating prekey bundles at an extremely high rate is suspicious:
//!
//! * **Key compromise indicator** — an attacker who compromises keys
//!   may rapidly rotate to cover their tracks, making forensic
//!   analysis harder.
//! * **Denial of service** — forcing rapid bundle rotation exhausts
//!   server-side storage and network bandwidth.
//! * **State synchronization failure** — peers can't keep up with
//!   rapid rotations, causing message delivery failures.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rotation interval >= `PBRL_MIN_INTERVAL_MS`.
//! 2. Rotation interval <= `PBRL_MAX_INTERVAL_MS`.
//! 3. Bundle ID must not be zero.
//! 4. No duplicate bundle IDs.
//! 5. Timestamps must be strictly increasing.
//! 6. Total rotations <= `PBRL_MAX_ROTATIONS`.
//!
//! Tests **PBRL-01..10**. Error enum [`RotationRateError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATE-LIMITED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum rotation interval in milliseconds.
pub const PBRL_MIN_INTERVAL_MS: u64 = 60_000;

/// Maximum rotation interval in milliseconds.
pub const PBRL_MAX_INTERVAL_MS: u64 = 7 * 24 * 3600 * 1000;

/// Maximum rotations per batch.
pub const PBRL_MAX_ROTATIONS: usize = 1024;

/// Bundle ID length.
pub const PBRL_BUNDLE_ID_LEN: usize = 32;

/// A prekey bundle rotation event.
#[derive(Debug, Clone)]
pub struct RotationEvent {
    /// Bundle identifier.
    pub bundle_id: [u8; PBRL_BUNDLE_ID_LEN],
    /// Timestamp of this rotation in milliseconds.
    pub timestamp_ms: u64,
}

/// All ways rotation rate validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RotationRateError {
    /// Rotation too fast.
    TooFast { idx: usize, interval_ms: u64, min: u64 },
    /// Rotation too slow.
    TooSlow { idx: usize, interval_ms: u64, max: u64 },
    /// Zero bundle ID.
    ZeroBundleId(usize),
    /// Duplicate bundle ID.
    DuplicateBundleId { idx: usize },
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too many rotations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate prekey bundle rotation rate.
pub fn validate_rotation_rate(
    rotations: &[RotationEvent],
) -> Result<(), RotationRateError> {
    if rotations.len() > PBRL_MAX_ROTATIONS {
        return Err(RotationRateError::TooMany {
            got: rotations.len(),
            max: PBRL_MAX_ROTATIONS,
        });
    }
    let mut seen: BTreeSet<[u8; PBRL_BUNDLE_ID_LEN]> = BTreeSet::new();
    let mut prev_ts: u64 = 0;
    for (i, r) in rotations.iter().enumerate() {
        if r.bundle_id == [0u8; PBRL_BUNDLE_ID_LEN] {
            return Err(RotationRateError::ZeroBundleId(i));
        }
        if !seen.insert(r.bundle_id) {
            return Err(RotationRateError::DuplicateBundleId { idx: i });
        }
        if i > 0 {
            if r.timestamp_ms <= prev_ts {
                return Err(RotationRateError::NonMonotonic {
                    idx: i,
                    prev: prev_ts,
                    current: r.timestamp_ms,
                });
            }
            let interval = r.timestamp_ms - prev_ts;
            if interval < PBRL_MIN_INTERVAL_MS {
                return Err(RotationRateError::TooFast {
                    idx: i,
                    interval_ms: interval,
                    min: PBRL_MIN_INTERVAL_MS,
                });
            }
            if interval > PBRL_MAX_INTERVAL_MS {
                return Err(RotationRateError::TooSlow {
                    idx: i,
                    interval_ms: interval,
                    max: PBRL_MAX_INTERVAL_MS,
                });
            }
        }
        prev_ts = r.timestamp_ms;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; PBRL_BUNDLE_ID_LEN] {
        [byte; PBRL_BUNDLE_ID_LEN]
    }

    fn rotation(bundle: u8, ts: u64) -> RotationEvent {
        RotationEvent { bundle_id: bid(bundle), timestamp_ms: ts }
    }

    fn valid_rotations() -> Vec<RotationEvent> {
        vec![
            rotation(0x01, 1_000_000),
            rotation(0x02, 1_000_000 + PBRL_MIN_INTERVAL_MS),
            rotation(0x03, 1_000_000 + PBRL_MIN_INTERVAL_MS * 2),
        ]
    }

    /// **PBRL-01** — too fast rejected.
    #[test]
    fn pbrl_01_too_fast_rejected() {
        let rs = vec![
            rotation(0x01, 1_000_000),
            rotation(0x02, 1_000_001),
        ];
        assert_eq!(
            validate_rotation_rate(&rs),
            Err(RotationRateError::TooFast {
                idx: 1,
                interval_ms: 1,
                min: PBRL_MIN_INTERVAL_MS,
            })
        );
    }

    /// **PBRL-02** — too slow rejected.
    #[test]
    fn pbrl_02_too_slow_rejected() {
        let rs = vec![
            rotation(0x01, 1_000_000),
            rotation(0x02, 1_000_000 + PBRL_MAX_INTERVAL_MS + 1),
        ];
        assert_eq!(
            validate_rotation_rate(&rs),
            Err(RotationRateError::TooSlow {
                idx: 1,
                interval_ms: PBRL_MAX_INTERVAL_MS + 1,
                max: PBRL_MAX_INTERVAL_MS,
            })
        );
    }

    /// **PBRL-03** — zero bundle ID rejected.
    #[test]
    fn pbrl_03_zero_bundle_rejected() {
        let r = RotationEvent { bundle_id: [0u8; PBRL_BUNDLE_ID_LEN], timestamp_ms: 1_000_000 };
        assert_eq!(
            validate_rotation_rate(&[r]),
            Err(RotationRateError::ZeroBundleId(0))
        );
    }

    /// **PBRL-04** — duplicate bundle ID rejected.
    #[test]
    fn pbrl_04_duplicate_rejected() {
        let rs = vec![
            rotation(0x01, 1_000_000),
            rotation(0x01, 1_000_000 + PBRL_MIN_INTERVAL_MS),
        ];
        assert_eq!(
            validate_rotation_rate(&rs),
            Err(RotationRateError::DuplicateBundleId { idx: 1 })
        );
    }

    /// **PBRL-05** — non-monotonic rejected.
    #[test]
    fn pbrl_05_non_monotonic_rejected() {
        let rs = vec![
            rotation(0x01, 2_000_000),
            rotation(0x02, 1_000_000),
        ];
        assert_eq!(
            validate_rotation_rate(&rs),
            Err(RotationRateError::NonMonotonic { idx: 1, prev: 2_000_000, current: 1_000_000 })
        );
    }

    /// **PBRL-06** — too many rejected.
    #[test]
    fn pbrl_06_too_many_rejected() {
        let rs: Vec<RotationEvent> = (0..=PBRL_MAX_ROTATIONS)
            .map(|i| {
                let mut id = [0u8; PBRL_BUNDLE_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                RotationEvent { bundle_id: id, timestamp_ms: 1_000_000 + (i as u64) * PBRL_MIN_INTERVAL_MS }
            })
            .collect();
        assert_eq!(
            validate_rotation_rate(&rs),
            Err(RotationRateError::TooMany {
                got: PBRL_MAX_ROTATIONS + 1,
                max: PBRL_MAX_ROTATIONS,
            })
        );
    }

    /// **PBRL-07** — valid accepted.
    #[test]
    fn pbrl_07_valid_accepted() {
        assert_eq!(validate_rotation_rate(&valid_rotations()), Ok(()));
    }

    /// **PBRL-08** — empty accepted.
    #[test]
    fn pbrl_08_empty_accepted() {
        assert_eq!(validate_rotation_rate(&[]), Ok(()));
    }

    /// **PBRL-09** — single accepted.
    #[test]
    fn pbrl_09_single_accepted() {
        assert_eq!(validate_rotation_rate(&[rotation(0x01, 1_000_000)]), Ok(()));
    }

    /// **PBRL-10** — boundary interval accepted.
    #[test]
    fn pbrl_10_boundary_accepted() {
        let rs = vec![
            rotation(0x01, 1_000_000),
            rotation(0x02, 1_000_000 + PBRL_MIN_INTERVAL_MS),
        ];
        assert_eq!(validate_rotation_rate(&rs), Ok(()));
    }
}
