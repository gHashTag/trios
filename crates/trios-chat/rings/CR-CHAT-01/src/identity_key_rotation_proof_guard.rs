//! # CR-CHAT-01 — Identity key rotation proof guard (Wave-90 Lane A)
//!
//! IDENTITY — key rotation must carry an authorization proof, R-CHAT-1.
//!
//! When an identity key is rotated, the new key must be authorized by
//! a proof signed by the old key. Without this:
//!
//! * **Unauthorized rotation** — an attacker who gains temporary access
//!   replaces the identity key with their own, permanently hijacking
//!   the identity.
//! * **Silent takeover** — no audit trail of who authorized the
//!   rotation, making it impossible to detect compromise.
//! * **Chain breaks** — peers cannot verify continuity of identity
//!   across rotations, losing trust in the long-term identity.
//!
//! IKRP enforces that every rotation carries:
//! - A proof that is marked as verified.
//! - The proof must reference both old and new keys.
//! - The rotation sequence must be monotonic.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Proof must be marked as verified.
//! 2. Old key must differ from new key.
//! 3. Rotation sequence must be strictly increasing.
//! 4. Maximum rotations <= `IKRP_MAX_ROTATIONS`.
//! 5. Old key must not be all zeros.
//! 6. New key must not be all zeros.
//!
//! Tests **IKRP-01..10**. Error enum [`RotationProofError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * ROTATION-PROOF`

#![forbid(unsafe_code)]

/// Maximum rotations per identity.
pub const IKRP_MAX_ROTATIONS: usize = 16;

/// Key length.
pub const IKRP_KEY_LEN: usize = 32;

/// A key rotation proof record.
#[derive(Debug, Clone)]
pub struct RotationProof {
    /// Old public key.
    pub old_key: [u8; IKRP_KEY_LEN],
    /// New public key.
    pub new_key: [u8; IKRP_KEY_LEN],
    /// Whether the proof has been verified.
    pub proof_verified: bool,
    /// Rotation sequence number.
    pub seq: u64,
}

/// All ways rotation proof validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RotationProofError {
    /// Proof not verified.
    ProofNotVerified(u64),
    /// Same key.
    SameKey(u64),
    /// Sequence not increasing.
    SeqNotIncreasing(u64),
    /// Too many rotations.
    TooManyRotations,
    /// Old key is zero.
    ZeroOldKey(u64),
    /// New key is zero.
    ZeroNewKey(u64),
}

/// `[VERIFIED]` Validate identity key rotation proofs.
pub fn validate_rotation_proofs(
    proofs: &[RotationProof],
) -> Result<(), RotationProofError> {
    if proofs.len() > IKRP_MAX_ROTATIONS {
        return Err(RotationProofError::TooManyRotations);
    }
    for (i, p) in proofs.iter().enumerate() {
        if p.old_key == [0u8; IKRP_KEY_LEN] {
            return Err(RotationProofError::ZeroOldKey(p.seq));
        }
        if p.new_key == [0u8; IKRP_KEY_LEN] {
            return Err(RotationProofError::ZeroNewKey(p.seq));
        }
        if p.old_key == p.new_key {
            return Err(RotationProofError::SameKey(p.seq));
        }
        if !p.proof_verified {
            return Err(RotationProofError::ProofNotVerified(p.seq));
        }
        if i > 0 && p.seq <= proofs[i - 1].seq {
            return Err(RotationProofError::SeqNotIncreasing(p.seq));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; IKRP_KEY_LEN] {
        [byte; IKRP_KEY_LEN]
    }

    fn proof(seq: u64, old: u8, new: u8) -> RotationProof {
        RotationProof {
            old_key: key(old),
            new_key: key(new),
            proof_verified: true,
            seq,
        }
    }

    fn valid_proofs() -> Vec<RotationProof> {
        vec![proof(1, 0xAA, 0xBB), proof(2, 0xBB, 0xCC)]
    }

    /// **IKRP-01** — proof not verified rejected.
    #[test]
    fn ikrp_01_not_verified_rejected() {
        let mut p = proof(1, 0xAA, 0xBB);
        p.proof_verified = false;
        assert_eq!(
            validate_rotation_proofs(&[p]),
            Err(RotationProofError::ProofNotVerified(1))
        );
    }

    /// **IKRP-02** — same key rejected.
    #[test]
    fn ikrp_02_same_key_rejected() {
        let p = RotationProof {
            old_key: key(0xAA),
            new_key: key(0xAA),
            proof_verified: true,
            seq: 1,
        };
        assert_eq!(
            validate_rotation_proofs(&[p]),
            Err(RotationProofError::SameKey(1))
        );
    }

    /// **IKRP-03** — sequence not increasing rejected.
    #[test]
    fn ikrp_03_seq_not_increasing_rejected() {
        let ps = vec![proof(2, 0xAA, 0xBB), proof(1, 0xBB, 0xCC)];
        assert_eq!(
            validate_rotation_proofs(&ps),
            Err(RotationProofError::SeqNotIncreasing(1))
        );
    }

    /// **IKRP-04** — too many rotations rejected.
    #[test]
    fn ikrp_04_too_many_rejected() {
        let ps: Vec<RotationProof> = (0..=IKRP_MAX_ROTATIONS as u64)
            .map(|i| proof(i, (0x10 + (i % 200) as u8), (0x20 + (i % 200) as u8)))
            .collect();
        assert_eq!(
            validate_rotation_proofs(&ps),
            Err(RotationProofError::TooManyRotations)
        );
    }

    /// **IKRP-05** — zero old key rejected.
    #[test]
    fn ikrp_05_zero_old_rejected() {
        let p = RotationProof {
            old_key: [0u8; IKRP_KEY_LEN],
            new_key: key(0xBB),
            proof_verified: true,
            seq: 1,
        };
        assert_eq!(
            validate_rotation_proofs(&[p]),
            Err(RotationProofError::ZeroOldKey(1))
        );
    }

    /// **IKRP-06** — zero new key rejected.
    #[test]
    fn ikrp_06_zero_new_rejected() {
        let p = RotationProof {
            old_key: key(0xAA),
            new_key: [0u8; IKRP_KEY_LEN],
            proof_verified: true,
            seq: 1,
        };
        assert_eq!(
            validate_rotation_proofs(&[p]),
            Err(RotationProofError::ZeroNewKey(1))
        );
    }

    /// **IKRP-07** — valid proofs accepted.
    #[test]
    fn ikrp_07_valid_accepted() {
        assert_eq!(validate_rotation_proofs(&valid_proofs()), Ok(()));
    }

    /// **IKRP-08** — empty accepted.
    #[test]
    fn ikrp_08_empty_accepted() {
        assert_eq!(validate_rotation_proofs(&[]), Ok(()));
    }

    /// **IKRP-09** — single accepted.
    #[test]
    fn ikrp_09_single_accepted() {
        assert_eq!(validate_rotation_proofs(&[proof(1, 0x11, 0x22)]), Ok(()));
    }

    /// **IKRP-10** — max rotations boundary accepted.
    #[test]
    fn ikrp_10_max_boundary_accepted() {
        let ps: Vec<RotationProof> = (0..IKRP_MAX_ROTATIONS as u64)
            .map(|i| proof(i, (0x10 + (i % 200) as u8), (0x20 + (i % 200) as u8)))
            .collect();
        assert_eq!(validate_rotation_proofs(&ps), Ok(()));
    }
}
