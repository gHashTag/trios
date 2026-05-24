//! # CR-CHAT-03 — TreeKEM epoch transition integrity guard (Wave-102 Lane A)
//!
//! RATCHET TREE — epoch transitions must produce valid states.
//!
//! In TreeKEM, each epoch transition rekeys the tree. If the transition
//! is corrupted or incomplete:
//!
//! * **TreeKEM inconsistency** — nodes derive different shared secrets
//!   from the same epoch, causing message decryption failures.
//! * **State fork** — two members end up with different tree hashes for
//!   the same epoch, permanently splitting the group.
//! * **Silent downgrade** — a malicious member proposes a transition
//!   that weakens the tree (e.g., reduces tree depth to leak membership
//!   size).
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Epoch numbers must be strictly increasing.
//! 2. Tree hash must change between epochs.
//! 3. Epoch number must not be zero.
//! 4. Tree depth must be <= `TETI_MAX_DEPTH`.
//! 5. No duplicate tree hashes across epochs.
//! 6. Total transitions <= `TETI_MAX_TRANSITIONS`.
//!
//! Tests **TETI-01..10**. Error enum [`EpochTransitionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-INTEGRITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum tree depth.
pub const TETI_MAX_DEPTH: u32 = 32;

/// Maximum transitions per batch.
pub const TETI_MAX_TRANSITIONS: usize = 1024;

/// Tree hash length.
pub const TETI_HASH_LEN: usize = 32;

/// An epoch transition record.
#[derive(Debug, Clone)]
pub struct EpochTransition {
    /// Epoch number.
    pub epoch: u64,
    /// Tree hash after transition.
    pub tree_hash: [u8; TETI_HASH_LEN],
    /// Tree depth after transition.
    pub tree_depth: u32,
}

/// All ways epoch transition validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochTransitionError {
    /// Epoch not increasing.
    NotIncreasing { idx: usize, prev: u64, current: u64 },
    /// Tree hash unchanged.
    HashUnchanged { idx: usize },
    /// Zero epoch.
    ZeroEpoch(usize),
    /// Tree depth exceeded.
    DepthExceeded { idx: usize, depth: u32, max: u32 },
    /// Duplicate tree hash.
    DuplicateHash(usize),
    /// Too many transitions.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate TreeKEM epoch transition integrity.
pub fn validate_epoch_transitions(
    transitions: &[EpochTransition],
) -> Result<(), EpochTransitionError> {
    if transitions.len() > TETI_MAX_TRANSITIONS {
        return Err(EpochTransitionError::TooMany {
            got: transitions.len(),
            max: TETI_MAX_TRANSITIONS,
        });
    }
    let mut seen: BTreeSet<[u8; TETI_HASH_LEN]> = BTreeSet::new();
    let mut prev_epoch: u64 = 0;
    for (i, t) in transitions.iter().enumerate() {
        if t.epoch == 0 {
            return Err(EpochTransitionError::ZeroEpoch(i));
        }
        if t.tree_depth > TETI_MAX_DEPTH {
            return Err(EpochTransitionError::DepthExceeded {
                idx: i,
                depth: t.tree_depth,
                max: TETI_MAX_DEPTH,
            });
        }
        if i > 0 {
            if t.epoch <= prev_epoch {
                return Err(EpochTransitionError::NotIncreasing {
                    idx: i,
                    prev: prev_epoch,
                    current: t.epoch,
                });
            }
            if t.tree_hash == transitions[i - 1].tree_hash {
                return Err(EpochTransitionError::HashUnchanged { idx: i });
            }
        }
        if !seen.insert(t.tree_hash) {
            return Err(EpochTransitionError::DuplicateHash(i));
        }
        prev_epoch = t.epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; TETI_HASH_LEN] {
        [byte; TETI_HASH_LEN]
    }

    fn transition(epoch: u64, hash_byte: u8, depth: u32) -> EpochTransition {
        EpochTransition { epoch, tree_hash: hash(hash_byte), tree_depth: depth }
    }

    fn valid_transitions() -> Vec<EpochTransition> {
        vec![
            transition(1, 0x01, 4),
            transition(2, 0x02, 4),
            transition(3, 0x03, 5),
        ]
    }

    /// **TETI-01** — not increasing rejected.
    #[test]
    fn teti_01_not_increasing_rejected() {
        let ts = vec![transition(5, 0x01, 4), transition(3, 0x02, 4)];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::NotIncreasing {
                idx: 1,
                prev: 5,
                current: 3,
            })
        );
    }

    /// **TETI-02** — hash unchanged rejected.
    #[test]
    fn teti_02_hash_unchanged_rejected() {
        let ts = vec![transition(1, 0xAA, 4), transition(2, 0xAA, 4)];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::HashUnchanged { idx: 1 })
        );
    }

    /// **TETI-03** — zero epoch rejected.
    #[test]
    fn teti_03_zero_epoch_rejected() {
        let t = EpochTransition { epoch: 0, tree_hash: hash(0x01), tree_depth: 4 };
        assert_eq!(
            validate_epoch_transitions(&[t]),
            Err(EpochTransitionError::ZeroEpoch(0))
        );
    }

    /// **TETI-04** — depth exceeded rejected.
    #[test]
    fn teti_04_depth_exceeded_rejected() {
        let t = EpochTransition { epoch: 1, tree_hash: hash(0x01), tree_depth: TETI_MAX_DEPTH + 1 };
        assert_eq!(
            validate_epoch_transitions(&[t]),
            Err(EpochTransitionError::DepthExceeded {
                idx: 0,
                depth: TETI_MAX_DEPTH + 1,
                max: TETI_MAX_DEPTH,
            })
        );
    }

    /// **TETI-05** — duplicate hash rejected.
    #[test]
    fn teti_05_duplicate_hash_rejected() {
        let ts = vec![
            transition(1, 0xAA, 4),
            transition(2, 0xBB, 4),
            transition(3, 0xAA, 5),
        ];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::DuplicateHash(2))
        );
    }

    /// **TETI-06** — too many rejected.
    #[test]
    fn teti_06_too_many_rejected() {
        let ts: Vec<EpochTransition> = (0..=TETI_MAX_TRANSITIONS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                EpochTransition {
                    epoch: (i as u64) + 1,
                    tree_hash: hash(b),
                    tree_depth: 4,
                }
            })
            .collect();
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::TooMany {
                got: TETI_MAX_TRANSITIONS + 1,
                max: TETI_MAX_TRANSITIONS,
            })
        );
    }

    /// **TETI-07** — valid accepted.
    #[test]
    fn teti_07_valid_accepted() {
        assert_eq!(validate_epoch_transitions(&valid_transitions()), Ok(()));
    }

    /// **TETI-08** — empty accepted.
    #[test]
    fn teti_08_empty_accepted() {
        assert_eq!(validate_epoch_transitions(&[]), Ok(()));
    }

    /// **TETI-09** — single accepted.
    #[test]
    fn teti_09_single_accepted() {
        let ts = vec![transition(1, 0x01, 4)];
        assert_eq!(validate_epoch_transitions(&ts), Ok(()));
    }

    /// **TETI-10** — max depth boundary accepted.
    #[test]
    fn teti_10_max_depth_accepted() {
        let ts = vec![transition(1, 0x01, TETI_MAX_DEPTH)];
        assert_eq!(validate_epoch_transitions(&ts), Ok(()));
    }
}
