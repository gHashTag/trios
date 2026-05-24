//! # CR-CHAT-02 — Root key derivation salt uniqueness guard (Wave-108 Lane B)
//!
//! RATCHET — root key derivation salts must be unique.
//!
//! Each DH ratchet step produces a new root key via HKDF with a salt
//! derived from the DH output. If salts are reused:
//!
//! * **Cross-session correlation** — the same salt produces the same
//!   root key from the same DH output, linking sessions.
//! * **Key collision** — two independent ratchet sessions derive the
//!   same root key, violating the uniqueness assumption.
//! * **Forward secrecy weakened** — if the salt is predictable, an
//!   adversary who compromises one root key can predict future salts.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate salts.
//! 2. Salt must not be all zeros.
//! 3. Salt length must be `RKDS_SALT_LEN`.
//! 4. Associated session ID must not be zero.
//! 5. Salt must not equal any previous root key.
//! 6. Total derivations <= `RKDS_MAX_DERIVATIONS`.
//!
//! Tests **RKDS-01..10**. Error enum [`SaltUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SALT-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Salt length.
pub const RKDS_SALT_LEN: usize = 32;

/// Session ID length.
pub const RKDS_SESSION_ID_LEN: usize = 32;

/// Maximum derivations per batch.
pub const RKDS_MAX_DERIVATIONS: usize = 1024;

/// A salt derivation record.
#[derive(Debug, Clone)]
pub struct SaltDerivation {
    /// Session identifier.
    pub session_id: [u8; RKDS_SESSION_ID_LEN],
    /// Salt used in derivation.
    pub salt: [u8; RKDS_SALT_LEN],
    /// Previous root key (to check salt ≠ root key).
    pub prev_root_key: [u8; RKDS_SALT_LEN],
}

/// All ways salt uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SaltUniquenessError {
    /// Duplicate salt.
    DuplicateSalt(usize),
    /// Zero salt.
    ZeroSalt(usize),
    /// Zero session ID.
    ZeroSession(usize),
    /// Salt equals previous root key.
    SaltEqualsRootKey(usize),
    /// Too many derivations.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate root key derivation salt uniqueness.
pub fn validate_salt_uniqueness(
    derivations: &[SaltDerivation],
) -> Result<(), SaltUniquenessError> {
    if derivations.len() > RKDS_MAX_DERIVATIONS {
        return Err(SaltUniquenessError::TooMany {
            got: derivations.len(),
            max: RKDS_MAX_DERIVATIONS,
        });
    }
    let mut seen: BTreeSet<[u8; RKDS_SALT_LEN]> = BTreeSet::new();
    for (i, d) in derivations.iter().enumerate() {
        if d.session_id == [0u8; RKDS_SESSION_ID_LEN] {
            return Err(SaltUniquenessError::ZeroSession(i));
        }
        if d.salt == [0u8; RKDS_SALT_LEN] {
            return Err(SaltUniquenessError::ZeroSalt(i));
        }
        if d.salt == d.prev_root_key {
            return Err(SaltUniquenessError::SaltEqualsRootKey(i));
        }
        if !seen.insert(d.salt) {
            return Err(SaltUniquenessError::DuplicateSalt(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sid(byte: u8) -> [u8; RKDS_SESSION_ID_LEN] {
        [byte; RKDS_SESSION_ID_LEN]
    }

    fn arr(byte: u8) -> [u8; RKDS_SALT_LEN] {
        [byte; RKDS_SALT_LEN]
    }

    fn derivation(session: u8, salt: u8, prev: u8) -> SaltDerivation {
        SaltDerivation { session_id: sid(session), salt: arr(salt), prev_root_key: arr(prev) }
    }

    fn valid_derivations() -> Vec<SaltDerivation> {
        vec![
            derivation(0x01, 0xAA, 0x10),
            derivation(0x01, 0xBB, 0x20),
            derivation(0x02, 0xCC, 0x30),
        ]
    }

    /// **RKDS-01** — duplicate salt rejected.
    #[test]
    fn rkds_01_duplicate_rejected() {
        let ds = vec![derivation(0x01, 0xAA, 0x10), derivation(0x01, 0xAA, 0x20)];
        assert_eq!(
            validate_salt_uniqueness(&ds),
            Err(SaltUniquenessError::DuplicateSalt(1))
        );
    }

    /// **RKDS-02** — zero salt rejected.
    #[test]
    fn rkds_02_zero_salt_rejected() {
        let d = SaltDerivation { session_id: sid(0x01), salt: [0u8; RKDS_SALT_LEN], prev_root_key: arr(0x10) };
        assert_eq!(
            validate_salt_uniqueness(&[d]),
            Err(SaltUniquenessError::ZeroSalt(0))
        );
    }

    /// **RKDS-03** — zero session rejected.
    #[test]
    fn rkds_03_zero_session_rejected() {
        let d = SaltDerivation { session_id: [0u8; RKDS_SESSION_ID_LEN], salt: arr(0xAA), prev_root_key: arr(0x10) };
        assert_eq!(
            validate_salt_uniqueness(&[d]),
            Err(SaltUniquenessError::ZeroSession(0))
        );
    }

    /// **RKDS-04** — salt equals root key rejected.
    #[test]
    fn rkds_04_salt_equals_root_rejected() {
        let d = derivation(0x01, 0xAA, 0xAA);
        assert_eq!(
            validate_salt_uniqueness(&[d]),
            Err(SaltUniquenessError::SaltEqualsRootKey(0))
        );
    }

    /// **RKDS-05** — too many rejected.
    #[test]
    fn rkds_05_too_many_rejected() {
        let ds: Vec<SaltDerivation> = (0..=RKDS_MAX_DERIVATIONS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                SaltDerivation { session_id: sid(b), salt: arr(b), prev_root_key: arr(0xFF) }
            })
            .collect();
        assert_eq!(
            validate_salt_uniqueness(&ds),
            Err(SaltUniquenessError::TooMany {
                got: RKDS_MAX_DERIVATIONS + 1,
                max: RKDS_MAX_DERIVATIONS,
            })
        );
    }

    /// **RKDS-06** — same salt different session rejected (salt is global).
    #[test]
    fn rkds_06_same_salt_diff_session_rejected() {
        let ds = vec![derivation(0x01, 0xAA, 0x10), derivation(0x02, 0xAA, 0x20)];
        assert_eq!(
            validate_salt_uniqueness(&ds),
            Err(SaltUniquenessError::DuplicateSalt(1))
        );
    }

    /// **RKDS-07** — valid accepted.
    #[test]
    fn rkds_07_valid_accepted() {
        assert_eq!(validate_salt_uniqueness(&valid_derivations()), Ok(()));
    }

    /// **RKDS-08** — empty accepted.
    #[test]
    fn rkds_08_empty_accepted() {
        assert_eq!(validate_salt_uniqueness(&[]), Ok(()));
    }

    /// **RKDS-09** — single accepted.
    #[test]
    fn rkds_09_single_accepted() {
        let ds = vec![derivation(0x01, 0xAA, 0x10)];
        assert_eq!(validate_salt_uniqueness(&ds), Ok(()));
    }

    /// **RKDS-10** — max boundary accepted.
    #[test]
    fn rkds_10_max_boundary_accepted() {
        let ds: Vec<SaltDerivation> = (0..RKDS_MAX_DERIVATIONS)
            .map(|i| {
                let mut session = [0u8; RKDS_SESSION_ID_LEN];
                let mut salt = [0u8; RKDS_SALT_LEN];
                let mut prev = [0u8; RKDS_SALT_LEN];
                let val = (i as u64) + 1;
                session[0..8].copy_from_slice(&val.to_be_bytes());
                salt[0..8].copy_from_slice(&val.to_be_bytes());
                prev[0] = 0xFF;
                SaltDerivation { session_id: session, salt, prev_root_key: prev }
            })
            .collect();
        assert_eq!(validate_salt_uniqueness(&ds), Ok(()));
    }
}
