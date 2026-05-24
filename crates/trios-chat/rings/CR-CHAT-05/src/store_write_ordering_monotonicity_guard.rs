//! # CR-CHAT-05 — Store write ordering monotonicity guard (Wave-107 Lane B)
//!
//! PERSISTENCE — store writes must be monotonically ordered.
//!
//! The persistence layer expects writes to arrive in strictly
//! monotonic counter order. Out-of-order writes indicate:
//!
//! * **Concurrency bug** — two threads writing to the same session
//!   without proper synchronization.
//! * **Replay attack** — an adversary re-sends an old write after a
//!   newer one has been committed.
//! * **Network reordering** — messages delivered out of order by the
//!   mesh transport layer.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Counters must be strictly increasing.
//! 2. Counter must not be zero.
//! 3. Session ID must not be zero.
//! 4. No duplicate counters within a session.
//! 5. Write size must be >= `SWOM_MIN_SIZE`.
//! 6. Total writes <= `SWOM_MAX_WRITES`.
//!
//! Tests **SWOM-01..10**. Error enum [`WriteOrderError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WRITE-ORDER`

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

/// Minimum write size (bytes).
pub const SWOM_MIN_SIZE: usize = 32;

/// Maximum writes per batch.
pub const SWOM_MAX_WRITES: usize = 10_000;

/// Session ID length.
pub const SWOM_SESSION_ID_LEN: usize = 32;

/// A store write record.
#[derive(Debug, Clone)]
pub struct StoreWrite {
    /// Session identifier.
    pub session_id: [u8; SWOM_SESSION_ID_LEN],
    /// Write counter.
    pub counter: u64,
    /// Write size in bytes.
    pub size: usize,
}

/// All ways write ordering validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteOrderError {
    /// Not strictly increasing within session.
    NotIncreasing {
        /// Index of the offending write.
        idx: usize,
        /// Previous counter.
        prev: u64,
        /// Current counter.
        current: u64,
    },
    /// Zero counter.
    ZeroCounter(usize),
    /// Zero session ID.
    ZeroSession(usize),
    /// Duplicate counter in session.
    DuplicateCounter(usize),
    /// Write too small.
    TooSmall {
        /// Index.
        idx: usize,
        /// Actual size.
        size: usize,
        /// Minimum size.
        min: usize,
    },
    /// Too many writes.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store write ordering monotonicity.
pub fn validate_write_ordering(writes: &[StoreWrite]) -> Result<(), WriteOrderError> {
    if writes.len() > SWOM_MAX_WRITES {
        return Err(WriteOrderError::TooMany {
            got: writes.len(),
            max: SWOM_MAX_WRITES,
        });
    }
    let mut session_counters: BTreeMap<[u8; SWOM_SESSION_ID_LEN], BTreeSet<u64>> = BTreeMap::new();
    let mut session_last: BTreeMap<[u8; SWOM_SESSION_ID_LEN], u64> = BTreeMap::new();
    for (i, w) in writes.iter().enumerate() {
        if w.session_id == [0u8; SWOM_SESSION_ID_LEN] {
            return Err(WriteOrderError::ZeroSession(i));
        }
        if w.counter == 0 {
            return Err(WriteOrderError::ZeroCounter(i));
        }
        if w.size < SWOM_MIN_SIZE {
            return Err(WriteOrderError::TooSmall {
                idx: i,
                size: w.size,
                min: SWOM_MIN_SIZE,
            });
        }
        let counters = session_counters.entry(w.session_id).or_default();
        if !counters.insert(w.counter) {
            return Err(WriteOrderError::DuplicateCounter(i));
        }
        if let Some(&last) = session_last.get(&w.session_id) {
            if w.counter <= last {
                return Err(WriteOrderError::NotIncreasing {
                    idx: i,
                    prev: last,
                    current: w.counter,
                });
            }
        }
        session_last.insert(w.session_id, w.counter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; SWOM_SESSION_ID_LEN] {
        [byte; SWOM_SESSION_ID_LEN]
    }

    fn write(session: u8, counter: u64, size: usize) -> StoreWrite {
        StoreWrite { session_id: sid(session), counter, size }
    }

    fn valid_writes() -> Vec<StoreWrite> {
        vec![
            write(0x01, 1, 64),
            write(0x01, 2, 128),
            write(0x01, 3, 64),
        ]
    }

    /// **SWOM-01** — not increasing rejected.
    #[test]
    fn swom_01_not_increasing_rejected() {
        let ws = vec![write(0x01, 10, 64), write(0x01, 5, 64)];
        assert_eq!(
            validate_write_ordering(&ws),
            Err(WriteOrderError::NotIncreasing {
                idx: 1,
                prev: 10,
                current: 5,
            })
        );
    }

    /// **SWOM-02** — zero counter rejected.
    #[test]
    fn swom_02_zero_counter_rejected() {
        let w = StoreWrite { session_id: sid(0x01), counter: 0, size: 64 };
        assert_eq!(
            validate_write_ordering(&[w]),
            Err(WriteOrderError::ZeroCounter(0))
        );
    }

    /// **SWOM-03** — zero session rejected.
    #[test]
    fn swom_03_zero_session_rejected() {
        let w = StoreWrite { session_id: [0u8; SWOM_SESSION_ID_LEN], counter: 1, size: 64 };
        assert_eq!(
            validate_write_ordering(&[w]),
            Err(WriteOrderError::ZeroSession(0))
        );
    }

    /// **SWOM-04** — duplicate counter rejected.
    #[test]
    fn swom_04_duplicate_rejected() {
        let ws = vec![write(0x01, 5, 64), write(0x01, 5, 64)];
        assert_eq!(
            validate_write_ordering(&ws),
            Err(WriteOrderError::DuplicateCounter(1))
        );
    }

    /// **SWOM-05** — too small rejected.
    #[test]
    fn swom_05_too_small_rejected() {
        let w = StoreWrite { session_id: sid(0x01), counter: 1, size: 16 };
        assert_eq!(
            validate_write_ordering(&[w]),
            Err(WriteOrderError::TooSmall {
                idx: 0,
                size: 16,
                min: SWOM_MIN_SIZE,
            })
        );
    }

    /// **SWOM-06** — too many rejected.
    #[test]
    fn swom_06_too_many_rejected() {
        let ws: Vec<StoreWrite> = (0..=SWOM_MAX_WRITES)
            .map(|i| StoreWrite {
                session_id: sid(0x01),
                counter: (i as u64) + 1,
                size: 64,
            })
            .collect();
        assert_eq!(
            validate_write_ordering(&ws),
            Err(WriteOrderError::TooMany {
                got: SWOM_MAX_WRITES + 1,
                max: SWOM_MAX_WRITES,
            })
        );
    }

    /// **SWOM-07** — valid accepted.
    #[test]
    fn swom_07_valid_accepted() {
        assert_eq!(validate_write_ordering(&valid_writes()), Ok(()));
    }

    /// **SWOM-08** — empty accepted.
    #[test]
    fn swom_08_empty_accepted() {
        assert_eq!(validate_write_ordering(&[]), Ok(()));
    }

    /// **SWOM-09** — interleaved sessions accepted.
    #[test]
    fn swom_09_interleaved_accepted() {
        let ws = vec![
            write(0x01, 1, 64),
            write(0x02, 1, 64),
            write(0x01, 2, 64),
            write(0x02, 2, 64),
        ];
        assert_eq!(validate_write_ordering(&ws), Ok(()));
    }

    /// **SWOM-10** — single accepted.
    #[test]
    fn swom_10_single_accepted() {
        let ws = vec![write(0x01, 1, 64)];
        assert_eq!(validate_write_ordering(&ws), Ok(()));
    }
}
