//! # CR-CHAT-01 — Identity key compromise recovery guard (Wave-86 Lane A)
//!
//! IDENTITY — recovery from a compromised identity key must be
//! authenticated via a signed transition, R-CHAT-1.
//!
//! After an identity key compromise:
//!
//! * **Unauthenticated recovery** — attacker replaces the victim's key
//!   with their own, taking over the identity permanently.
//! * **Replay recovery** — old recovery transcript replayed to roll
//!   back to a previously-compromised key.
//! * **Orphan recovery** — new key has no cryptographic binding to the
//!   old key, so peers cannot verify continuity of identity.
//!
//! IKCR enforces that every key recovery is authenticated: the old key
//! (or a designated recovery key) must sign the transition to the new
//! key, and each identity can have at most one pending recovery.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Recovery must be signed by the old key.
//! 2. New key must differ from old key.
//! 3. No duplicate recovery for same identity.
//! 4. Recovery sequence must be strictly increasing.
//! 5. Maximum recoveries <= `IKCR_MAX_RECOVERIES`.
//! 6. New key must not be all zeros.
//!
//! Tests **IKCR-01..10**. Error enum [`RecoveryError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * IDENTITY-RECOVERY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum recoveries per identity.
pub const IKCR_MAX_RECOVERIES: usize = 8;

/// Length of identity keys.
pub const IKCR_KEY_LEN: usize = 32;

/// A key recovery transition record.
#[derive(Debug, Clone)]
pub struct RecoveryTransition {
    /// Identity being recovered.
    pub identity: u64,
    /// Old (compromised) key.
    pub old_key: [u8; IKCR_KEY_LEN],
    /// New (replacement) key.
    pub new_key: [u8; IKCR_KEY_LEN],
    /// Whether the transition is signed by the old key.
    pub is_signed: bool,
    /// Recovery sequence number.
    pub seq: u64,
}

/// All ways recovery validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecoveryError {
    /// Recovery not signed.
    NotSigned(u64),
    /// New key same as old key.
    SameKey(u64),
    /// Duplicate recovery for identity.
    DuplicateRecovery(u64),
    /// Sequence not increasing.
    SeqNotIncreasing(u64),
    /// Too many recoveries.
    TooManyRecoveries,
    /// New key is all zeros.
    ZeroKey(u64),
}

/// `[VERIFIED]` Validate identity key compromise recovery transitions.
pub fn validate_recovery_transitions(
    transitions: &[RecoveryTransition],
) -> Result<(), RecoveryError> {
    if transitions.len() > IKCR_MAX_RECOVERIES {
        return Err(RecoveryError::TooManyRecoveries);
    }
    let mut seen = BTreeSet::new();
    for (i, t) in transitions.iter().enumerate() {
        if t.new_key == [0u8; IKCR_KEY_LEN] {
            return Err(RecoveryError::ZeroKey(t.identity));
        }
        if !t.is_signed {
            return Err(RecoveryError::NotSigned(t.identity));
        }
        if t.old_key == t.new_key {
            return Err(RecoveryError::SameKey(t.identity));
        }
        if i > 0 && t.seq <= transitions[i - 1].seq {
            return Err(RecoveryError::SeqNotIncreasing(t.seq));
        }
        if !seen.insert(t.identity) {
            return Err(RecoveryError::DuplicateRecovery(t.identity));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; IKCR_KEY_LEN] {
        [byte; IKCR_KEY_LEN]
    }

    fn transition(identity: u64, old: u8, new: u8, seq: u64) -> RecoveryTransition {
        RecoveryTransition {
            identity,
            old_key: key(old),
            new_key: key(new),
            is_signed: true,
            seq,
        }
    }

    fn valid_transitions() -> Vec<RecoveryTransition> {
        vec![transition(1, 0xAA, 0xBB, 1), transition(2, 0xCC, 0xDD, 2)]
    }

    /// **IKCR-01** — not signed rejected.
    #[test]
    fn ikcr_01_not_signed_rejected() {
        let mut t = transition(1, 0xAA, 0xBB, 1);
        t.is_signed = false;
        assert_eq!(
            validate_recovery_transitions(&[t]),
            Err(RecoveryError::NotSigned(1))
        );
    }

    /// **IKCR-02** — same key rejected.
    #[test]
    fn ikcr_02_same_key_rejected() {
        let t = RecoveryTransition {
            identity: 1,
            old_key: key(0xAA),
            new_key: key(0xAA),
            is_signed: true,
            seq: 1,
        };
        assert_eq!(
            validate_recovery_transitions(&[t]),
            Err(RecoveryError::SameKey(1))
        );
    }

    /// **IKCR-03** — duplicate recovery rejected.
    #[test]
    fn ikcr_03_duplicate_rejected() {
        let ts = vec![transition(1, 0xAA, 0xBB, 1), transition(1, 0xBB, 0xCC, 2)];
        assert_eq!(
            validate_recovery_transitions(&ts),
            Err(RecoveryError::DuplicateRecovery(1))
        );
    }

    /// **IKCR-04** — sequence not increasing rejected.
    #[test]
    fn ikcr_04_seq_not_increasing_rejected() {
        let ts = vec![transition(1, 0xAA, 0xBB, 3), transition(2, 0xCC, 0xDD, 2)];
        assert_eq!(
            validate_recovery_transitions(&ts),
            Err(RecoveryError::SeqNotIncreasing(2))
        );
    }

    /// **IKCR-05** — too many recoveries rejected.
    #[test]
    fn ikcr_05_too_many_rejected() {
        let ts: Vec<RecoveryTransition> = (0..=IKCR_MAX_RECOVERIES as u64)
            .map(|i| transition(i, 0x10, 0x20, i))
            .collect();
        assert_eq!(
            validate_recovery_transitions(&ts),
            Err(RecoveryError::TooManyRecoveries)
        );
    }

    /// **IKCR-06** — zero key rejected.
    #[test]
    fn ikcr_06_zero_key_rejected() {
        let t = RecoveryTransition {
            identity: 1,
            old_key: key(0xAA),
            new_key: [0u8; IKCR_KEY_LEN],
            is_signed: true,
            seq: 1,
        };
        assert_eq!(
            validate_recovery_transitions(&[t]),
            Err(RecoveryError::ZeroKey(1))
        );
    }

    /// **IKCR-07** — valid transitions accepted.
    #[test]
    fn ikcr_07_valid_accepted() {
        assert_eq!(validate_recovery_transitions(&valid_transitions()), Ok(()));
    }

    /// **IKCR-08** — empty accepted.
    #[test]
    fn ikcr_08_empty_accepted() {
        assert_eq!(validate_recovery_transitions(&[]), Ok(()));
    }

    /// **IKCR-09** — single accepted.
    #[test]
    fn ikcr_09_single_accepted() {
        assert_eq!(validate_recovery_transitions(&[transition(1, 0x11, 0x22, 1)]), Ok(()));
    }

    /// **IKCR-10** — max recoveries boundary accepted.
    #[test]
    fn ikcr_10_max_boundary_accepted() {
        let ts: Vec<RecoveryTransition> = (0..IKCR_MAX_RECOVERIES as u64)
            .map(|i| transition(i, 0x10, 0x20, i))
            .collect();
        assert_eq!(validate_recovery_transitions(&ts), Ok(()));
    }
}
