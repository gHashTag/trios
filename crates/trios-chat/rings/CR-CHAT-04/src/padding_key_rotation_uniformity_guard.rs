//! # CR-CHAT-04 — Padding key rotation uniformity guard (Wave-101 Lane A)
//!
//! PADDING — padding keys must rotate at uniform intervals.
//!
//! Padding keys determine which size class is chosen for a given
//! message epoch. If key rotation is irregular:
//!
//! * **Pattern correlation** — if keys rotate every N messages, an
//!   observer can align size-class changes with key boundaries to
//!   determine the padding schedule.
//! * **Key reuse across epochs** — a stale key causes the same size
//!   class selection pattern to repeat, enabling cross-epoch
//!   correlation.
//! * **Side-channel leakage** — irregular rotation timing reveals
//!   when the system is under load (delayed rotation) vs. idle.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rotation interval must be >= `PKRU_MIN_INTERVAL`.
//! 2. Rotation interval must be <= `PKRU_MAX_INTERVAL`.
//! 3. Intervals must be monotonically non-decreasing (uniformity).
//! 4. No duplicate key IDs.
//! 5. Key ID must not be zero.
//! 6. Total rotations <= `PKRU_MAX_ROTATIONS`.
//!
//! Tests **PKRU-01..10**. Error enum [`KeyRotationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PADDING-KEY-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum rotation interval (messages).
pub const PKRU_MIN_INTERVAL: u64 = 100;

/// Maximum rotation interval (messages).
pub const PKRU_MAX_INTERVAL: u64 = 10_000;

/// Maximum rotations per batch.
pub const PKRU_MAX_ROTATIONS: usize = 1024;

/// Key ID length.
pub const PKRU_KEY_ID_LEN: usize = 16;

/// A padding key rotation record.
#[derive(Debug, Clone)]
pub struct KeyRotation {
    /// Key identifier.
    pub key_id: [u8; PKRU_KEY_ID_LEN],
    /// Interval since previous rotation (in messages).
    pub interval: u64,
}

/// All ways key rotation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyRotationError {
    /// Interval below minimum.
    TooFrequent { idx: usize, interval: u64, min: u64 },
    /// Interval above maximum.
    TooSlow { idx: usize, interval: u64, max: u64 },
    /// Not monotonic.
    NotMonotonic { idx: usize, prev: u64, current: u64 },
    /// Duplicate key ID.
    DuplicateKey(usize),
    /// Zero key ID.
    ZeroKey(usize),
    /// Too many rotations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate padding key rotation uniformity.
pub fn validate_key_rotations(
    rotations: &[KeyRotation],
) -> Result<(), KeyRotationError> {
    if rotations.len() > PKRU_MAX_ROTATIONS {
        return Err(KeyRotationError::TooMany {
            got: rotations.len(),
            max: PKRU_MAX_ROTATIONS,
        });
    }
    let mut seen: BTreeSet<[u8; PKRU_KEY_ID_LEN]> = BTreeSet::new();
    let mut prev: u64 = 0;
    for (i, r) in rotations.iter().enumerate() {
        if r.key_id == [0u8; PKRU_KEY_ID_LEN] {
            return Err(KeyRotationError::ZeroKey(i));
        }
        if r.interval < PKRU_MIN_INTERVAL {
            return Err(KeyRotationError::TooFrequent {
                idx: i,
                interval: r.interval,
                min: PKRU_MIN_INTERVAL,
            });
        }
        if r.interval > PKRU_MAX_INTERVAL {
            return Err(KeyRotationError::TooSlow {
                idx: i,
                interval: r.interval,
                max: PKRU_MAX_INTERVAL,
            });
        }
        if i > 0 && r.interval < prev {
            return Err(KeyRotationError::NotMonotonic {
                idx: i,
                prev,
                current: r.interval,
            });
        }
        if !seen.insert(r.key_id) {
            return Err(KeyRotationError::DuplicateKey(i));
        }
        prev = r.interval;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kid(byte: u8) -> [u8; PKRU_KEY_ID_LEN] {
        [byte; PKRU_KEY_ID_LEN]
    }

    fn rotation(key_byte: u8, interval: u64) -> KeyRotation {
        KeyRotation { key_id: kid(key_byte), interval }
    }

    fn valid_rotations() -> Vec<KeyRotation> {
        vec![
            rotation(0x01, 500),
            rotation(0x02, 500),
            rotation(0x03, 600),
        ]
    }

    /// **PKRU-01** — too frequent rejected.
    #[test]
    fn pkru_01_too_frequent_rejected() {
        let rs = vec![rotation(0x01, PKRU_MIN_INTERVAL - 1)];
        assert_eq!(
            validate_key_rotations(&rs),
            Err(KeyRotationError::TooFrequent {
                idx: 0,
                interval: PKRU_MIN_INTERVAL - 1,
                min: PKRU_MIN_INTERVAL,
            })
        );
    }

    /// **PKRU-02** — too slow rejected.
    #[test]
    fn pkru_02_too_slow_rejected() {
        let rs = vec![rotation(0x01, PKRU_MAX_INTERVAL + 1)];
        assert_eq!(
            validate_key_rotations(&rs),
            Err(KeyRotationError::TooSlow {
                idx: 0,
                interval: PKRU_MAX_INTERVAL + 1,
                max: PKRU_MAX_INTERVAL,
            })
        );
    }

    /// **PKRU-03** — not monotonic rejected.
    #[test]
    fn pkru_03_not_monotonic_rejected() {
        let rs = vec![rotation(0x01, 500), rotation(0x02, 400)];
        assert_eq!(
            validate_key_rotations(&rs),
            Err(KeyRotationError::NotMonotonic {
                idx: 1,
                prev: 500,
                current: 400,
            })
        );
    }

    /// **PKRU-04** — duplicate key rejected.
    #[test]
    fn pkru_04_duplicate_rejected() {
        let rs = vec![rotation(0x01, 500), rotation(0x01, 600)];
        assert_eq!(
            validate_key_rotations(&rs),
            Err(KeyRotationError::DuplicateKey(1))
        );
    }

    /// **PKRU-05** — zero key rejected.
    #[test]
    fn pkru_05_zero_key_rejected() {
        let r = KeyRotation { key_id: [0u8; PKRU_KEY_ID_LEN], interval: 500 };
        assert_eq!(
            validate_key_rotations(&[r]),
            Err(KeyRotationError::ZeroKey(0))
        );
    }

    /// **PKRU-06** — too many rotations rejected.
    #[test]
    fn pkru_06_too_many_rejected() {
        let rs: Vec<KeyRotation> = (0..=PKRU_MAX_ROTATIONS)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                KeyRotation { key_id: kid(b), interval: PKRU_MIN_INTERVAL }
            })
            .collect();
        assert_eq!(
            validate_key_rotations(&rs),
            Err(KeyRotationError::TooMany {
                got: PKRU_MAX_ROTATIONS + 1,
                max: PKRU_MAX_ROTATIONS,
            })
        );
    }

    /// **PKRU-07** — valid accepted.
    #[test]
    fn pkru_07_valid_accepted() {
        assert_eq!(validate_key_rotations(&valid_rotations()), Ok(()));
    }

    /// **PKRU-08** — empty accepted.
    #[test]
    fn pkru_08_empty_accepted() {
        assert_eq!(validate_key_rotations(&[]), Ok(()));
    }

    /// **PKRU-09** — single at min interval accepted.
    #[test]
    fn pkru_09_min_interval_accepted() {
        let rs = vec![rotation(0x01, PKRU_MIN_INTERVAL)];
        assert_eq!(validate_key_rotations(&rs), Ok(()));
    }

    /// **PKRU-10** — equal intervals accepted (non-decreasing).
    #[test]
    fn pkru_10_equal_intervals_accepted() {
        let rs = vec![rotation(0x01, 500), rotation(0x02, 500), rotation(0x03, 500)];
        assert_eq!(validate_key_rotations(&rs), Ok(()));
    }
}
