//! # CR-CHAT-01 — Prekey bundle one-time use guard (Wave-74 Lane A)
//!
//! IDENTITY — each prekey bundle must be used at most once, R-CHAT-2.
//!
//! X3DH one-time prekeys are consumed on first use. If the same prekey
//! is reused for two sessions:
//!
//! * **Forward secrecy break** — both sessions share the same DH output,
//!   so compromising one session's state reveals the other's root key.
//! * **Key derivation collision** — two sessions derive the same chain
//!   key from the same prekey material.
//! * **State confusion** — the receiver processes two sessions with
//!   overlapping keys, leading to decryption failures.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each prekey ID appears at most once in the consumed set.
//! 2. Prekey ID is non-zero.
//! 3. Consumed count <= `PBOU_MAX_CONSUMED`.
//! 4. Consumed set size + available count <= `PBOU_MAX_BUNDLE`.
//! 5. No prekey ID in both consumed and available sets.
//! 6. Available count > 0 (at least one prekey remains).
//!
//! Tests **PBOU-01..10**. Error enum [`PrekeyReuseError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PREKEY-ONE-TIME`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum consumed prekeys tracked.
pub const PBOU_MAX_CONSUMED: usize = 1024;

/// Maximum bundle size (consumed + available).
pub const PBOU_MAX_BUNDLE: usize = 2048;

/// All ways prekey one-time use validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PrekeyReuseError {
    /// Prekey already consumed.
    AlreadyConsumed(u64),
    /// Zero prekey ID.
    ZeroPrekeyId,
    /// Too many consumed.
    TooManyConsumed,
    /// Bundle size exceeded.
    BundleSizeExceeded,
    /// Prekey in both sets.
    InBothSets(u64),
    /// No available prekeys.
    NoAvailable,
}

/// `[VERIFIED]` Validate that a prekey is used at most once.
pub fn validate_prekey_one_time(
    consumed: &BTreeSet<u64>,
    available: &BTreeSet<u64>,
    prekey_id: u64,
) -> Result<(), PrekeyReuseError> {
    if prekey_id == 0 {
        return Err(PrekeyReuseError::ZeroPrekeyId);
    }
    if consumed.len() > PBOU_MAX_CONSUMED {
        return Err(PrekeyReuseError::TooManyConsumed);
    }
    if consumed.len() + available.len() > PBOU_MAX_BUNDLE {
        return Err(PrekeyReuseError::BundleSizeExceeded);
    }
    if consumed.contains(&prekey_id) {
        return Err(PrekeyReuseError::AlreadyConsumed(prekey_id));
    }
    for id in consumed {
        if available.contains(id) {
            return Err(PrekeyReuseError::InBothSets(*id));
        }
    }
    if available.is_empty() {
        return Err(PrekeyReuseError::NoAvailable);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consumed_set(ids: &[u64]) -> BTreeSet<u64> {
        ids.iter().copied().collect()
    }

    fn available_set(ids: &[u64]) -> BTreeSet<u64> {
        ids.iter().copied().collect()
    }

    fn valid_sets() -> (BTreeSet<u64>, BTreeSet<u64>) {
        (consumed_set(&[1, 2, 3]), available_set(&[4, 5, 6]))
    }

    /// **PBOU-01** — already consumed rejected.
    #[test]
    fn pbou_01_already_consumed_rejected() {
        let (consumed, available) = valid_sets();
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 2),
            Err(PrekeyReuseError::AlreadyConsumed(2))
        );
    }

    /// **PBOU-02** — zero prekey ID rejected.
    #[test]
    fn pbou_02_zero_rejected() {
        let (consumed, available) = valid_sets();
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 0),
            Err(PrekeyReuseError::ZeroPrekeyId)
        );
    }

    /// **PBOU-03** — too many consumed rejected.
    #[test]
    fn pbou_03_too_many_consumed_rejected() {
        let consumed: BTreeSet<u64> = (1..=PBOU_MAX_CONSUMED as u64 + 1).collect();
        let available = available_set(&[9999]);
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 9999),
            Err(PrekeyReuseError::TooManyConsumed)
        );
    }

    /// **PBOU-04** — bundle size exceeded rejected.
    #[test]
    fn pbou_04_bundle_exceeded_rejected() {
        let consumed: BTreeSet<u64> = (1..=1000).collect();
        let available: BTreeSet<u64> = (1001u64..=1001 + PBOU_MAX_BUNDLE as u64).collect();
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 9999),
            Err(PrekeyReuseError::BundleSizeExceeded)
        );
    }

    /// **PBOU-05** — prekey in both sets rejected.
    #[test]
    fn pbou_05_both_sets_rejected() {
        let consumed = consumed_set(&[1, 2, 3]);
        let available = available_set(&[3, 4, 5]);
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 5),
            Err(PrekeyReuseError::InBothSets(3))
        );
    }

    /// **PBOU-06** — no available prekeys rejected.
    #[test]
    fn pbou_06_no_available_rejected() {
        let consumed = consumed_set(&[1, 2, 3]);
        let available = BTreeSet::new();
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, 5),
            Err(PrekeyReuseError::NoAvailable)
        );
    }

    /// **PBOU-07** — valid prekey accepted.
    #[test]
    fn pbou_07_valid_accepted() {
        let (consumed, available) = valid_sets();
        assert_eq!(validate_prekey_one_time(&consumed, &available, 4), Ok(()));
    }

    /// **PBOU-08** — fresh available prekey accepted.
    #[test]
    fn pbou_08_fresh_accepted() {
        let (consumed, available) = valid_sets();
        assert_eq!(validate_prekey_one_time(&consumed, &available, 6), Ok(()));
    }

    /// **PBOU-09** — empty consumed accepted.
    #[test]
    fn pbou_09_empty_consumed_accepted() {
        let consumed = BTreeSet::new();
        let available = available_set(&[1, 2, 3]);
        assert_eq!(validate_prekey_one_time(&consumed, &available, 1), Ok(()));
    }

    /// **PBOU-10** — max consumed accepted.
    #[test]
    fn pbou_10_max_consumed_accepted() {
        let consumed: BTreeSet<u64> = (1..=PBOU_MAX_CONSUMED as u64).collect();
        let available = available_set(&[PBOU_MAX_CONSUMED as u64 + 1]);
        assert_eq!(
            validate_prekey_one_time(&consumed, &available, PBOU_MAX_CONSUMED as u64 + 1),
            Ok(())
        );
    }
}
