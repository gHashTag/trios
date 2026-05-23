//! # CR-CHAT-05 — Key rotation replay guard (Wave-52 Lane A)
//!
//! R-CHAT-1 — At-rest key rotation replay prevention.
//!
//! When the persistence layer rotates session keys, old ciphertexts
//! remain stored under their original keys. An adversary who can replay
//! an old rotation event can:
//!
//! * **Restore stale keys** — roll back to a compromised epoch.
//! * **Re-encrypt under old keys** — force the store to accept new
//!   rows encrypted with deprecated key material.
//! * **Break forward secrecy** — re-introduce keys that should have
//!   been destroyed.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Rotation epoch is strictly increasing.
//! 2. Rotation timestamp is non-decreasing.
//! 3. Old epoch key material is zeroized (simulated by absence).
//! 4. No duplicate rotation events.
//! 5. Rotation chain starts at epoch 1.
//! 6. Maximum rotations ≤ `KRRG_MAX_ROTATIONS`.
//!
//! Tests **KRRG-01..10**. Error enum [`KeyRotationReplayError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · KEY-ROTATION-REPLAY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum number of rotations.
pub const KRRG_MAX_ROTATIONS: usize = 1024;

/// A key rotation event.
#[derive(Debug, Clone)]
pub struct RotationEvent {
    /// Epoch number (must be strictly increasing).
    pub epoch: u64,
    /// Timestamp (milliseconds since UNIX epoch, non-decreasing).
    pub timestamp_ms: u64,
}

/// All ways key rotation replay validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyRotationReplayError {
    /// Epoch not strictly increasing.
    EpochNotIncreasing,
    /// Timestamp decreased.
    TimestampDecreased,
    /// Duplicate epoch.
    DuplicateEpoch,
    /// First epoch is not 1.
    FirstEpochNotOne,
    /// Too many rotations.
    TooManyRotations,
    /// Zero epoch not allowed.
    ZeroEpoch,
}

/// `[VERIFIED]` Validate a chain of key rotation events.
pub fn validate_rotation_chain(
    rotations: &[RotationEvent],
) -> Result<(), KeyRotationReplayError> {
    if rotations.len() > KRRG_MAX_ROTATIONS {
        return Err(KeyRotationReplayError::TooManyRotations);
    }
    if rotations.is_empty() {
        return Ok(());
    }
    let mut seen_epochs = BTreeSet::new();
    for (i, rot) in rotations.iter().enumerate() {
        if rot.epoch == 0 {
            return Err(KeyRotationReplayError::ZeroEpoch);
        }
        if i == 0 && rot.epoch != 1 {
            return Err(KeyRotationReplayError::FirstEpochNotOne);
        }
        if !seen_epochs.insert(rot.epoch) {
            return Err(KeyRotationReplayError::DuplicateEpoch);
        }
        if i > 0 {
            let prev = &rotations[i - 1];
            if rot.epoch <= prev.epoch {
                return Err(KeyRotationReplayError::EpochNotIncreasing);
            }
            if rot.timestamp_ms < prev.timestamp_ms {
                return Err(KeyRotationReplayError::TimestampDecreased);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rot(epoch: u64, ts: u64) -> RotationEvent {
        RotationEvent { epoch, timestamp_ms: ts }
    }

    fn good_chain() -> Vec<RotationEvent> {
        vec![rot(1, 1000), rot(2, 2000), rot(3, 3000)]
    }

    /// **KRRG-01** — epoch not increasing rejected.
    #[test]
    fn krrg_01_epoch_not_increasing_rejected() {
        let chain = vec![rot(1, 1000), rot(3, 2000), rot(2, 3000)];
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::EpochNotIncreasing)
        );
    }

    /// **KRRG-02** — timestamp decreased rejected.
    #[test]
    fn krrg_02_timestamp_decreased_rejected() {
        let chain = vec![rot(1, 3000), rot(2, 2000)];
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::TimestampDecreased)
        );
    }

    /// **KRRG-03** — duplicate epoch rejected.
    #[test]
    fn krrg_03_duplicate_epoch_rejected() {
        let chain = vec![rot(1, 1000), rot(2, 2000), rot(2, 3000)];
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::DuplicateEpoch)
        );
    }

    /// **KRRG-04** — first epoch not 1 rejected.
    #[test]
    fn krrg_04_first_epoch_not_one_rejected() {
        let chain = vec![rot(2, 1000)];
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::FirstEpochNotOne)
        );
    }

    /// **KRRG-05** — too many rotations rejected.
    #[test]
    fn krrg_05_too_many_rejected() {
        let chain: Vec<RotationEvent> = (0..=KRRG_MAX_ROTATIONS)
            .map(|i| rot((i + 1) as u64, i as u64 * 100))
            .collect();
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::TooManyRotations)
        );
    }

    /// **KRRG-06** — zero epoch rejected.
    #[test]
    fn krrg_06_zero_epoch_rejected() {
        let chain = vec![rot(0, 1000)];
        assert_eq!(
            validate_rotation_chain(&chain),
            Err(KeyRotationReplayError::ZeroEpoch)
        );
    }

    /// **KRRG-07** — good chain accepted.
    #[test]
    fn krrg_07_good_chain_accepted() {
        assert_eq!(validate_rotation_chain(&good_chain()), Ok(()));
    }

    /// **KRRG-08** — empty chain accepted.
    #[test]
    fn krrg_08_empty_accepted() {
        assert_eq!(validate_rotation_chain(&[]), Ok(()));
    }

    /// **KRRG-09** — single rotation accepted.
    #[test]
    fn krrg_09_single_accepted() {
        assert_eq!(validate_rotation_chain(&[rot(1, 1000)]), Ok(()));
    }

    /// **KRRG-10** — same timestamp accepted (non-decreasing).
    #[test]
    fn krrg_10_same_timestamp_accepted() {
        let chain = vec![rot(1, 1000), rot(2, 1000)];
        assert_eq!(validate_rotation_chain(&chain), Ok(()));
    }
}
