//! # CR-CHAT-07 — Cover traffic inter-epoch continuity guard (Wave-103 Lane A)
//!
//! ANTI-CORRELATION — cover traffic rate must be consistent across epochs.
//!
//! When a key rotation or group change triggers a new epoch, the cover
//! traffic emission rate must not change detectably:
//!
//! * **Epoch boundary leakage** — if cover rate drops at epoch N, an
//!   observer knows a key rotation occurred at that point.
//! * **Group change fingerprinting** — rate changes correlated with
//!   group membership changes reveal who joined or left.
//! * **Activity correlation** — the epoch where rate changes is the
//!   same epoch where message content changed, enabling content-epoch
//!   correlation.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rate deviation between consecutive epochs <= `CTIC_MAX_RATE_DEV`.
//! 2. Epoch numbers must be strictly increasing.
//! 3. Epoch number must not be zero.
//! 4. Rate must be >= `CTIC_MIN_RATE`.
//! 5. Rate must be <= `CTIC_MAX_RATE`.
//! 6. Total epochs <= `CTIC_MAX_EPOCHS`.
//!
//! Tests **CTIC-01..10**. Error enum [`ContinuityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * INTER-EPOCH-CONTINUITY`

#![forbid(unsafe_code)]

/// Maximum rate deviation between consecutive epochs (msgs/sec).
pub const CTIC_MAX_RATE_DEV: f64 = 0.1;

/// Minimum emission rate (msgs/sec).
pub const CTIC_MIN_RATE: f64 = 0.5;

/// Maximum emission rate (msgs/sec).
pub const CTIC_MAX_RATE: f64 = 10.0;

/// Maximum epochs per batch.
pub const CTIC_MAX_EPOCHS: usize = 256;

/// An epoch's cover traffic rate record.
#[derive(Debug, Clone)]
pub struct EpochRate {
    /// Epoch number.
    pub epoch: u64,
    /// Cover traffic emission rate (msgs/sec).
    pub rate: f64,
}

/// All ways inter-epoch continuity validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum ContinuityError {
    /// Rate deviation exceeded.
    DeviationExceeded { idx: usize, prev: f64, current: f64, max_dev: f64 },
    /// Epoch not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Rate below minimum.
    BelowMin { idx: usize, rate: f64, min: f64 },
    /// Rate above maximum.
    AboveMax { idx: usize, rate: f64, max: f64 },
    /// Too many epochs.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic inter-epoch continuity.
pub fn validate_inter_epoch_continuity(
    epochs: &[EpochRate],
) -> Result<(), ContinuityError> {
    if epochs.len() > CTIC_MAX_EPOCHS {
        return Err(ContinuityError::TooMany {
            got: epochs.len(),
            max: CTIC_MAX_EPOCHS,
        });
    }
    let mut prev_epoch: u64 = 0;
    let mut prev_rate: f64 = 0.0;
    for (i, e) in epochs.iter().enumerate() {
        if e.epoch == 0 {
            return Err(ContinuityError::ZeroEpoch(i));
        }
        if e.rate < CTIC_MIN_RATE {
            return Err(ContinuityError::BelowMin {
                idx: i,
                rate: e.rate,
                min: CTIC_MIN_RATE,
            });
        }
        if e.rate > CTIC_MAX_RATE {
            return Err(ContinuityError::AboveMax {
                idx: i,
                rate: e.rate,
                max: CTIC_MAX_RATE,
            });
        }
        if i > 0 {
            if e.epoch <= prev_epoch {
                return Err(ContinuityError::NotIncreasing {
                    idx: i,
                    prev: prev_epoch,
                    current: e.epoch,
                });
            }
            let dev = (e.rate - prev_rate).abs() / prev_rate;
            if dev > CTIC_MAX_RATE_DEV {
                return Err(ContinuityError::DeviationExceeded {
                    idx: i,
                    prev: prev_rate,
                    current: e.rate,
                    max_dev: CTIC_MAX_RATE_DEV,
                });
            }
        }
        prev_epoch = e.epoch;
        prev_rate = e.rate;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch_rate(epoch: u64, rate: f64) -> EpochRate {
        EpochRate { epoch, rate }
    }

    fn valid_epochs() -> Vec<EpochRate> {
        vec![
            epoch_rate(1, 2.0),
            epoch_rate(2, 2.05),
            epoch_rate(3, 1.95),
        ]
    }

    /// **CTIC-01** — deviation exceeded rejected.
    #[test]
    fn ctic_01_deviation_exceeded_rejected() {
        let es = vec![epoch_rate(1, 2.0), epoch_rate(2, 3.0)];
        assert!(matches!(
            validate_inter_epoch_continuity(&es),
            Err(ContinuityError::DeviationExceeded { .. })
        ));
    }

    /// **CTIC-02** — not increasing rejected.
    #[test]
    fn ctic_02_not_increasing_rejected() {
        let es = vec![epoch_rate(5, 2.0), epoch_rate(3, 2.0)];
        assert_eq!(
            validate_inter_epoch_continuity(&es),
            Err(ContinuityError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **CTIC-03** — zero epoch rejected.
    #[test]
    fn ctic_03_zero_epoch_rejected() {
        let e = epoch_rate(0, 2.0);
        assert_eq!(
            validate_inter_epoch_continuity(&[e]),
            Err(ContinuityError::ZeroEpoch(0))
        );
    }

    /// **CTIC-04** — below minimum rejected.
    #[test]
    fn ctic_04_below_min_rejected() {
        let e = epoch_rate(1, 0.1);
        assert_eq!(
            validate_inter_epoch_continuity(&[e]),
            Err(ContinuityError::BelowMin {
                idx: 0,
                rate: 0.1,
                min: CTIC_MIN_RATE,
            })
        );
    }

    /// **CTIC-05** — above maximum rejected.
    #[test]
    fn ctic_05_above_max_rejected() {
        let e = epoch_rate(1, 15.0);
        assert_eq!(
            validate_inter_epoch_continuity(&[e]),
            Err(ContinuityError::AboveMax {
                idx: 0,
                rate: 15.0,
                max: CTIC_MAX_RATE,
            })
        );
    }

    /// **CTIC-06** — too many rejected.
    #[test]
    fn ctic_06_too_many_rejected() {
        let es: Vec<EpochRate> = (0..=CTIC_MAX_EPOCHS)
            .map(|i| EpochRate { epoch: (i as u64) + 1, rate: 2.0 })
            .collect();
        assert_eq!(
            validate_inter_epoch_continuity(&es),
            Err(ContinuityError::TooMany {
                got: CTIC_MAX_EPOCHS + 1,
                max: CTIC_MAX_EPOCHS,
            })
        );
    }

    /// **CTIC-07** — valid accepted.
    #[test]
    fn ctic_07_valid_accepted() {
        assert_eq!(validate_inter_epoch_continuity(&valid_epochs()), Ok(()));
    }

    /// **CTIC-08** — empty accepted.
    #[test]
    fn ctic_08_empty_accepted() {
        assert_eq!(validate_inter_epoch_continuity(&[]), Ok(()));
    }

    /// **CTIC-09** — single accepted.
    #[test]
    fn ctic_09_single_accepted() {
        let es = vec![epoch_rate(1, 2.0)];
        assert_eq!(validate_inter_epoch_continuity(&es), Ok(()));
    }

    /// **CTIC-10** — max deviation boundary accepted.
    #[test]
    fn ctic_10_boundary_accepted() {
        let r = 2.0 * (1.0 + CTIC_MAX_RATE_DEV * 0.99);
        let es = vec![epoch_rate(1, 2.0), epoch_rate(2, r)];
        assert_eq!(validate_inter_epoch_continuity(&es), Ok(()));
    }
}
