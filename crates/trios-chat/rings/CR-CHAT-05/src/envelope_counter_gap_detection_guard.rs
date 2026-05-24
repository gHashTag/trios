//! # CR-CHAT-05 — Envelope counter gap detection guard (Wave-142 Lane B)
//!
//! PERSISTENCE — envelope counters within a session must be
//! contiguous; gaps indicate missing or reordered messages.
//!
//! Each envelope in a session carries a monotonically increasing
//! counter. If counters have gaps:
//!
//! * **Missing messages** — a gap in counters indicates that
//!   messages were lost in transit or deliberately dropped.
//! * **Reorder attack** — an attacker who reorders messages creates
//!   counter sequences that appear to have gaps.
//! * **Integrity check** — contiguous counters are a lightweight
//!   integrity check for message delivery completeness.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Counters must be contiguous (next = prev + 1).
//! 2. Session ID must not be zero.
//! 3. No duplicate session IDs.
//! 4. First counter must be 1.
//! 5. Counter must not be zero.
//! 6. Batch size <= `ECGD_MAX_ENTRIES`.
//!
//! Tests **ECGD-01..10**. Error enum [`CounterGapError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CONTIGUOUS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum entries per batch.
pub const ECGD_MAX_ENTRIES: usize = 4096;

/// Session ID length.
pub const ECGD_SESSION_ID_LEN: usize = 32;

/// An envelope counter record.
#[derive(Debug, Clone)]
pub struct CounterGapRecord {
    /// Session identifier.
    pub session_id: [u8; ECGD_SESSION_ID_LEN],
    /// Envelope counter value.
    pub counter: u64,
}

/// All ways counter gap validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CounterGapError {
    /// Gap detected between counters.
    Gap {
        /// Index of the gap.
        idx: usize,
        /// Current counter.
        got: u64,
        /// Expected counter.
        expected: u64,
    },
    /// Zero session ID.
    ZeroSessionId(
        /// Index.
        usize,
    ),
    /// Duplicate session ID.
    DuplicateSessionId {
        /// Index.
        idx: usize,
    },
    /// First counter must be 1.
    FirstNotOne {
        /// Declared counter.
        got: u64,
    },
    /// Zero counter.
    ZeroCounter(
        /// Index.
        usize,
    ),
    /// Too many entries.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate envelope counter gap detection.
pub fn validate_counter_contiguity(
    records: &[CounterGapRecord],
) -> Result<(), CounterGapError> {
    if records.len() > ECGD_MAX_ENTRIES {
        return Err(CounterGapError::TooMany {
            got: records.len(),
            max: ECGD_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<[u8; ECGD_SESSION_ID_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.session_id == [0u8; ECGD_SESSION_ID_LEN] {
            return Err(CounterGapError::ZeroSessionId(i));
        }
        if !seen.insert(r.session_id) {
            return Err(CounterGapError::DuplicateSessionId { idx: i });
        }
        if r.counter == 0 {
            return Err(CounterGapError::ZeroCounter(i));
        }
        if i == 0 {
            if r.counter != 1 {
                return Err(CounterGapError::FirstNotOne { got: r.counter });
            }
        } else {
            let expected = records[i - 1].counter + 1;
            if r.counter != expected {
                return Err(CounterGapError::Gap {
                    idx: i,
                    got: r.counter,
                    expected,
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; ECGD_SESSION_ID_LEN] {
        [byte; ECGD_SESSION_ID_LEN]
    }

    fn rec(session: u8, counter: u64) -> CounterGapRecord {
        CounterGapRecord { session_id: sid(session), counter }
    }

    fn valid_sequence() -> Vec<CounterGapRecord> {
        vec![
            rec(0x01, 1),
            rec(0x02, 2),
            rec(0x03, 3),
            rec(0x04, 4),
            rec(0x05, 5),
        ]
    }

    /// **ECGD-01** — gap rejected.
    #[test]
    fn ecgd_01_gap_rejected() {
        let rs = vec![
            rec(0x01, 1),
            rec(0x02, 5),
        ];
        assert_eq!(
            validate_counter_contiguity(&rs),
            Err(CounterGapError::Gap {
                idx: 1,
                got: 5,
                expected: 2,
            })
        );
    }

    /// **ECGD-02** — zero session ID rejected.
    #[test]
    fn ecgd_02_zero_session_rejected() {
        let r = CounterGapRecord { session_id: [0u8; ECGD_SESSION_ID_LEN], counter: 1 };
        assert_eq!(
            validate_counter_contiguity(&[r]),
            Err(CounterGapError::ZeroSessionId(0))
        );
    }

    /// **ECGD-03** — duplicate session ID rejected.
    #[test]
    fn ecgd_03_duplicate_rejected() {
        let rs = vec![
            rec(0x01, 1),
            rec(0x01, 2),
        ];
        assert_eq!(
            validate_counter_contiguity(&rs),
            Err(CounterGapError::DuplicateSessionId { idx: 1 })
        );
    }

    /// **ECGD-04** — first not 1 rejected.
    #[test]
    fn ecgd_04_first_not_one_rejected() {
        let r = rec(0x01, 5);
        assert_eq!(
            validate_counter_contiguity(&[r]),
            Err(CounterGapError::FirstNotOne { got: 5 })
        );
    }

    /// **ECGD-05** — zero counter rejected.
    #[test]
    fn ecgd_05_zero_counter_rejected() {
        let r = CounterGapRecord { session_id: sid(0x01), counter: 0 };
        assert_eq!(
            validate_counter_contiguity(&[r]),
            Err(CounterGapError::ZeroCounter(0))
        );
    }

    /// **ECGD-06** — too many rejected.
    #[test]
    fn ecgd_06_too_many_rejected() {
        let rs: Vec<CounterGapRecord> = (0..=ECGD_MAX_ENTRIES)
            .map(|i| {
                let mut s = [0u8; ECGD_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                CounterGapRecord { session_id: s, counter: (i as u64) + 1 }
            })
            .collect();
        assert_eq!(
            validate_counter_contiguity(&rs),
            Err(CounterGapError::TooMany {
                got: ECGD_MAX_ENTRIES + 1,
                max: ECGD_MAX_ENTRIES,
            })
        );
    }

    /// **ECGD-07** — valid accepted.
    #[test]
    fn ecgd_07_valid_accepted() {
        assert_eq!(validate_counter_contiguity(&valid_sequence()), Ok(()));
    }

    /// **ECGD-08** — empty accepted.
    #[test]
    fn ecgd_08_empty_accepted() {
        assert_eq!(validate_counter_contiguity(&[]), Ok(()));
    }

    /// **ECGD-09** — single counter=1 accepted.
    #[test]
    fn ecgd_09_single_accepted() {
        assert_eq!(validate_counter_contiguity(&[rec(0x01, 1)]), Ok(()));
    }

    /// **ECGD-10** — long contiguous sequence accepted.
    #[test]
    fn ecgd_10_long_sequence_accepted() {
        let rs: Vec<CounterGapRecord> = (0..200)
            .map(|i| {
                let mut s = [0u8; ECGD_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                CounterGapRecord { session_id: s, counter: (i as u64) + 1 }
            })
            .collect();
        assert_eq!(validate_counter_contiguity(&rs), Ok(()));
    }
}
