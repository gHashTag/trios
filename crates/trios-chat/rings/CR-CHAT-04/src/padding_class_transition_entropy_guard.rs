//! # CR-CHAT-04 — Padding class transition entropy guard (Wave-154 Lane B)
//!
//! PADDING — transitions between padding classes must be diverse;
//! monotonous transitions leak message sizes.
//!
//! When messages are padded to different size classes, the sequence of
//! class transitions must be sufficiently diverse. If transitions are
//! monotonous:
//!
//! * **Size correlation** — always transitioning to the same class
//!   reveals the actual message size distribution.
//! * **Pattern detection** — an observer can identify when a user's
//!   behavior changes based on class transition patterns.
//! * **Fingerprinting** — a narrow set of transitions creates a
//!   unique fingerprint for the user's communication.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Number of distinct transition types >= `PCTE_MIN_TRANSITIONS`.
//! 2. No duplicate observation IDs.
//! 3. Observation ID must not be zero.
//! 4. From-class must not equal to-class (no self-transitions).
//! 5. At least `PCTE_MIN_OBS` observations.
//! 6. Batch size <= `PCTE_MAX_OBS`.
//!
//! Tests **PCTE-01..10**. Error enum [`TransitionEntropyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CLASS-ENTROPY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum observations per batch.
pub const PCTE_MAX_OBS: usize = 4096;

/// Minimum observations required.
pub const PCTE_MIN_OBS: usize = 8;

/// Minimum distinct transition types.
pub const PCTE_MIN_TRANSITIONS: usize = 3;

/// Observation ID length.
pub const PCTE_OBS_ID_LEN: usize = 16;

/// A padding class transition observation.
#[derive(Debug, Clone)]
pub struct TransitionObservation {
    /// Observation identifier.
    pub obs_id: [u8; PCTE_OBS_ID_LEN],
    /// Source padding class index.
    pub from_class: u8,
    /// Destination padding class index.
    pub to_class: u8,
}

/// All ways transition entropy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionEntropyError {
    /// Too few distinct transitions.
    LowDiversity {
        distinct: usize,
        min: usize,
    },
    /// Duplicate observation ID.
    DuplicateId {
        idx: usize,
    },
    /// Zero observation ID.
    ZeroId(usize),
    /// Self-transition (from == to).
    SelfTransition(usize),
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

/// `[VERIFIED]` Validate padding class transition entropy.
pub fn validate_transition_entropy(
    obs: &[TransitionObservation],
) -> Result<(), TransitionEntropyError> {
    if obs.len() > PCTE_MAX_OBS {
        return Err(TransitionEntropyError::TooMany {
            got: obs.len(),
            max: PCTE_MAX_OBS,
        });
    }
    if obs.len() < PCTE_MIN_OBS {
        return Err(TransitionEntropyError::TooFew {
            got: obs.len(),
            min: PCTE_MIN_OBS,
        });
    }
    let mut seen_ids: BTreeSet<[u8; PCTE_OBS_ID_LEN]> = BTreeSet::new();
    let mut transitions: BTreeSet<(u8, u8)> = BTreeSet::new();
    for (i, o) in obs.iter().enumerate() {
        if o.obs_id == [0u8; PCTE_OBS_ID_LEN] {
            return Err(TransitionEntropyError::ZeroId(i));
        }
        if !seen_ids.insert(o.obs_id) {
            return Err(TransitionEntropyError::DuplicateId { idx: i });
        }
        if o.from_class == o.to_class {
            return Err(TransitionEntropyError::SelfTransition(i));
        }
        transitions.insert((o.from_class, o.to_class));
    }
    if transitions.len() < PCTE_MIN_TRANSITIONS {
        return Err(TransitionEntropyError::LowDiversity {
            distinct: transitions.len(),
            min: PCTE_MIN_TRANSITIONS,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> [u8; PCTE_OBS_ID_LEN] {
        [byte; PCTE_OBS_ID_LEN]
    }

    fn obs(id: u8, from: u8, to: u8) -> TransitionObservation {
        TransitionObservation { obs_id: oid(id), from_class: from, to_class: to }
    }

    fn diverse_obs() -> Vec<TransitionObservation> {
        vec![
            obs(0x01, 0, 1),
            obs(0x02, 0, 2),
            obs(0x03, 0, 3),
            obs(0x04, 1, 0),
            obs(0x05, 1, 2),
            obs(0x06, 2, 0),
            obs(0x07, 2, 3),
            obs(0x08, 3, 1),
        ]
    }

    /// **PCTE-01** — low diversity rejected.
    #[test]
    fn pcte_01_low_diversity_rejected() {
        let os: Vec<TransitionObservation> = (0..10u8)
            .map(|i| obs(i + 1, 0, 1))
            .collect();
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::LowDiversity { distinct: 1, min: PCTE_MIN_TRANSITIONS })
        );
    }

    /// **PCTE-02** — duplicate ID rejected.
    #[test]
    fn pcte_02_duplicate_rejected() {
        let mut os = diverse_obs();
        os[7] = obs(0x01, 2, 3);
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::DuplicateId { idx: 7 })
        );
    }

    /// **PCTE-03** — zero ID rejected.
    #[test]
    fn pcte_03_zero_id_rejected() {
        let mut os = diverse_obs();
        os[0].obs_id = [0u8; PCTE_OBS_ID_LEN];
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::ZeroId(0))
        );
    }

    /// **PCTE-04** — self-transition rejected.
    #[test]
    fn pcte_04_self_transition_rejected() {
        let mut os = diverse_obs();
        os[0] = obs(0x01, 2, 2);
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::SelfTransition(0))
        );
    }

    /// **PCTE-05** — too few rejected.
    #[test]
    fn pcte_05_too_few_rejected() {
        let os: Vec<TransitionObservation> = (0..5u8)
            .map(|i| obs(i + 1, 0, (i % 3) + 1))
            .collect();
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::TooFew { got: 5, min: PCTE_MIN_OBS })
        );
    }

    /// **PCTE-06** — too many rejected.
    #[test]
    fn pcte_06_too_many_rejected() {
        let os: Vec<TransitionObservation> = (0..=PCTE_MAX_OBS)
            .map(|i| {
                let mut id = [0u8; PCTE_OBS_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let from = ((i as u64) % 4) as u8;
                let to = (((i as u64) + 1) % 4) as u8;
                TransitionObservation { obs_id: id, from_class: from, to_class: to }
            })
            .collect();
        assert_eq!(
            validate_transition_entropy(&os),
            Err(TransitionEntropyError::TooMany {
                got: PCTE_MAX_OBS + 1,
                max: PCTE_MAX_OBS,
            })
        );
    }

    /// **PCTE-07** — valid accepted.
    #[test]
    fn pcte_07_valid_accepted() {
        assert_eq!(validate_transition_entropy(&diverse_obs()), Ok(()));
    }

    /// **PCTE-08** — empty rejected.
    #[test]
    fn pcte_08_empty_rejected() {
        assert_eq!(
            validate_transition_entropy(&[]),
            Err(TransitionEntropyError::TooFew { got: 0, min: PCTE_MIN_OBS })
        );
    }

    /// **PCTE-09** — exact minimum transitions accepted.
    #[test]
    fn pcte_09_exact_min_transitions_accepted() {
        let os: Vec<TransitionObservation> = (0..10u8)
            .map(|i| obs(i + 1, i % 4, (i + 1) % 4))
            .collect();
        assert_eq!(validate_transition_entropy(&os), Ok(()));
    }

    /// **PCTE-10** — many diverse accepted.
    #[test]
    fn pcte_10_many_diverse_accepted() {
        let os: Vec<TransitionObservation> = (0..100u8)
            .map(|i| {
                let mut id = [0u8; PCTE_OBS_ID_LEN];
                id[0] = i + 1;
                let from = i % 4;
                let to = (from + 1 + (i / 17) % 3) % 4;
                TransitionObservation { obs_id: id, from_class: from, to_class: to }
            })
            .collect();
        assert_eq!(validate_transition_entropy(&os), Ok(()));
    }
}
