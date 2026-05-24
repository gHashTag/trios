//! # CR-CHAT-07 — Cover traffic size distribution guard (Wave-155 Lane B)
//!
//! ANTI-CORRELATION — cover traffic payloads must match the size
//! distribution of real messages; divergent distributions leak
//! cover vs real.
//!
//! When cover traffic is generated, its size distribution must be
//! statistically indistinguishable from real traffic. If distributions
//! diverge:
//!
//! * **Cover detection** — an observer can distinguish cover from
//!   real messages based on size distribution differences.
//! * **Volume fingerprinting** — consistent size differences create
//!   a fingerprint for the user's real vs cover traffic.
//! * **Leakage amplification** — over time, distribution differences
//!   compound, increasing the observer's confidence.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Kolmogorov-Smirnov-like distance <= `CTSD_MAX_DISTANCE`.
//! 2. No duplicate observation IDs.
//! 3. Observation ID must not be zero.
//! 4. Size must be > 0.
//! 5. At least `CTSD_MIN_OBS` observations.
//! 6. Batch size <= `CTSD_MAX_OBS`.
//!
//! Tests **CTSD-01..10**. Error enum [`SizeDistributionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIZE-INDISTINGUISHABLE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum observations per batch.
pub const CTSD_MAX_OBS: usize = 4096;

/// Minimum observations required.
pub const CTSD_MIN_OBS: usize = 16;

/// Maximum KS-like distance (scaled by 10000).
pub const CTSD_MAX_DISTANCE: u64 = 2000;

/// Observation ID length.
pub const CTSD_OBS_ID_LEN: usize = 16;

/// A size distribution observation.
#[derive(Debug, Clone)]
pub struct SizeObservation {
    /// Observation identifier.
    pub obs_id: [u8; CTSD_OBS_ID_LEN],
    /// Size in bytes.
    pub size: usize,
    /// Whether this is a real message (true) or cover (false).
    pub is_real: bool,
}

/// All ways size distribution validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SizeDistributionError {
    /// Distribution distance too large.
    DistanceTooLarge {
        distance: u64,
        max: u64,
    },
    /// Duplicate observation ID.
    DuplicateId {
        idx: usize,
    },
    /// Zero observation ID.
    ZeroId(usize),
    /// Zero size.
    ZeroSize(usize),
    /// Too few observations.
    TooFew {
        got: usize,
        min: usize,
    },
    /// Too many observations.
    TooMany {
        got: usize,
        max: usize,
    },
}

/// `[VERIFIED]` Validate cover traffic size distribution.
pub fn validate_size_distribution(
    obs: &[SizeObservation],
) -> Result<(), SizeDistributionError> {
    if obs.len() > CTSD_MAX_OBS {
        return Err(SizeDistributionError::TooMany {
            got: obs.len(),
            max: CTSD_MAX_OBS,
        });
    }
    if obs.len() < CTSD_MIN_OBS {
        return Err(SizeDistributionError::TooFew {
            got: obs.len(),
            min: CTSD_MIN_OBS,
        });
    }
    let mut seen: BTreeSet<[u8; CTSD_OBS_ID_LEN]> = BTreeSet::new();
    let mut real_sizes: Vec<usize> = Vec::new();
    let mut cover_sizes: Vec<usize> = Vec::new();
    for (i, o) in obs.iter().enumerate() {
        if o.obs_id == [0u8; CTSD_OBS_ID_LEN] {
            return Err(SizeDistributionError::ZeroId(i));
        }
        if !seen.insert(o.obs_id) {
            return Err(SizeDistributionError::DuplicateId { idx: i });
        }
        if o.size == 0 {
            return Err(SizeDistributionError::ZeroSize(i));
        }
        if o.is_real {
            real_sizes.push(o.size);
        } else {
            cover_sizes.push(o.size);
        }
    }
    if real_sizes.is_empty() || cover_sizes.is_empty() {
        return Ok(());
    }
    let distance = compute_ks_distance(&real_sizes, &cover_sizes);
    if distance > CTSD_MAX_DISTANCE {
        return Err(SizeDistributionError::DistanceTooLarge {
            distance,
            max: CTSD_MAX_DISTANCE,
        });
    }
    Ok(())
}

fn compute_ks_distance(a: &[usize], b: &[usize]) -> u64 {
    let mut all_sizes: Vec<usize> = a.iter().chain(b.iter()).copied().collect();
    all_sizes.sort();
    all_sizes.dedup();
    let n_a = a.len() as u64;
    let n_b = b.len() as u64;
    let mut max_diff: u64 = 0;
    for &s in &all_sizes {
        let fa = a.iter().filter(|&&x| x <= s).count() as u64;
        let fb = b.iter().filter(|&&x| x <= s).count() as u64;
        let diff = if fa * n_b > fb * n_a {
            fa * n_b - fb * n_a
        } else {
            fb * n_a - fa * n_b
        };
        let scaled = (diff * 10000) / (n_a * n_b);
        if scaled > max_diff {
            max_diff = scaled;
        }
    }
    max_diff
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; CTSD_OBS_ID_LEN] {
        [byte; CTSD_OBS_ID_LEN]
    }

    fn obs(id: u8, size: usize, real: bool) -> SizeObservation {
        SizeObservation { obs_id: oid(id), size, is_real: real }
    }

    fn matched_obs() -> Vec<SizeObservation> {
        let mut v = Vec::new();
        for i in 0..16u8 {
            v.push(obs(i + 1, 256, true));
            v.push(obs(i + 17, 256, false));
        }
        v
    }

    /// **CTSD-01** — distance too large rejected.
    #[test]
    fn ctsd_01_distance_too_large_rejected() {
        let mut v = Vec::new();
        for i in 0..16u8 {
            v.push(obs(i + 1, 256, true));
        }
        for i in 0..16u8 {
            v.push(obs(i + 17, 16384, false));
        }
        let r = validate_size_distribution(&v);
        assert!(matches!(r, Err(SizeDistributionError::DistanceTooLarge { .. })));
    }

    /// **CTSD-02** — duplicate ID rejected.
    #[test]
    fn ctsd_02_duplicate_rejected() {
        let mut v = matched_obs();
        v.push(obs(1, 256, false));
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistributionError::DuplicateId { idx: 32 })
        );
    }

    /// **CTSD-03** — zero ID rejected.
    #[test]
    fn ctsd_03_zero_id_rejected() {
        let mut v = matched_obs();
        v[0].obs_id = [0u8; CTSD_OBS_ID_LEN];
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistributionError::ZeroId(0))
        );
    }

    /// **CTSD-04** — zero size rejected.
    #[test]
    fn ctsd_04_zero_size_rejected() {
        let mut v = matched_obs();
        v[0].size = 0;
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistributionError::ZeroSize(0))
        );
    }

    /// **CTSD-05** — too few rejected.
    #[test]
    fn ctsd_05_too_few_rejected() {
        let v: Vec<SizeObservation> = (0..5u8)
            .map(|i| obs(i + 1, 256, i % 2 == 0))
            .collect();
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistributionError::TooFew { got: 5, min: CTSD_MIN_OBS })
        );
    }

    /// **CTSD-06** — too many rejected.
    #[test]
    fn ctsd_06_too_many_rejected() {
        let v: Vec<SizeObservation> = (0..=CTSD_MAX_OBS)
            .map(|i| {
                let mut id = [0u8; CTSD_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                SizeObservation { obs_id: id, size: 256, is_real: i % 2 == 0 }
            })
            .collect();
        assert_eq!(
            validate_size_distribution(&v),
            Err(SizeDistributionError::TooMany {
                got: CTSD_MAX_OBS + 1,
                max: CTSD_MAX_OBS,
            })
        );
    }

    /// **CTSD-07** — valid matched accepted.
    #[test]
    fn ctsd_07_valid_accepted() {
        assert_eq!(validate_size_distribution(&matched_obs()), Ok(()));
    }

    /// **CTSD-08** — empty rejected.
    #[test]
    fn ctsd_08_empty_rejected() {
        assert_eq!(
            validate_size_distribution(&[]),
            Err(SizeDistributionError::TooFew { got: 0, min: CTSD_MIN_OBS })
        );
    }

    /// **CTSD-09** — only real accepted (no cover to compare).
    #[test]
    fn ctsd_09_only_real_accepted() {
        let v: Vec<SizeObservation> = (0..20u8)
            .map(|i| obs(i + 1, 256, true))
            .collect();
        assert_eq!(validate_size_distribution(&v), Ok(()));
    }

    /// **CTSD-10** — many matched accepted.
    #[test]
    fn ctsd_10_many_matched_accepted() {
        let mut v = Vec::new();
        for i in 0..50u8 {
            v.push(obs(i + 1, 1024, true));
            v.push(obs(i + 51, 1024, false));
        }
        assert_eq!(validate_size_distribution(&v), Ok(()));
    }
}
