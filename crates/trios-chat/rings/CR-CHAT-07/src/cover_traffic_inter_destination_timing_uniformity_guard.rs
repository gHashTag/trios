//! # CR-CHAT-07 — Cover traffic inter-destination timing uniformity guard (Wave-118 Lane B)
//!
//! ANTI-CORRELATION — cover traffic must have uniform inter-destination
//! timing; biased timing towards a specific destination leaks the real
//! recipient.
//!
//! When cover traffic is routed to destinations, the timing distribution
//! per destination must be statistically uniform. A biased distribution:
//!
//! * **Destination fingerprint** — if cover traffic is routed to
//!   destination A more often or faster, the observer learns A is the
//!   likely real recipient.
//! * **Burst correlation** — a burst of cover to one destination
//!   correlates with a real message burst to the same destination.
//! * **Traffic analysis** — statistical timing analysis across
//!   destinations reveals the true communication pattern.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Inter-destination timing stddev <= `CIDT_MAX_STDDEV`.
//! 2. Destination hash must not be zero.
//! 3. Timestamp must be > 0.
//! 4. Timestamps must be monotonically increasing.
//! 5. Minimum emissions per destination >= `CIDT_MIN_PER_DEST`.
//! 6. Total emissions <= `CIDT_MAX_EMISSIONS`.
//!
//! Tests **CIDT-01..10**. Error enum [`DestTimingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TIMING-UNIFORM`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Maximum allowed standard deviation of inter-destination timing.
pub const CIDT_MAX_STDDEV: f64 = 5000.0;

/// Minimum emissions per destination for statistical validity.
pub const CIDT_MIN_PER_DEST: usize = 2;

/// Maximum emissions per batch.
pub const CIDT_MAX_EMISSIONS: usize = 1024;

/// Destination hash length.
pub const CIDT_DEST_LEN: usize = 16;

/// A cover traffic emission to a destination.
#[derive(Debug, Clone)]
pub struct DestTimingEmission {
    /// Destination hash.
    pub dest: [u8; CIDT_DEST_LEN],
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

/// All ways inter-destination timing validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum DestTimingError {
    /// Timing stddev too high.
    HighStddev { stddev: f64, max: f64 },
    /// Zero destination.
    ZeroDest(usize),
    /// Zero timestamp.
    ZeroTimestamp(usize),
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too few emissions for a destination.
    TooFew { dest_idx: usize, got: usize, min: usize },
    /// Too many emissions.
    TooMany { got: usize, max: usize },
}

fn stddev(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}

/// `[VERIFIED]` Validate cover traffic inter-destination timing uniformity.
pub fn validate_dest_timing(
    emissions: &[DestTimingEmission],
) -> Result<(), DestTimingError> {
    if emissions.len() > CIDT_MAX_EMISSIONS {
        return Err(DestTimingError::TooMany {
            got: emissions.len(),
            max: CIDT_MAX_EMISSIONS,
        });
    }
    let mut prev_ts: u64 = 0;
    for (i, e) in emissions.iter().enumerate() {
        if e.dest == [0u8; CIDT_DEST_LEN] {
            return Err(DestTimingError::ZeroDest(i));
        }
        if e.timestamp_ms == 0 {
            return Err(DestTimingError::ZeroTimestamp(i));
        }
        if e.timestamp_ms <= prev_ts && i > 0 {
            return Err(DestTimingError::NonMonotonic {
                idx: i,
                prev: prev_ts,
                current: e.timestamp_ms,
            });
        }
        prev_ts = e.timestamp_ms;
    }
    let mut dest_times: BTreeMap<[u8; CIDT_DEST_LEN], Vec<u64>> = BTreeMap::new();
    for e in emissions {
        dest_times.entry(e.dest).or_default().push(e.timestamp_ms);
    }
    let mut all_means: Vec<f64> = Vec::new();
    for (idx, (_, times)) in dest_times.iter().enumerate() {
        if times.len() < CIDT_MIN_PER_DEST {
            return Err(DestTimingError::TooFew {
                dest_idx: idx,
                got: times.len(),
                min: CIDT_MIN_PER_DEST,
            });
        }
        let mean_ts = times.iter().sum::<u64>() as f64 / times.len() as f64;
        all_means.push(mean_ts);
    }
    if all_means.len() >= 2 {
        let s = stddev(&all_means);
        if s > CIDT_MAX_STDDEV {
            return Err(DestTimingError::HighStddev { stddev: s, max: CIDT_MAX_STDDEV });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dest(byte: u8) -> [u8; CIDT_DEST_LEN] {
        [byte; CIDT_DEST_LEN]
    }

    fn emission(d: u8, ts: u64) -> DestTimingEmission {
        DestTimingEmission { dest: dest(d), timestamp_ms: ts }
    }

    /// **CIDT-01** — high stddev rejected.
    #[test]
    fn cidt_01_high_stddev_rejected() {
        let es = vec![
            emission(0x01, 1000),
            emission(0x01, 2000),
            emission(0x02, 100_000),
            emission(0x02, 200_000),
        ];
        assert!(matches!(
            validate_dest_timing(&es),
            Err(DestTimingError::HighStddev { .. })
        ));
    }

    /// **CIDT-02** — zero dest rejected.
    #[test]
    fn cidt_02_zero_dest_rejected() {
        let e = DestTimingEmission { dest: [0u8; CIDT_DEST_LEN], timestamp_ms: 1000 };
        assert_eq!(
            validate_dest_timing(&[e]),
            Err(DestTimingError::ZeroDest(0))
        );
    }

    /// **CIDT-03** — zero timestamp rejected.
    #[test]
    fn cidt_03_zero_timestamp_rejected() {
        let e = DestTimingEmission { dest: dest(0x01), timestamp_ms: 0 };
        assert_eq!(
            validate_dest_timing(&[e]),
            Err(DestTimingError::ZeroTimestamp(0))
        );
    }

    /// **CIDT-04** — non-monotonic rejected.
    #[test]
    fn cidt_04_non_monotonic_rejected() {
        let es = vec![
            emission(0x01, 2000),
            emission(0x02, 1000),
        ];
        assert_eq!(
            validate_dest_timing(&es),
            Err(DestTimingError::NonMonotonic { idx: 1, prev: 2000, current: 1000 })
        );
    }

    /// **CIDT-05** — too few per dest rejected.
    #[test]
    fn cidt_05_too_few_rejected() {
        let es = vec![
            emission(0x01, 1000),
            emission(0x01, 2000),
            emission(0x02, 3000),
        ];
        assert!(matches!(
            validate_dest_timing(&es),
            Err(DestTimingError::TooFew { dest_idx: 1, got: 1, min: 2 })
        ));
    }

    /// **CIDT-06** — too many rejected.
    #[test]
    fn cidt_06_too_many_rejected() {
        let es: Vec<DestTimingEmission> = (0..=CIDT_MAX_EMISSIONS)
            .map(|i| {
                let d = (i as u64 % 3 + 1) as u8;
                emission(d, (i as u64) + 1)
            })
            .collect();
        assert_eq!(
            validate_dest_timing(&es),
            Err(DestTimingError::TooMany {
                got: CIDT_MAX_EMISSIONS + 1,
                max: CIDT_MAX_EMISSIONS,
            })
        );
    }

    /// **CIDT-07** — uniform accepted.
    #[test]
    fn cidt_07_uniform_accepted() {
        let es = vec![
            emission(0x01, 1000),
            emission(0x02, 1500),
            emission(0x01, 2000),
            emission(0x02, 2500),
        ];
        assert_eq!(validate_dest_timing(&es), Ok(()));
    }

    /// **CIDT-08** — empty accepted.
    #[test]
    fn cidt_08_empty_accepted() {
        assert_eq!(validate_dest_timing(&[]), Ok(()));
    }

    /// **CIDT-09** — single dest accepted.
    #[test]
    fn cidt_09_single_dest_accepted() {
        let es = vec![
            emission(0x01, 1000),
            emission(0x01, 2000),
        ];
        assert_eq!(validate_dest_timing(&es), Ok(()));
    }

    /// **CIDT-10** — many uniform destinations accepted.
    #[test]
    fn cidt_10_many_uniform_accepted() {
        let mut es = Vec::new();
        let mut ts: u64 = 1;
        for d in 1u8..=5u8 {
            for _ in 0..4u64 {
                es.push(emission(d, ts));
                ts += 1;
            }
        }
        assert_eq!(validate_dest_timing(&es), Ok(()));
    }
}
