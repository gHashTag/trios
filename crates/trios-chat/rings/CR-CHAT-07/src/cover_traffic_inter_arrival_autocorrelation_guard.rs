//! # CR-CHAT-07 — Cover traffic inter-arrival autocorrelation guard (Wave-144 Lane B)
//!
//! ANTI-CORRELATION — inter-arrival times of cover traffic must not
//! show autocorrelation; patterns reveal the scheduling algorithm.
//!
//! Cover traffic is generated at intervals to hide real traffic
//! patterns. If consecutive inter-arrival times are correlated:
//!
//! * **Scheduler detection** — autocorrelation reveals the
//!   deterministic scheduling algorithm, enabling prediction.
//! * **Pattern extraction** — correlated intervals create a
//!   recognizable signature distinguishable from random traffic.
//! * **Prediction attack** — once the pattern is known, an attacker
//!   can predict when real traffic will be sent between covers.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Lag-1 autocorrelation <= `CTIA_MAX_AUTOCORR`.
//! 2. Minimum intervals >= `CTIA_MIN_INTERVALS`.
//! 3. Mean interval >= `CTIA_MIN_MEAN_US`.
//! 4. No duplicate observation IDs.
//! 5. Observation ID must not be zero.
//! 6. Batch size <= `CTIA_MAX_OBSERVATIONS`.
//!
//! Tests **CTIA-01..10**. Error enum [`AutocorrelationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * AUTOCORR-ZERO`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum lag-1 autocorrelation (×1000).
pub const CTIA_MAX_AUTOCORR: i64 = 300;

/// Minimum number of intervals.
pub const CTIA_MIN_INTERVALS: usize = 8;

/// Minimum mean interval in microseconds.
pub const CTIA_MIN_MEAN_US: u64 = 100;

/// Maximum observations per batch.
pub const CTIA_MAX_OBSERVATIONS: usize = 512;

/// Observation ID length.
pub const CTIA_OBS_ID_LEN: usize = 16;

/// An inter-arrival observation record.
#[derive(Debug, Clone)]
pub struct IntervalObservation {
    /// Observation identifier.
    pub obs_id: [u8; CTIA_OBS_ID_LEN],
    /// Inter-arrival interval in microseconds.
    pub interval_us: u64,
}

/// All ways autocorrelation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AutocorrelationError {
    /// Autocorrelation too high.
    HighAutocorr {
        /// Computed autocorrelation ×1000.
        got: i64,
        /// Maximum allowed ×1000.
        max: i64,
    },
    /// Too few intervals.
    TooFew { got: usize, min: usize },
    /// Mean interval too low.
    LowMean { got: u64, min: u64 },
    /// Duplicate observation ID.
    Duplicate { idx: usize },
    /// Zero observation ID.
    ZeroId(usize),
    /// Too many observations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic inter-arrival autocorrelation.
pub fn validate_inter_arrival_autocorr(
    observations: &[IntervalObservation],
) -> Result<(), AutocorrelationError> {
    if observations.len() > CTIA_MAX_OBSERVATIONS {
        return Err(AutocorrelationError::TooMany {
            got: observations.len(),
            max: CTIA_MAX_OBSERVATIONS,
        });
    }
    if observations.is_empty() {
        return Ok(());
    }
    if observations.len() < CTIA_MIN_INTERVALS {
        return Err(AutocorrelationError::TooFew {
            got: observations.len(),
            min: CTIA_MIN_INTERVALS,
        });
    }
    let mut seen: BTreeSet<[u8; CTIA_OBS_ID_LEN]> = BTreeSet::new();
    for (i, o) in observations.iter().enumerate() {
        if o.obs_id == [0u8; CTIA_OBS_ID_LEN] {
            return Err(AutocorrelationError::ZeroId(i));
        }
        if !seen.insert(o.obs_id) {
            return Err(AutocorrelationError::Duplicate { idx: i });
        }
    }
    let sum: u64 = observations.iter().map(|o| o.interval_us).sum();
    let mean = sum / observations.len() as u64;
    if mean < CTIA_MIN_MEAN_US {
        return Err(AutocorrelationError::LowMean { got: mean, min: CTIA_MIN_MEAN_US });
    }
    let intervals: Vec<i64> = observations.iter().map(|o| o.interval_us as i64).collect();
    let mean_i = mean as i64;
    let mut num: i64 = 0;
    let mut den: i64 = 0;
    for i in 0..intervals.len() {
        let diff = intervals[i] - mean_i;
        den += diff * diff;
        if i > 0 {
            let prev_diff = intervals[i - 1] - mean_i;
            num += prev_diff * diff;
        }
    }
    if den == 0 {
        return Ok(());
    }
    let autocorr_x1000 = (num * 1000) / den;
    if autocorr_x1000 > CTIA_MAX_AUTOCORR {
        return Err(AutocorrelationError::HighAutocorr {
            got: autocorr_x1000,
            max: CTIA_MAX_AUTOCORR,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; CTIA_OBS_ID_LEN] {
        [byte; CTIA_OBS_ID_LEN]
    }

    fn obs(id: u8, interval: u64) -> IntervalObservation {
        IntervalObservation { obs_id: oid(id), interval_us: interval }
    }

    fn alternating_observations() -> Vec<IntervalObservation> {
        (0..20u8)
            .map(|i| obs(i + 1, 500 + ((i as u64) % 3) * 100))
            .collect()
    }

    /// **CTIA-01** — high autocorrelation rejected.
    #[test]
    fn ctia_01_high_autocorr_rejected() {
        let os: Vec<IntervalObservation> = (0..10u8)
            .map(|i| obs(i + 1, 100 + (i as u64) * 1000))
            .collect();
        let result = validate_inter_arrival_autocorr(&os);
        assert!(matches!(result, Err(AutocorrelationError::HighAutocorr { .. })));
    }

    /// **CTIA-02** — too few intervals rejected.
    #[test]
    fn ctia_02_too_few_rejected() {
        let os = vec![obs(0x01, 500), obs(0x02, 500), obs(0x03, 500)];
        assert_eq!(
            validate_inter_arrival_autocorr(&os),
            Err(AutocorrelationError::TooFew { got: 3, min: CTIA_MIN_INTERVALS })
        );
    }

    /// **CTIA-03** — low mean rejected.
    #[test]
    fn ctia_03_low_mean_rejected() {
        let os: Vec<IntervalObservation> = (0..10u8)
            .map(|i| obs(i + 1, 10))
            .collect();
        assert!(matches!(
            validate_inter_arrival_autocorr(&os),
            Err(AutocorrelationError::LowMean { .. })
        ));
    }

    /// **CTIA-04** — duplicate rejected.
    #[test]
    fn ctia_04_duplicate_rejected() {
        let os = vec![
            obs(0x01, 500), obs(0x01, 600), obs(0x02, 500), obs(0x03, 500),
            obs(0x04, 500), obs(0x05, 500), obs(0x06, 500), obs(0x07, 500),
            obs(0x08, 500),
        ];
        assert_eq!(
            validate_inter_arrival_autocorr(&os),
            Err(AutocorrelationError::Duplicate { idx: 1 })
        );
    }

    /// **CTIA-05** — zero ID rejected.
    #[test]
    fn ctia_05_zero_id_rejected() {
        let o = IntervalObservation { obs_id: [0u8; CTIA_OBS_ID_LEN], interval_us: 500 };
        let os: Vec<IntervalObservation> = std::iter::once(o)
            .chain((1..CTIA_MIN_INTERVALS).map(|i| obs(i as u8, 500)))
            .collect();
        assert_eq!(
            validate_inter_arrival_autocorr(&os),
            Err(AutocorrelationError::ZeroId(0))
        );
    }

    /// **CTIA-06** — too many rejected.
    #[test]
    fn ctia_06_too_many_rejected() {
        let os: Vec<IntervalObservation> = (0..=CTIA_MAX_OBSERVATIONS)
            .map(|i| {
                let mut id = [0u8; CTIA_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                IntervalObservation { obs_id: id, interval_us: 500 }
            })
            .collect();
        assert_eq!(
            validate_inter_arrival_autocorr(&os),
            Err(AutocorrelationError::TooMany { got: CTIA_MAX_OBSERVATIONS + 1, max: CTIA_MAX_OBSERVATIONS })
        );
    }

    /// **CTIA-07** — valid accepted.
    #[test]
    fn ctia_07_valid_accepted() {
        assert_eq!(validate_inter_arrival_autocorr(&alternating_observations()), Ok(()));
    }

    /// **CTIA-08** — empty accepted.
    #[test]
    fn ctia_08_empty_accepted() {
        assert_eq!(validate_inter_arrival_autocorr(&[]), Ok(()));
    }

    /// **CTIA-09** — constant intervals accepted (zero autocorr).
    #[test]
    fn ctia_09_constant_accepted() {
        let os: Vec<IntervalObservation> = (0..10u8)
            .map(|i| obs(i + 1, 1000))
            .collect();
        assert_eq!(validate_inter_arrival_autocorr(&os), Ok(()));
    }

    /// **CTIA-10** — many alternating accepted.
    #[test]
    fn ctia_10_many_alternating_accepted() {
        let os: Vec<IntervalObservation> = (0..50u8)
            .map(|i| {
                let interval = if i % 2 == 0 { 400u64 } else { 600 };
                obs(i + 1, interval)
            })
            .collect();
        assert_eq!(validate_inter_arrival_autocorr(&os), Ok(()));
    }
}
