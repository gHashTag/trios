//! # CR-CHAT-05 — Store envelope counter monotonicity guard (Wave-157 Lane B)
//!
//! PERSISTENCE — envelope counters must be strictly monotonic per
//! session; gaps or replays indicate tampering.
//!
//! When envelopes are stored, their counters must be strictly
//! increasing within each session. If counters are not monotonic:
//!
//! * **Replay attacks** — duplicate counters indicate replayed or
//!   re-injected messages.
//! * **Gap exploitation** — counter gaps may indicate deleted or
//!   intercepted messages.
//! * **Ordering violations** — non-monotonic counters suggest
//!   database corruption or tampering.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Counters must be strictly increasing per session.
//! 2. Session ID must not be zero.
//! 3. No duplicate (session_id, counter) pairs.
//! 4. Counter must not be zero.
//! 5. Maximum gap between consecutive counters <= `SECM_MAX_GAP`.
//! 6. Batch size <= `SECM_MAX_ENVELOPES`.
//!
//! Tests **SECM-01..10**. Error enum [`CounterMonotonicityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COUNTER-MONOTONE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum gap between consecutive counters.
pub const SECM_MAX_GAP: u64 = 1024;

/// Maximum envelopes per batch.
pub const SECM_MAX_ENVELOPES: usize = 4096;

/// Session ID length.
pub const SECM_SESSION_ID_LEN: usize = 32;

/// An envelope counter record.
#[derive(Debug, Clone)]
pub struct EnvelopeCounterRecord {
    /// Session identifier.
    pub session_id: [u8; SECM_SESSION_ID_LEN],
    /// Envelope counter.
    pub counter: u64,
}

/// All ways counter monotonicity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CounterMonotonicityError {
    /// Counter not strictly increasing within session.
    NonMonotonic {
        /// Index of the offending record.
        idx: usize,
        /// Previous counter.
        prev: u64,
        /// Current counter.
        got: u64,
    },
    /// Zero session ID.
    ZeroSessionId(usize),
    /// Duplicate session+counter pair.
    DuplicatePair {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Zero counter.
    ZeroCounter(usize),
    /// Gap exceeds maximum.
    GapTooLarge {
        /// Index.
        idx: usize,
        /// Gap size.
        gap: u64,
        /// Maximum allowed.
        max: u64,
    },
    /// Too many envelopes.
    TooMany {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store envelope counter monotonicity.
pub fn validate_counter_monotonicity(
    envelopes: &[EnvelopeCounterRecord],
) -> Result<(), CounterMonotonicityError> {
    if envelopes.len() > SECM_MAX_ENVELOPES {
        return Err(CounterMonotonicityError::TooMany {
            got: envelopes.len(),
            max: SECM_MAX_ENVELOPES,
        });
    }
    let mut seen: BTreeSet<([u8; SECM_SESSION_ID_LEN], u64)> = BTreeSet::new();
    let mut last_counter: std::collections::BTreeMap<[u8; SECM_SESSION_ID_LEN], u64> =
        std::collections::BTreeMap::new();
    for (i, e) in envelopes.iter().enumerate() {
        if e.session_id == [0u8; SECM_SESSION_ID_LEN] {
            return Err(CounterMonotonicityError::ZeroSessionId(i));
        }
        if e.counter == 0 {
            return Err(CounterMonotonicityError::ZeroCounter(i));
        }
        if !seen.insert((e.session_id, e.counter)) {
            return Err(CounterMonotonicityError::DuplicatePair { idx: i });
        }
        if let Some(&prev) = last_counter.get(&e.session_id) {
            if e.counter <= prev {
                return Err(CounterMonotonicityError::NonMonotonic {
                    idx: i,
                    prev,
                    got: e.counter,
                });
            }
            let gap = e.counter - prev;
            if gap > SECM_MAX_GAP {
                return Err(CounterMonotonicityError::GapTooLarge {
                    idx: i,
                    gap,
                    max: SECM_MAX_GAP,
                });
            }
        }
        last_counter.insert(e.session_id, e.counter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; SECM_SESSION_ID_LEN] {
        [byte; SECM_SESSION_ID_LEN]
    }

    fn env(session: u8, counter: u64) -> EnvelopeCounterRecord {
        EnvelopeCounterRecord { session_id: sid(session), counter }
    }

    fn valid_envelopes() -> Vec<EnvelopeCounterRecord> {
        vec![
            env(0x01, 1),
            env(0x01, 2),
            env(0x01, 3),
            env(0x02, 1),
            env(0x02, 2),
        ]
    }

    /// **SECM-01** — non-monotonic rejected.
    #[test]
    fn secm_01_non_monotonic_rejected() {
        let es = vec![
            env(0x01, 5),
            env(0x01, 3),
        ];
        assert_eq!(
            validate_counter_monotonicity(&es),
            Err(CounterMonotonicityError::NonMonotonic { idx: 1, prev: 5, got: 3 })
        );
    }

    /// **SECM-02** — zero session ID rejected.
    #[test]
    fn secm_02_zero_session_rejected() {
        let e = EnvelopeCounterRecord { session_id: [0u8; SECM_SESSION_ID_LEN], counter: 1 };
        assert_eq!(
            validate_counter_monotonicity(&[e]),
            Err(CounterMonotonicityError::ZeroSessionId(0))
        );
    }

    /// **SECM-03** — duplicate pair rejected.
    #[test]
    fn secm_03_duplicate_rejected() {
        let es = vec![
            env(0x01, 1),
            env(0x01, 1),
        ];
        assert_eq!(
            validate_counter_monotonicity(&es),
            Err(CounterMonotonicityError::DuplicatePair { idx: 1 })
        );
    }

    /// **SECM-04** — zero counter rejected.
    #[test]
    fn secm_04_zero_counter_rejected() {
        let e = EnvelopeCounterRecord { session_id: sid(0x01), counter: 0 };
        assert_eq!(
            validate_counter_monotonicity(&[e]),
            Err(CounterMonotonicityError::ZeroCounter(0))
        );
    }

    /// **SECM-05** — gap too large rejected.
    #[test]
    fn secm_05_gap_too_large_rejected() {
        let es = vec![
            env(0x01, 1),
            env(0x01, 1 + SECM_MAX_GAP + 1),
        ];
        assert_eq!(
            validate_counter_monotonicity(&es),
            Err(CounterMonotonicityError::GapTooLarge {
                idx: 1,
                gap: SECM_MAX_GAP + 1,
                max: SECM_MAX_GAP,
            })
        );
    }

    /// **SECM-06** — too many rejected.
    #[test]
    fn secm_06_too_many_rejected() {
        let es: Vec<EnvelopeCounterRecord> = (0..=SECM_MAX_ENVELOPES)
            .map(|i| {
                let mut s = [0u8; SECM_SESSION_ID_LEN];
                let val = (i as u64) + 1;
                s[0..8].copy_from_slice(&val.to_be_bytes());
                EnvelopeCounterRecord { session_id: s, counter: 1 }
            })
            .collect();
        assert_eq!(
            validate_counter_monotonicity(&es),
            Err(CounterMonotonicityError::TooMany {
                got: SECM_MAX_ENVELOPES + 1,
                max: SECM_MAX_ENVELOPES,
            })
        );
    }

    /// **SECM-07** — valid accepted.
    #[test]
    fn secm_07_valid_accepted() {
        assert_eq!(validate_counter_monotonicity(&valid_envelopes()), Ok(()));
    }

    /// **SECM-08** — empty accepted.
    #[test]
    fn secm_08_empty_accepted() {
        assert_eq!(validate_counter_monotonicity(&[]), Ok(()));
    }

    /// **SECM-09** — boundary gap accepted.
    #[test]
    fn secm_09_boundary_gap_accepted() {
        let es = vec![
            env(0x01, 1),
            env(0x01, 1 + SECM_MAX_GAP),
        ];
        assert_eq!(validate_counter_monotonicity(&es), Ok(()));
    }

    /// **SECM-10** — multi-session accepted.
    #[test]
    fn secm_10_multi_session_accepted() {
        let es: Vec<EnvelopeCounterRecord> = (0..10u8)
            .flat_map(|s| (0..5u64).map(move |c| env(s + 1, c + 1)))
            .collect();
        assert_eq!(validate_counter_monotonicity(&es), Ok(()));
    }
}
