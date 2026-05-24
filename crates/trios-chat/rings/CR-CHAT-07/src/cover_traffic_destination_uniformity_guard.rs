//! # CR-CHAT-07 — Cover traffic destination uniformity guard (Wave-110 Lane A)
//!
//! ANTI-CORRELATION — cover traffic destinations must be uniformly distributed.
//!
//! Cover traffic envelopes are routed to cover destination hashes. If
//! the distribution of cover destinations is skewed:
//!
//! * **Recipient identification** — if 90% of cover traffic goes to
//!   10% of destinations, the remaining 90% of destinations are
//!   likely real recipients.
//! * **Volume fingerprint** — the per-destination cover volume
//!   uniquely fingerprints the user's contact list.
//! * **Statistical test** — a chi-squared test on destination
//!   frequencies distinguishes cover from real traffic.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each destination must have >= `CDUG_MIN_PER_DEST` cover emissions.
//! 2. No destination exceeds `CDUG_MAX_RATIO` of total.
//! 3. Total emissions >= `CDUG_MIN_EMISSIONS`.
//! 4. All destinations must be present in the declared set.
//! 5. Destination hash must not be zero.
//! 6. Total emissions <= `CDUG_MAX_EMISSIONS`.
//!
//! Tests **CDUG-01..10**. Error enum [`DestUniformityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DEST-UNIFORMITY`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Minimum cover emissions per destination.
pub const CDUG_MIN_PER_DEST: usize = 5;

/// Maximum ratio numerator for any single destination.
pub const CDUG_MAX_RATIO_NUM: usize = 3;

/// Maximum ratio denominator.
pub const CDUG_MAX_RATIO_DEN: usize = 4;

/// Minimum total emissions.
pub const CDUG_MIN_EMISSIONS: usize = 20;

/// Maximum total emissions.
pub const CDUG_MAX_EMISSIONS: usize = 1_000_000;

/// Destination hash length.
pub const CDUG_DEST_LEN: usize = 16;

/// Valid destination set (for validation).
pub const CDUG_NUM_DESTINATIONS: usize = 4;

/// A cover emission directed at a destination.
#[derive(Debug, Clone)]
pub struct DestCoverEmission {
    /// Destination hash.
    pub dest: [u8; CDUG_DEST_LEN],
}

/// All ways destination uniformity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DestUniformityError {
    /// Below minimum per destination.
    BelowMin { dest_idx: usize, count: usize, min: usize },
    /// Dominant destination.
    Dominant { count: usize, total: usize },
    /// Too few emissions.
    TooFew { got: usize, min: usize },
    /// Destination not in declared set.
    UnknownDest(usize),
    /// Zero destination hash.
    ZeroDest(usize),
    /// Too many emissions.
    TooMany,
}

/// `[VERIFIED]` Validate cover traffic destination uniformity.
pub fn validate_dest_uniformity(
    declared_dests: &[[u8; CDUG_DEST_LEN]],
    emissions: &[DestCoverEmission],
) -> Result<(), DestUniformityError> {
    if emissions.len() > CDUG_MAX_EMISSIONS {
        return Err(DestUniformityError::TooMany);
    }
    if emissions.len() < CDUG_MIN_EMISSIONS {
        return Err(DestUniformityError::TooFew {
            got: emissions.len(),
            min: CDUG_MIN_EMISSIONS,
        });
    }
    let valid: BTreeSet<[u8; CDUG_DEST_LEN]> = declared_dests.iter().copied().collect();
    let mut counts: BTreeMap<[u8; CDUG_DEST_LEN], usize> = BTreeMap::new();
    for (i, e) in emissions.iter().enumerate() {
        if e.dest == [0u8; CDUG_DEST_LEN] {
            return Err(DestUniformityError::ZeroDest(i));
        }
        if !valid.contains(&e.dest) {
            return Err(DestUniformityError::UnknownDest(i));
        }
        *counts.entry(e.dest).or_insert(0) += 1;
    }
    let total = emissions.len();
    let threshold = total / CDUG_MAX_RATIO_DEN;
    for (&dest, &count) in &counts {
        if count < CDUG_MIN_PER_DEST {
            let idx = declared_dests.iter().position(|d| *d == dest).unwrap_or(0);
            return Err(DestUniformityError::BelowMin {
                dest_idx: idx,
                count,
                min: CDUG_MIN_PER_DEST,
            });
        }
        if count > threshold * CDUG_MAX_RATIO_NUM {
            return Err(DestUniformityError::Dominant { count, total });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dest(byte: u8) -> [u8; CDUG_DEST_LEN] {
        [byte; CDUG_DEST_LEN]
    }

    fn emission(byte: u8) -> DestCoverEmission {
        DestCoverEmission { dest: dest(byte) }
    }

    fn declared() -> Vec<[u8; CDUG_DEST_LEN]> {
        vec![dest(0x01), dest(0x02), dest(0x03), dest(0x04)]
    }

    fn balanced() -> Vec<DestCoverEmission> {
        let mut v = Vec::new();
        for _ in 0..10 {
            for b in &[0x01u8, 0x02, 0x03, 0x04] {
                v.push(emission(*b));
            }
        }
        v
    }

    /// **CDUG-01** — below min per dest rejected.
    #[test]
    fn cdug_01_below_min_rejected() {
        let mut v = Vec::new();
        for _ in 0..20 { v.push(emission(0x01)); }
        for &b in &[0x02u8, 0x03, 0x04] { for _ in 0..4 { v.push(emission(b)); } }
        assert!(matches!(
            validate_dest_uniformity(&declared(), &v),
            Err(DestUniformityError::BelowMin { .. })
        ));
    }

    /// **CDUG-02** — dominant rejected.
    #[test]
    fn cdug_02_dominant_rejected() {
        let mut v = Vec::new();
        for _ in 0..80 { v.push(emission(0x01)); }
        for &b in &[0x02u8, 0x03, 0x04] { for _ in 0..5 { v.push(emission(b)); } }
        assert!(matches!(
            validate_dest_uniformity(&declared(), &v),
            Err(DestUniformityError::Dominant { .. })
        ));
    }

    /// **CDUG-03** — too few rejected.
    #[test]
    fn cdug_03_too_few_rejected() {
        let v: Vec<DestCoverEmission> = declared().iter().map(|&d| DestCoverEmission { dest: d }).collect();
        assert_eq!(
            validate_dest_uniformity(&declared(), &v),
            Err(DestUniformityError::TooFew { got: 4, min: 20 })
        );
    }

    /// **CDUG-04** — unknown dest rejected.
    #[test]
    fn cdug_04_unknown_rejected() {
        let mut v = balanced();
        v.push(emission(0x99));
        assert_eq!(
            validate_dest_uniformity(&declared(), &v),
            Err(DestUniformityError::UnknownDest(40))
        );
    }

    /// **CDUG-05** — zero dest rejected.
    #[test]
    fn cdug_05_zero_rejected() {
        let mut v = balanced();
        v.push(DestCoverEmission { dest: [0u8; CDUG_DEST_LEN] });
        assert_eq!(
            validate_dest_uniformity(&declared(), &v),
            Err(DestUniformityError::ZeroDest(40))
        );
    }

    /// **CDUG-06** — too many rejected.
    #[test]
    fn cdug_06_too_many_rejected() {
        let v: Vec<DestCoverEmission> = (0..=CDUG_MAX_EMISSIONS)
            .map(|i| DestCoverEmission { dest: dest((i as u8) % 4 + 1) })
            .collect();
        assert_eq!(validate_dest_uniformity(&declared(), &v), Err(DestUniformityError::TooMany));
    }

    /// **CDUG-07** — balanced accepted.
    #[test]
    fn cdug_07_balanced_accepted() {
        assert_eq!(validate_dest_uniformity(&declared(), &balanced()), Ok(()));
    }

    /// **CDUG-08** — min boundary accepted.
    #[test]
    fn cdug_08_min_boundary_accepted() {
        let mut v = Vec::new();
        for _ in 0..CDUG_MIN_PER_DEST {
            for &b in &[0x01u8, 0x02, 0x03, 0x04] { v.push(emission(b)); }
        }
        assert_eq!(validate_dest_uniformity(&declared(), &v), Ok(()));
    }

    /// **CDUG-09** — slightly imbalanced accepted.
    #[test]
    fn cdug_09_slightly_imbalanced_accepted() {
        let mut v = Vec::new();
        for _ in 0..12 { v.push(emission(0x01)); }
        for &b in &[0x02u8, 0x03, 0x04] { for _ in 0..8 { v.push(emission(b)); } }
        assert_eq!(validate_dest_uniformity(&declared(), &v), Ok(()));
    }

    /// **CDUG-10** — uniform accepted.
    #[test]
    fn cdug_10_uniform_accepted() {
        let mut v = Vec::new();
        for _ in 0..25 {
            for &b in &[0x01u8, 0x02, 0x03, 0x04] { v.push(emission(b)); }
        }
        assert_eq!(validate_dest_uniformity(&declared(), &v), Ok(()));
    }
}
