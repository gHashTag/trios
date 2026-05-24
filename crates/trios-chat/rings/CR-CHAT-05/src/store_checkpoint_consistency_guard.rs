//! # CR-CHAT-05 — Store checkpoint consistency guard (Wave-104 Lane B)
//!
//! PERSISTENCE — periodic checkpoints must be consistent with store state.
//!
//! The persistence layer periodically writes checkpoints (snapshots of
//! the current store state) for crash recovery. If a checkpoint is
//! inconsistent with the actual store state:
//!
//! * **Data loss on crash** — recovering from an inconsistent checkpoint
//!   silently drops records that were committed after the checkpoint.
//! * **Silent corruption** — a checkpoint that includes records not yet
//!   committed to the store causes phantom data to appear after recovery.
//! * **Liveness violation** — if the checkpoint counter exceeds the
//!   actual store counter, the system believes it has processed more
//!   messages than it actually has.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Checkpoint counter <= actual store counter.
//! 2. Checkpoint counter must be > 0.
//! 3. Checkpoint hash must not be zero.
//! 4. Checkpoints must be strictly increasing by counter.
//! 5. No duplicate checkpoint counters.
//! 6. Total checkpoints <= `SCCG_MAX_CHECKPOINTS`.
//!
//! Tests **SCCG-01..10**. Error enum [`CheckpointError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHECKPOINT-CONSISTENCY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum checkpoints per batch.
pub const SCCG_MAX_CHECKPOINTS: usize = 1024;

/// Checkpoint hash length.
pub const SCCG_HASH_LEN: usize = 32;

/// A checkpoint record.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Checkpoint sequence counter.
    pub checkpoint_counter: u64,
    /// Store counter at checkpoint time.
    pub store_counter: u64,
    /// Hash of the store state at checkpoint time.
    pub state_hash: [u8; SCCG_HASH_LEN],
}

/// All ways checkpoint consistency validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CheckpointError {
    /// Checkpoint exceeds store counter.
    ExceedsStore {
        /// Index of the offending checkpoint.
        idx: usize,
        /// Checkpoint counter value.
        checkpoint: u64,
        /// Store counter value.
        store: u64,
    },
    /// Zero checkpoint counter.
    ZeroCounter(usize),
    /// Zero hash.
    ZeroHash(usize),
    /// Not strictly increasing.
    NotIncreasing {
        /// Index of the offending checkpoint.
        idx: usize,
        /// Previous checkpoint counter.
        prev: u64,
        /// Current checkpoint counter.
        current: u64,
    },
    /// Duplicate counter.
    DuplicateCounter(usize),
    /// Too many checkpoints.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store checkpoint consistency.
pub fn validate_checkpoints(
    checkpoints: &[Checkpoint],
) -> Result<(), CheckpointError> {
    if checkpoints.len() > SCCG_MAX_CHECKPOINTS {
        return Err(CheckpointError::TooMany {
            got: checkpoints.len(),
            max: SCCG_MAX_CHECKPOINTS,
        });
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    let mut prev: u64 = 0;
    for (i, c) in checkpoints.iter().enumerate() {
        if c.checkpoint_counter == 0 {
            return Err(CheckpointError::ZeroCounter(i));
        }
        if c.state_hash == [0u8; SCCG_HASH_LEN] {
            return Err(CheckpointError::ZeroHash(i));
        }
        if c.checkpoint_counter > c.store_counter {
            return Err(CheckpointError::ExceedsStore {
                idx: i,
                checkpoint: c.checkpoint_counter,
                store: c.store_counter,
            });
        }
        if !seen.insert(c.checkpoint_counter) {
            return Err(CheckpointError::DuplicateCounter(i));
        }
        if i > 0 && c.checkpoint_counter <= prev {
            return Err(CheckpointError::NotIncreasing {
                idx: i,
                prev,
                current: c.checkpoint_counter,
            });
        }
        prev = c.checkpoint_counter;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; SCCG_HASH_LEN] {
        [byte; SCCG_HASH_LEN]
    }

    fn checkpoint(cp: u64, store: u64, hash_byte: u8) -> Checkpoint {
        Checkpoint { checkpoint_counter: cp, store_counter: store, state_hash: hash(hash_byte) }
    }

    fn valid_checkpoints() -> Vec<Checkpoint> {
        vec![
            checkpoint(10, 100, 0x01),
            checkpoint(20, 150, 0x02),
            checkpoint(30, 200, 0x03),
        ]
    }

    /// **SCCG-01** — exceeds store counter rejected.
    #[test]
    fn sccg_01_exceeds_store_rejected() {
        let cs = vec![checkpoint(50, 30, 0x01)];
        assert_eq!(
            validate_checkpoints(&cs),
            Err(CheckpointError::ExceedsStore {
                idx: 0,
                checkpoint: 50,
                store: 30,
            })
        );
    }

    /// **SCCG-02** — zero counter rejected.
    #[test]
    fn sccg_02_zero_counter_rejected() {
        let c = Checkpoint { checkpoint_counter: 0, store_counter: 100, state_hash: hash(0x01) };
        assert_eq!(
            validate_checkpoints(&[c]),
            Err(CheckpointError::ZeroCounter(0))
        );
    }

    /// **SCCG-03** — zero hash rejected.
    #[test]
    fn sccg_03_zero_hash_rejected() {
        let c = Checkpoint { checkpoint_counter: 1, store_counter: 100, state_hash: [0u8; SCCG_HASH_LEN] };
        assert_eq!(
            validate_checkpoints(&[c]),
            Err(CheckpointError::ZeroHash(0))
        );
    }

    /// **SCCG-04** — not increasing rejected.
    #[test]
    fn sccg_04_not_increasing_rejected() {
        let cs = vec![checkpoint(50, 100, 0x01), checkpoint(30, 120, 0x02)];
        assert_eq!(
            validate_checkpoints(&cs),
            Err(CheckpointError::NotIncreasing {
                idx: 1,
                prev: 50,
                current: 30,
            })
        );
    }

    /// **SCCG-05** — duplicate counter rejected.
    #[test]
    fn sccg_05_duplicate_rejected() {
        let cs = vec![checkpoint(10, 100, 0x01), checkpoint(10, 150, 0x02)];
        assert_eq!(
            validate_checkpoints(&cs),
            Err(CheckpointError::DuplicateCounter(1))
        );
    }

    /// **SCCG-06** — too many rejected.
    #[test]
    fn sccg_06_too_many_rejected() {
        let cs: Vec<Checkpoint> = (0..=SCCG_MAX_CHECKPOINTS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                Checkpoint {
                    checkpoint_counter: (i as u64) + 1,
                    store_counter: (i as u64) * 10 + 100,
                    state_hash: hash(b),
                }
            })
            .collect();
        assert_eq!(
            validate_checkpoints(&cs),
            Err(CheckpointError::TooMany {
                got: SCCG_MAX_CHECKPOINTS + 1,
                max: SCCG_MAX_CHECKPOINTS,
            })
        );
    }

    /// **SCCG-07** — valid accepted.
    #[test]
    fn sccg_07_valid_accepted() {
        assert_eq!(validate_checkpoints(&valid_checkpoints()), Ok(()));
    }

    /// **SCCG-08** — empty accepted.
    #[test]
    fn sccg_08_empty_accepted() {
        assert_eq!(validate_checkpoints(&[]), Ok(()));
    }

    /// **SCCG-09** — checkpoint equals store counter accepted.
    #[test]
    fn sccg_09_equals_store_accepted() {
        let cs = vec![checkpoint(100, 100, 0x01)];
        assert_eq!(validate_checkpoints(&cs), Ok(()));
    }

    /// **SCCG-10** — single accepted.
    #[test]
    fn sccg_10_single_accepted() {
        let cs = vec![checkpoint(1, 50, 0xAA)];
        assert_eq!(validate_checkpoints(&cs), Ok(()));
    }
}
