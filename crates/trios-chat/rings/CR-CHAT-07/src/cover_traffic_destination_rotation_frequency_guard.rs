//! # CR-CHAT-07 — Cover traffic destination rotation frequency guard (Wave-133 Lane A)
//!
//! ANTI-CORRELATION — cover traffic destinations must rotate at a
//! minimum rate; static destinations leak the real recipient.
//!
//! Cover traffic is routed to multiple destinations. If the same
//! destination receives cover traffic continuously without rotation:
//!
//! * **Recipient fingerprint** — a destination that consistently
//!   receives cover traffic is identified as the real recipient.
//! * **Correlation attack** — matching cover destination frequency
//!   with real message timing reveals the true recipient.
//! * **Intersection attack** — observing which destinations receive
//!   traffic when the user is active narrows the candidate set.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each destination must appear <= `CDRF_MAX_PER_DEST` times.
//! 2. Destination hash must not be zero.
//! 3. Minimum unique destinations >= `CDRF_MIN_DESTINATIONS`.
//! 4. Timestamps must be strictly increasing.
//! 5. No duplicate emission IDs.
//! 6. Total emissions <= `CDRF_MAX_EMISSIONS`.
//!
//! Tests **CDRF-01..10**. Error enum [`DestRotationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DEST-ROTATING`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum emissions per destination.
pub const CDRF_MAX_PER_DEST: usize = 10;

/// Minimum unique destinations.
pub const CDRF_MIN_DESTINATIONS: usize = 3;

/// Maximum emissions per batch.
pub const CDRF_MAX_EMISSIONS: usize = 1024;

/// Destination hash length.
pub const CDRF_DEST_LEN: usize = 16;

/// Emission ID length.
pub const CDRF_EMISSION_ID_LEN: usize = 32;

/// A cover traffic emission to a destination.
#[derive(Debug, Clone)]
pub struct DestEmission {
    /// Emission identifier.
    pub emission_id: [u8; CDRF_EMISSION_ID_LEN],
    /// Destination hash.
    pub dest: [u8; CDRF_DEST_LEN],
    /// Timestamp in milliseconds.
    pub timestamp_ms: u64,
}

/// All ways destination rotation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DestRotationError {
    /// Too many emissions to single destination.
    TooManyPerDest { dest: [u8; CDRF_DEST_LEN], count: usize, max: usize },
    /// Zero destination.
    ZeroDest(usize),
    /// Too few unique destinations.
    TooFewDests { got: usize, min: usize },
    /// Non-monotonic timestamp.
    NonMonotonic { idx: usize, prev: u64, current: u64 },
    /// Duplicate emission ID.
    DuplicateEmissionId { idx: usize },
    /// Too many emissions.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate cover traffic destination rotation frequency.
pub fn validate_dest_rotation(
    emissions: &[DestEmission],
) -> Result<(), DestRotationError> {
    if emissions.len() > CDRF_MAX_EMISSIONS {
        return Err(DestRotationError::TooMany {
            got: emissions.len(),
            max: CDRF_MAX_EMISSIONS,
        });
    }
    let mut seen_ids: BTreeSet<[u8; CDRF_EMISSION_ID_LEN]> = BTreeSet::new();
    let mut dest_counts: std::collections::BTreeMap<[u8; CDRF_DEST_LEN], usize> =
        std::collections::BTreeMap::new();
    let mut prev_ts: u64 = 0;
    for (i, e) in emissions.iter().enumerate() {
        if e.dest == [0u8; CDRF_DEST_LEN] {
            return Err(DestRotationError::ZeroDest(i));
        }
        if !seen_ids.insert(e.emission_id) {
            return Err(DestRotationError::DuplicateEmissionId { idx: i });
        }
        if i > 0 && e.timestamp_ms <= prev_ts {
            return Err(DestRotationError::NonMonotonic {
                idx: i,
                prev: prev_ts,
                current: e.timestamp_ms,
            });
        }
        *dest_counts.entry(e.dest).or_insert(0) += 1;
        prev_ts = e.timestamp_ms;
    }
    for (&dest, &count) in &dest_counts {
        if count > CDRF_MAX_PER_DEST {
            return Err(DestRotationError::TooManyPerDest {
                dest,
                count,
                max: CDRF_MAX_PER_DEST,
            });
        }
    }
    if !emissions.is_empty() && dest_counts.len() < CDRF_MIN_DESTINATIONS {
        return Err(DestRotationError::TooFewDests {
            got: dest_counts.len(),
            min: CDRF_MIN_DESTINATIONS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(byte: u8) -> [u8; CDRF_EMISSION_ID_LEN] {
        [byte; CDRF_EMISSION_ID_LEN]
    }

    fn dest(byte: u8) -> [u8; CDRF_DEST_LEN] {
        [byte; CDRF_DEST_LEN]
    }

    fn emission(id: u8, d: u8, ts: u64) -> DestEmission {
        DestEmission { emission_id: eid(id), dest: dest(d), timestamp_ms: ts }
    }

    fn valid_batch() -> Vec<DestEmission> {
        vec![
            emission(0x01, 0x01, 1000),
            emission(0x02, 0x02, 2000),
            emission(0x03, 0x03, 3000),
            emission(0x04, 0x01, 4000),
            emission(0x05, 0x02, 5000),
            emission(0x06, 0x03, 6000),
        ]
    }

    /// **CDRF-01** — too many per dest rejected.
    #[test]
    fn cdrf_01_too_many_per_dest_rejected() {
        let es: Vec<DestEmission> = (0..=CDRF_MAX_PER_DEST)
            .map(|i| emission((i as u8).wrapping_add(1), 0x01, (i as u64) + 1))
            .collect();
        assert!(matches!(
            validate_dest_rotation(&es),
            Err(DestRotationError::TooManyPerDest { .. })
        ));
    }

    /// **CDRF-02** — zero dest rejected.
    #[test]
    fn cdrf_02_zero_dest_rejected() {
        let e = DestEmission { emission_id: eid(0x01), dest: [0u8; CDRF_DEST_LEN], timestamp_ms: 1000 };
        assert_eq!(
            validate_dest_rotation(&[e]),
            Err(DestRotationError::ZeroDest(0))
        );
    }

    /// **CDRF-03** — too few dests rejected.
    #[test]
    fn cdrf_03_too_few_dests_rejected() {
        let es = vec![
            emission(0x01, 0x01, 1000),
            emission(0x02, 0x01, 2000),
            emission(0x03, 0x01, 3000),
        ];
        assert_eq!(
            validate_dest_rotation(&es),
            Err(DestRotationError::TooFewDests { got: 1, min: CDRF_MIN_DESTINATIONS })
        );
    }

    /// **CDRF-04** — non-monotonic rejected.
    #[test]
    fn cdrf_04_non_monotonic_rejected() {
        let es = vec![
            emission(0x01, 0x01, 2000),
            emission(0x02, 0x02, 1000),
        ];
        assert_eq!(
            validate_dest_rotation(&es),
            Err(DestRotationError::NonMonotonic { idx: 1, prev: 2000, current: 1000 })
        );
    }

    /// **CDRF-05** — duplicate emission ID rejected.
    #[test]
    fn cdrf_05_duplicate_id_rejected() {
        let es = vec![
            emission(0x01, 0x01, 1000),
            emission(0x01, 0x02, 2000),
        ];
        assert_eq!(
            validate_dest_rotation(&es),
            Err(DestRotationError::DuplicateEmissionId { idx: 1 })
        );
    }

    /// **CDRF-06** — too many rejected.
    #[test]
    fn cdrf_06_too_many_rejected() {
        let mut es = Vec::new();
        for i in 0..=CDRF_MAX_EMISSIONS {
            let mut id = [0u8; CDRF_EMISSION_ID_LEN];
            let val = (i as u64) + 1;
            id[0..8].copy_from_slice(&val.to_be_bytes());
            let d = ((i % 5) + 1) as u8;
            es.push(DestEmission { emission_id: id, dest: dest(d), timestamp_ms: (i as u64) + 1 });
        }
        assert_eq!(
            validate_dest_rotation(&es),
            Err(DestRotationError::TooMany {
                got: CDRF_MAX_EMISSIONS + 1,
                max: CDRF_MAX_EMISSIONS,
            })
        );
    }

    /// **CDRF-07** — valid accepted.
    #[test]
    fn cdrf_07_valid_accepted() {
        assert_eq!(validate_dest_rotation(&valid_batch()), Ok(()));
    }

    /// **CDRF-08** — empty accepted.
    #[test]
    fn cdrf_08_empty_accepted() {
        assert_eq!(validate_dest_rotation(&[]), Ok(()));
    }

    /// **CDRF-09** — max per dest boundary accepted.
    #[test]
    fn cdrf_09_max_per_dest_accepted() {
        let mut es = Vec::new();
        let mut id = 1u8;
        for d in 1..=CDRF_MIN_DESTINATIONS {
            for _ in 0..CDRF_MAX_PER_DEST {
                es.push(DestEmission {
                    emission_id: eid(id),
                    dest: dest(d as u8),
                    timestamp_ms: id as u64,
                });
                id = id.wrapping_add(1);
            }
        }
        assert_eq!(validate_dest_rotation(&es), Ok(()));
    }

    /// **CDRF-10** — many rotating dests accepted.
    #[test]
    fn cdrf_10_many_rotating_accepted() {
        let es: Vec<DestEmission> = (0..50)
            .map(|i| DestEmission {
                emission_id: eid((i as u8).wrapping_add(1)),
                dest: dest((i % 5 + 1) as u8),
                timestamp_ms: (i as u64) + 1,
            })
            .collect();
        assert_eq!(validate_dest_rotation(&es), Ok(()));
    }
}
