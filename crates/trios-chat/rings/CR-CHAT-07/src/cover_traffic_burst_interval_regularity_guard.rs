//! # CR-CHAT-07 — Cover traffic burst interval regularity guard (Wave-152 Lane B)
//!
//! ANTI-CORRELATION — cover traffic bursts must have irregular
//! intervals; regular patterns leak real message injection.
//!
//! When cover traffic is sent in bursts, the intervals between bursts
//! must not be predictable. If bursts arrive at regular intervals:
//!
//! * **Message injection detection** — an observer can identify when
//!   real messages are injected between regular cover bursts.
//! * **Pattern fingerprinting** — regular burst intervals create a
//!   unique fingerprint for the user's communication pattern.
//! * **Burst separation analysis** — consistent intervals allow an
//!   attacker to predict future burst timing.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Standard deviation of burst intervals must be >= `CTBI_MIN_STD_DEV`.
//! 2. No duplicate burst IDs.
//! 3. Burst ID must not be zero.
//! 4. Interval must be > 0.
//! 5. At least `CTBI_MIN_BURSTS` bursts.
//! 6. Batch size <= `CTBI_MAX_BURSTS`.
//!
//! Tests **CTBI-01..10**. Error enum [`BurstRegularityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BURST-IRREGULAR`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum bursts per batch.
pub const CTBI_MAX_BURSTS: usize = 4096;

/// Minimum bursts required.
pub const CTBI_MIN_BURSTS: usize = 8;

/// Minimum standard deviation of intervals (microseconds).
pub const CTBI_MIN_STD_DEV: u64 = 500;

/// Burst ID length.
pub const CTBI_BURST_ID_LEN: usize = 16;

/// A burst interval observation.
#[derive(Debug, Clone)]
pub struct BurstObservation {
    /// Burst identifier.
    pub burst_id: [u8; CTBI_BURST_ID_LEN],
    /// Interval since last burst in microseconds.
    pub interval_us: u64,
}

/// All ways burst regularity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BurstRegularityError {
    /// Intervals too regular (low std dev).
    TooRegular {
        /// Computed std dev.
        std_dev: u64,
        /// Minimum required.
        min: u64,
    },
    /// Duplicate burst ID.
    DuplicateId {
        /// Index.
        idx: usize,
    },
    /// Zero burst ID.
    ZeroId(usize),
    /// Zero interval.
    ZeroInterval(usize),
    /// Too few bursts.
    TooFew {
        got: usize,
        min: usize,
    },
    /// Too many bursts.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate cover traffic burst interval regularity.
pub fn validate_burst_regularity(
    bursts: &[BurstObservation],
) -> Result<(), BurstRegularityError> {
    if bursts.len() > CTBI_MAX_BURSTS {
        return Err(BurstRegularityError::TooMany {
            got: bursts.len(),
            max: CTBI_MAX_BURSTS,
        });
    }
    if bursts.len() < CTBI_MIN_BURSTS {
        return Err(BurstRegularityError::TooFew {
            got: bursts.len(),
            min: CTBI_MIN_BURSTS,
        });
    }
    let mut seen: BTreeSet<[u8; CTBI_BURST_ID_LEN]> = BTreeSet::new();
    let mut sum: u128 = 0;
    for (i, b) in bursts.iter().enumerate() {
        if b.burst_id == [0u8; CTBI_BURST_ID_LEN] {
            return Err(BurstRegularityError::ZeroId(i));
        }
        if !seen.insert(b.burst_id) {
            return Err(BurstRegularityError::DuplicateId { idx: i });
        }
        if b.interval_us == 0 {
            return Err(BurstRegularityError::ZeroInterval(i));
        }
        sum += b.interval_us as u128;
    }
    let n = bursts.len() as u128;
    let mean = sum / n;
    let variance_sum: u128 = bursts.iter().map(|b| {
        let diff = if b.interval_us as u128 > mean { b.interval_us as u128 - mean } else { mean - b.interval_us as u128 };
        diff * diff
    }).sum();
    let variance = (variance_sum / n) as u64;
    let std_dev = approx_sqrt(variance);
    if std_dev < CTBI_MIN_STD_DEV {
        return Err(BurstRegularityError::TooRegular { std_dev, min: CTBI_MIN_STD_DEV });
    }
    Ok(())
}

fn approx_sqrt(n: u64) -> u64 {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bid(byte: u8) -> [u8; CTBI_BURST_ID_LEN] {
        [byte; CTBI_BURST_ID_LEN]
    }

    fn burst(id: u8, interval_us: u64) -> BurstObservation {
        BurstObservation { burst_id: bid(id), interval_us }
    }

    fn irregular_bursts() -> Vec<BurstObservation> {
        vec![
            burst(0x01, 1000),
            burst(0x02, 5000),
            burst(0x03, 2000),
            burst(0x04, 8000),
            burst(0x05, 1500),
            burst(0x06, 12000),
            burst(0x07, 3000),
            burst(0x08, 7000),
        ]
    }

    /// **CTBI-01** — too regular rejected.
    #[test]
    fn ctbi_01_too_regular_rejected() {
        let bs: Vec<BurstObservation> = (0..10u8)
            .map(|i| burst(i + 1, 1000))
            .collect();
        let r = validate_burst_regularity(&bs);
        assert!(matches!(r, Err(BurstRegularityError::TooRegular { .. })));
    }

    /// **CTBI-02** — duplicate ID rejected.
    #[test]
    fn ctbi_02_duplicate_rejected() {
        let mut bs = irregular_bursts();
        bs.push(burst(0x01, 4000));
        assert_eq!(
            validate_burst_regularity(&bs),
            Err(BurstRegularityError::DuplicateId { idx: 8 })
        );
    }

    /// **CTBI-03** — zero ID rejected.
    #[test]
    fn ctbi_03_zero_id_rejected() {
        let mut bs = irregular_bursts();
        bs[0].burst_id = [0u8; CTBI_BURST_ID_LEN];
        assert_eq!(
            validate_burst_regularity(&bs),
            Err(BurstRegularityError::ZeroId(0))
        );
    }

    /// **CTBI-04** — zero interval rejected.
    #[test]
    fn ctbi_04_zero_interval_rejected() {
        let mut bs = irregular_bursts();
        bs[0].interval_us = 0;
        assert_eq!(
            validate_burst_regularity(&bs),
            Err(BurstRegularityError::ZeroInterval(0))
        );
    }

    /// **CTBI-05** — too few rejected.
    #[test]
    fn ctbi_05_too_few_rejected() {
        let bs: Vec<BurstObservation> = (0..5u8)
            .map(|i| burst(i + 1, 1000 + i as u64 * 1000))
            .collect();
        assert_eq!(
            validate_burst_regularity(&bs),
            Err(BurstRegularityError::TooFew { got: 5, min: CTBI_MIN_BURSTS })
        );
    }

    /// **CTBI-06** — too many rejected.
    #[test]
    fn ctbi_06_too_many_rejected() {
        let bs: Vec<BurstObservation> = (0..=CTBI_MAX_BURSTS)
            .map(|i| {
                let mut id = [0u8; CTBI_BURST_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                BurstObservation { burst_id: id, interval_us: 1000 + (i as u64) * 500 }
            })
            .collect();
        assert_eq!(
            validate_burst_regularity(&bs),
            Err(BurstRegularityError::TooMany {
                got: CTBI_MAX_BURSTS + 1,
                max: CTBI_MAX_BURSTS,
            })
        );
    }

    /// **CTBI-07** — valid accepted.
    #[test]
    fn ctbi_07_valid_accepted() {
        assert_eq!(validate_burst_regularity(&irregular_bursts()), Ok(()));
    }

    /// **CTBI-08** — empty rejected.
    #[test]
    fn ctbi_08_empty_rejected() {
        assert_eq!(
            validate_burst_regularity(&[]),
            Err(BurstRegularityError::TooFew { got: 0, min: CTBI_MIN_BURSTS })
        );
    }

    /// **CTBI-09** — exact minimum count accepted (with irregular intervals).
    #[test]
    fn ctbi_09_exact_min_accepted() {
        let bs: Vec<BurstObservation> = (0..CTBI_MIN_BURSTS as u8)
            .map(|i| burst(i + 1, 1000 + (i as u64) * (i as u64 + 1) * 100))
            .collect();
        assert_eq!(validate_burst_regularity(&bs), Ok(()));
    }

    /// **CTBI-10** — many irregular accepted.
    #[test]
    fn ctbi_10_many_irregular_accepted() {
        let bs: Vec<BurstObservation> = (0..100u8)
            .map(|i| {
                let mut id = [0u8; CTBI_BURST_ID_LEN];
                id[0] = i + 1;
                BurstObservation { burst_id: id, interval_us: 500 + ((i as u64 * 137) % 15000) }
            })
            .collect();
        assert_eq!(validate_burst_regularity(&bs), Ok(()));
    }
}
