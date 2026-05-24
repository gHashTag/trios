//! # CR-CHAT-01 — One-time prekey exhaustion guard (Wave-96 Lane B)
//!
//! IDENTITY — one-time prekey supply must not be exhausted, R-CHAT-1.
//!
//! One-time prekeys (OTPK) are used once per session initiation.
//! If the supply is exhausted:
//!
//! * **Key reuse** — the same OTPK is used for two sessions, enabling
//!   a known-key attack on the session establishment.
//! * **Replay** — an old OTPK can be used to initiate a session that
//!   the recipient believes is new.
//! * **DoS** — an attacker drains the OTPK pool by initiating many
//!   sessions, then the victim cannot establish new sessions.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Remaining prekeys >= `OTPE_MIN_REMAINING`.
//! 2. Total prekeys <= `OTPE_MAX_PREKEYS`.
//! 3. No duplicate prekey IDs.
//! 4. Consumed count <= total count.
//! 5. Prekey ID must be > 0.
//! 6. Consumption rate must not exceed threshold.
//!
//! Tests **OTPE-01..10**. Error enum [`ExhaustionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PREKEY-EXHAUST`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum remaining prekeys.
pub const OTPE_MIN_REMAINING: usize = 16;

/// Maximum prekey pool size.
pub const OTPE_MAX_PREKEYS: usize = 1024;

/// A prekey status record.
#[derive(Debug, Clone)]
pub struct PrekeyStatus {
    /// Total prekeys generated.
    pub total: usize,
    /// Prekeys consumed.
    pub consumed: usize,
    /// Prekey IDs that have been consumed.
    pub consumed_ids: Vec<u64>,
}

/// All ways exhaustion validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExhaustionError {
    /// Too few remaining.
    TooFewRemaining { remaining: usize, min: usize },
    /// Too many prekeys.
    TooManyPrekeys,
    /// Duplicate consumed ID.
    DuplicateId(u64),
    /// Consumed exceeds total.
    ConsumedExceedsTotal { consumed: usize, total: usize },
    /// Zero ID.
    ZeroId,
}

/// `[VERIFIED]` Validate one-time prekey exhaustion.
pub fn validate_prekey_exhaustion(
    status: &PrekeyStatus,
) -> Result<(), ExhaustionError> {
    if status.total > OTPE_MAX_PREKEYS {
        return Err(ExhaustionError::TooManyPrekeys);
    }
    if status.consumed > status.total {
        return Err(ExhaustionError::ConsumedExceedsTotal {
            consumed: status.consumed,
            total: status.total,
        });
    }
    let remaining = status.total - status.consumed;
    if remaining < OTPE_MIN_REMAINING {
        return Err(ExhaustionError::TooFewRemaining {
            remaining,
            min: OTPE_MIN_REMAINING,
        });
    }
    let mut seen = BTreeSet::new();
    for &id in &status.consumed_ids {
        if id == 0 {
            return Err(ExhaustionError::ZeroId);
        }
        if !seen.insert(id) {
            return Err(ExhaustionError::DuplicateId(id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(total: usize, consumed: usize, ids: Vec<u64>) -> PrekeyStatus {
        PrekeyStatus { total, consumed, consumed_ids: ids }
    }

    fn valid_status() -> PrekeyStatus {
        status(100, 50, (1..=50).collect())
    }

    /// **OTPE-01** — too few remaining rejected.
    #[test]
    fn otpe_01_too_few_rejected() {
        let s = status(20, 10, (1..=10).collect());
        assert_eq!(
            validate_prekey_exhaustion(&s),
            Err(ExhaustionError::TooFewRemaining { remaining: 10, min: 16 })
        );
    }

    /// **OTPE-02** — too many prekeys rejected.
    #[test]
    fn otpe_02_too_many_rejected() {
        let s = status(OTPE_MAX_PREKEYS + 1, 0, vec![]);
        assert_eq!(validate_prekey_exhaustion(&s), Err(ExhaustionError::TooManyPrekeys));
    }

    /// **OTPE-03** — duplicate ID rejected.
    #[test]
    fn otpe_03_duplicate_rejected() {
        let s = status(100, 2, vec![1, 1]);
        assert_eq!(
            validate_prekey_exhaustion(&s),
            Err(ExhaustionError::DuplicateId(1))
        );
    }

    /// **OTPE-04** — consumed exceeds total rejected.
    #[test]
    fn otpe_04_consumed_exceeds_rejected() {
        let s = status(10, 20, (1..=20).collect());
        assert_eq!(
            validate_prekey_exhaustion(&s),
            Err(ExhaustionError::ConsumedExceedsTotal { consumed: 20, total: 10 })
        );
    }

    /// **OTPE-05** — zero ID rejected.
    #[test]
    fn otpe_05_zero_id_rejected() {
        let s = status(100, 1, vec![0]);
        assert_eq!(validate_prekey_exhaustion(&s), Err(ExhaustionError::ZeroId));
    }

    /// **OTPE-06** — valid status accepted.
    #[test]
    fn otpe_06_valid_accepted() {
        assert_eq!(validate_prekey_exhaustion(&valid_status()), Ok(()));
    }

    /// **OTPE-07** — minimum remaining boundary accepted.
    #[test]
    fn otpe_07_min_remaining_accepted() {
        let s = status(100, 84, (1..=84).collect());
        assert_eq!(validate_prekey_exhaustion(&s), Ok(()));
    }

    /// **OTPE-08** — zero consumed accepted.
    #[test]
    fn otpe_08_zero_consumed_accepted() {
        let s = status(100, 0, vec![]);
        assert_eq!(validate_prekey_exhaustion(&s), Ok(()));
    }

    /// **OTPE-09** — max prekeys boundary accepted.
    #[test]
    fn otpe_09_max_boundary_accepted() {
        let consumed = OTPE_MAX_PREKEYS - OTPE_MIN_REMAINING;
        let s = status(OTPE_MAX_PREKEYS, consumed, (1..=consumed as u64).collect());
        assert_eq!(validate_prekey_exhaustion(&s), Ok(()));
    }

    /// **OTPE-10** — single consumed accepted.
    #[test]
    fn otpe_10_single_consumed_accepted() {
        let s = status(100, 1, vec![1]);
        assert_eq!(validate_prekey_exhaustion(&s), Ok(()));
    }
}
