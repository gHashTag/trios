//! # CR-CHAT-07 — Cover traffic volume stability guard (Wave-106 Lane B)
//!
//! ANTI-CORRELATION — cover traffic total volume must be stable over time.
//!
//! The total number of emissions (real + cover) per time window must
//! remain stable. If the volume fluctuates:
//!
//! * **Activity detection** — a drop in total emissions reveals that
//!   the user went offline (no real messages + no cover emissions).
//! * **Burst detection** — a spike reveals a burst of real messages,
//!   leaking conversation activity patterns.
//! * **Correlation** — volume changes correlated with external events
//!   (e.g., news, market movements) reveal user behaviour.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Volume deviation <= `CTSG_MAX_DEVIATION` from mean.
//! 2. Window index must be strictly increasing.
//! 3. Window index must not be zero.
//! 4. Volume must be >= `CTSG_MIN_VOLUME`.
//! 5. Volume must be <= `CTSG_MAX_VOLUME`.
//! 6. Total windows <= `CTSG_MAX_WINDOWS`.
//!
//! Tests **CTSG-01..10**. Error enum [`VolumeStabilityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * VOLUME-STABILITY`

#![forbid(unsafe_code)]

/// Maximum deviation from mean (fraction of mean).
pub const CTSG_MAX_DEVIATION: f64 = 0.15;

/// Minimum volume per window.
pub const CTSG_MIN_VOLUME: u64 = 10;

/// Maximum volume per window.
pub const CTSG_MAX_VOLUME: u64 = 10_000;

/// Maximum windows per batch.
pub const CTSG_MAX_WINDOWS: usize = 256;

/// A volume window record.
#[derive(Debug, Clone)]
pub struct VolumeWindow {
    /// Window index (sequential).
    pub index: u64,
    /// Total emissions in this window.
    pub volume: u64,
}

/// All ways volume stability validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum VolumeStabilityError {
    /// Deviation exceeded.
    DeviationExceeded { idx: usize, volume: u64, mean: f64, max_dev: f64 },
    /// Not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero index.
    ZeroIndex(usize),
    /// Below minimum.
    BelowMin { idx: usize, volume: u64, min: u64 },
    /// Above maximum.
    AboveMax { idx: usize, volume: u64, max: u64 },
    /// Too many windows.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic volume stability.
pub fn validate_volume_stability(
    windows: &[VolumeWindow],
) -> Result<(), VolumeStabilityError> {
    if windows.len() > CTSG_MAX_WINDOWS {
        return Err(VolumeStabilityError::TooMany {
            got: windows.len(),
            max: CTSG_MAX_WINDOWS,
        });
    }
    if windows.is_empty() {
        return Ok(());
    }
    let sum: u64 = windows.iter().map(|w| w.volume).sum();
    let mean = sum as f64 / windows.len() as f64;
    let mut prev: u64 = 0;
    for (i, w) in windows.iter().enumerate() {
        if w.index == 0 {
            return Err(VolumeStabilityError::ZeroIndex(i));
        }
        if w.volume < CTSG_MIN_VOLUME {
            return Err(VolumeStabilityError::BelowMin {
                idx: i,
                volume: w.volume,
                min: CTSG_MIN_VOLUME,
            });
        }
        if w.volume > CTSG_MAX_VOLUME {
            return Err(VolumeStabilityError::AboveMax {
                idx: i,
                volume: w.volume,
                max: CTSG_MAX_VOLUME,
            });
        }
        if i > 0 && w.index <= prev {
            return Err(VolumeStabilityError::NotIncreasing {
                idx: i,
                prev,
                current: w.index,
            });
        }
        let dev = (w.volume as f64 - mean).abs() / mean;
        if dev > CTSG_MAX_DEVIATION {
            return Err(VolumeStabilityError::DeviationExceeded {
                idx: i,
                volume: w.volume,
                mean,
                max_dev: CTSG_MAX_DEVIATION,
            });
        }
        prev = w.index;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(index: u64, volume: u64) -> VolumeWindow {
        VolumeWindow { index, volume }
    }

    fn valid_windows() -> Vec<VolumeWindow> {
        vec![
            window(1, 100),
            window(2, 105),
            window(3, 95),
        ]
    }

    /// **CTSG-01** — deviation exceeded rejected.
    #[test]
    fn ctsg_01_deviation_rejected() {
        let ws = vec![window(1, 100), window(2, 200)];
        assert!(matches!(
            validate_volume_stability(&ws),
            Err(VolumeStabilityError::DeviationExceeded { .. })
        ));
    }

    /// **CTSG-02** — not increasing rejected.
    #[test]
    fn ctsg_02_not_increasing_rejected() {
        let ws = vec![window(5, 100), window(3, 100)];
        assert_eq!(
            validate_volume_stability(&ws),
            Err(VolumeStabilityError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **CTSG-03** — zero index rejected.
    #[test]
    fn ctsg_03_zero_index_rejected() {
        let w = VolumeWindow { index: 0, volume: 100 };
        assert_eq!(
            validate_volume_stability(&[w]),
            Err(VolumeStabilityError::ZeroIndex(0))
        );
    }

    /// **CTSG-04** — below minimum rejected.
    #[test]
    fn ctsg_04_below_min_rejected() {
        let w = VolumeWindow { index: 1, volume: 5 };
        assert_eq!(
            validate_volume_stability(&[w]),
            Err(VolumeStabilityError::BelowMin {
                idx: 0,
                volume: 5,
                min: CTSG_MIN_VOLUME,
            })
        );
    }

    /// **CTSG-05** — above maximum rejected.
    #[test]
    fn ctsg_05_above_max_rejected() {
        let w = VolumeWindow { index: 1, volume: CTSG_MAX_VOLUME + 1 };
        assert_eq!(
            validate_volume_stability(&[w]),
            Err(VolumeStabilityError::AboveMax {
                idx: 0,
                volume: CTSG_MAX_VOLUME + 1,
                max: CTSG_MAX_VOLUME,
            })
        );
    }

    /// **CTSG-06** — too many rejected.
    #[test]
    fn ctsg_06_too_many_rejected() {
        let ws: Vec<VolumeWindow> = (0..=CTSG_MAX_WINDOWS)
            .map(|i| VolumeWindow { index: (i as u64) + 1, volume: 100 })
            .collect();
        assert_eq!(
            validate_volume_stability(&ws),
            Err(VolumeStabilityError::TooMany {
                got: CTSG_MAX_WINDOWS + 1,
                max: CTSG_MAX_WINDOWS,
            })
        );
    }

    /// **CTSG-07** — valid accepted.
    #[test]
    fn ctsg_07_valid_accepted() {
        assert_eq!(validate_volume_stability(&valid_windows()), Ok(()));
    }

    /// **CTSG-08** — empty accepted.
    #[test]
    fn ctsg_08_empty_accepted() {
        assert_eq!(validate_volume_stability(&[]), Ok(()));
    }

    /// **CTSG-09** — single accepted.
    #[test]
    fn ctsg_09_single_accepted() {
        let ws = vec![window(1, 100)];
        assert_eq!(validate_volume_stability(&ws), Ok(()));
    }

    /// **CTSG-10** — stable uniform accepted.
    #[test]
    fn ctsg_10_uniform_accepted() {
        let ws: Vec<VolumeWindow> = (0..10)
            .map(|i| VolumeWindow { index: (i as u64) + 1, volume: 100 })
            .collect();
        assert_eq!(validate_volume_stability(&ws), Ok(()));
    }
}
