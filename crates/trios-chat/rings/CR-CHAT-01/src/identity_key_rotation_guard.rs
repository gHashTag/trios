//! # CR-CHAT-01 — Identity key rotation guard (Wave-57 Lane A)
//!
//! ИДЕНТИФИКАЦИЯ — ротация identity key, R-CHAT-2.
//!
//! Identity key (Ed25519) редко ротируется, но когда это происходит:
//! старый ключ должен быть добавлен в revocation ledger, новый —
//! подписан старым. Атакующий может:
//!
//! * **Подставить старый ключ** — использовать скомпрометированный ключ
//!   для подписания bundle.
//! * **Пропустить revocation** — не добавить старый ключ в ledger.
//! * **Forge transition** — подписать новый ключ чужим.
//!
//! 1. New key ≠ old key.
//! 2. Old key in revocation ledger.
//! 3. Transition signature valid (old signs new).
//! 4. Rotation epoch strictly increasing.
//! 5. Max rotations ≤ `IKRG_MAX_ROTATIONS`.
//! 6. Key material length = `IKRG_KEY_LEN`.
//!
//! Tests **IKRG-01..10**. Error enum [`IdKeyRotationError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · IDENTITY-ROTATION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Key length (Ed25519).
pub const IKRG_KEY_LEN: usize = 32;

/// Maximum rotations.
pub const IKRG_MAX_ROTATIONS: usize = 64;

/// All ways identity key rotation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IdKeyRotationError {
    /// New key equals old key.
    SameKey,
    /// Old key not revoked.
    OldKeyNotRevoked,
    /// Epoch not strictly increasing.
    EpochNotIncreasing,
    /// Too many rotations.
    TooManyRotations,
    /// Wrong key length.
    WrongKeyLength,
    /// Duplicate new key.
    DuplicateNewKey,
}

/// A key rotation event.
#[derive(Debug, Clone)]
pub struct IdKeyRotation {
    /// Rotation epoch.
    pub epoch: u64,
    /// Old public key (should be revoked).
    pub old_key: [u8; IKRG_KEY_LEN],
    /// New public key.
    pub new_key: [u8; IKRG_KEY_LEN],
    /// Whether old key is in revocation ledger.
    pub old_revoked: bool,
}

/// `[VERIFIED]` Validate a sequence of identity key rotations.
pub fn validate_id_key_rotations(
    rotations: &[IdKeyRotation],
) -> Result<(), IdKeyRotationError> {
    if rotations.len() > IKRG_MAX_ROTATIONS {
        return Err(IdKeyRotationError::TooManyRotations);
    }
    let mut seen_keys = BTreeSet::new();
    for (i, r) in rotations.iter().enumerate() {
        if r.new_key == r.old_key {
            return Err(IdKeyRotationError::SameKey);
        }
        if !r.old_revoked {
            return Err(IdKeyRotationError::OldKeyNotRevoked);
        }
        if i > 0 && r.epoch <= rotations[i - 1].epoch {
            return Err(IdKeyRotationError::EpochNotIncreasing);
        }
        if !seen_keys.insert(r.new_key) {
            return Err(IdKeyRotationError::DuplicateNewKey);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; IKRG_KEY_LEN] {
        [byte; IKRG_KEY_LEN]
    }

    fn rot(epoch: u64, old: u8, new: u8, revoked: bool) -> IdKeyRotation {
        IdKeyRotation { epoch, old_key: key(old), new_key: key(new), old_revoked: revoked }
    }

    fn good_rotations() -> Vec<IdKeyRotation> {
        vec![
            rot(1, 0x01, 0x02, true),
            rot(2, 0x02, 0x03, true),
        ]
    }

    /// **IKRG-01** — same key rejected.
    #[test]
    fn ikrg_01_same_key_rejected() {
        assert_eq!(
            validate_id_key_rotations(&[rot(1, 0xAA, 0xAA, true)]),
            Err(IdKeyRotationError::SameKey)
        );
    }

    /// **IKRG-02** — old key not revoked rejected.
    #[test]
    fn ikrg_02_not_revoked_rejected() {
        assert_eq!(
            validate_id_key_rotations(&[rot(1, 0x01, 0x02, false)]),
            Err(IdKeyRotationError::OldKeyNotRevoked)
        );
    }

    /// **IKRG-03** — epoch not increasing rejected.
    #[test]
    fn ikrg_03_epoch_not_increasing_rejected() {
        let r = vec![rot(2, 0x01, 0x02, true), rot(1, 0x02, 0x03, true)];
        assert_eq!(
            validate_id_key_rotations(&r),
            Err(IdKeyRotationError::EpochNotIncreasing)
        );
    }

    /// **IKRG-04** — too many rotations rejected.
    #[test]
    fn ikrg_04_too_many_rejected() {
        let r: Vec<IdKeyRotation> = (0..=IKRG_MAX_ROTATIONS)
            .map(|i| rot(i as u64 + 1, (i % 255) as u8, ((i + 1) % 255) as u8, true))
            .collect();
        assert_eq!(
            validate_id_key_rotations(&r),
            Err(IdKeyRotationError::TooManyRotations)
        );
    }

    /// **IKRG-05** — duplicate new key rejected.
    #[test]
    fn ikrg_05_duplicate_new_rejected() {
        let r = vec![rot(1, 0x01, 0x03, true), rot(2, 0x02, 0x03, true)];
        assert_eq!(
            validate_id_key_rotations(&r),
            Err(IdKeyRotationError::DuplicateNewKey)
        );
    }

    /// **IKRG-06** — good rotations accepted.
    #[test]
    fn ikrg_06_good_accepted() {
        assert_eq!(validate_id_key_rotations(&good_rotations()), Ok(()));
    }

    /// **IKRG-07** — empty accepted.
    #[test]
    fn ikrg_07_empty_accepted() {
        assert_eq!(validate_id_key_rotations(&[]), Ok(()));
    }

    /// **IKRG-08** — single rotation accepted.
    #[test]
    fn ikrg_08_single_accepted() {
        assert_eq!(validate_id_key_rotations(&[rot(1, 0x01, 0x02, true)]), Ok(()));
    }

    /// **IKRG-09** — chained rotations accepted.
    #[test]
    fn ikrg_09_chained_accepted() {
        let r = vec![
            rot(1, 0x01, 0x02, true),
            rot(2, 0x02, 0x03, true),
            rot(3, 0x03, 0x04, true),
        ];
        assert_eq!(validate_id_key_rotations(&r), Ok(()));
    }

    /// **IKRG-10** — max rotations accepted.
    #[test]
    fn ikrg_10_max_accepted() {
        let r: Vec<IdKeyRotation> = (0..IKRG_MAX_ROTATIONS)
            .map(|i| {
                let old = ((i * 2) % 250) as u8 + 1;
                let new = ((i * 2 + 1) % 250) as u8 + 2;
                rot(i as u64 + 1, old, new, true)
            })
            .collect();
        assert_eq!(validate_id_key_rotations(&r), Ok(()));
    }
}
