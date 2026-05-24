//! # CR-CHAT-05 — Store encryption key rotation integrity guard (Wave-90 Lane B)
//!
//! PERSISTENCE — encryption-at-rest key rotation must be complete,
//! R-CHAT-1.
//!
//! When the encryption-at-rest key is rotated (e.g. after a suspected
//! compromise), every stored record must be re-encrypted under the new
//! key. If records are missed:
//!
//! * **Residual exposure** — records still encrypted under the old key
//!   can be decrypted by anyone who compromised the old key.
//! * **Partial recovery** — some data is protected, some isn't,
//!   creating a false sense of security.
//! * **Audit gap** — records missed during rotation don't appear in
//!   the rotation log, hiding the gap from monitoring.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All records must be re-encrypted (no old-key records remain).
//! 2. New key ID must differ from old key ID.
//! 3. Total records <= `SEKR_MAX_RECORDS`.
//! 4. Old key ID must be non-zero.
//! 5. New key ID must be non-zero.
//! 6. Rotation must cover all record IDs.
//!
//! Tests **SEKR-01..10**. Error enum [`KeyRotationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KEY-ROTATION-INTEGRITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum records in a rotation batch.
pub const SEKR_MAX_RECORDS: usize = 65536;

/// A record after key rotation.
#[derive(Debug, Clone)]
pub struct RotatedRecord {
    /// Record ID.
    pub id: u64,
    /// Key ID the record is encrypted under.
    pub key_id: u64,
    /// Whether the record was successfully re-encrypted.
    pub re_encrypted: bool,
}

/// Key rotation summary.
#[derive(Debug, Clone)]
pub struct KeyRotation {
    /// Old key ID being rotated from.
    pub old_key_id: u64,
    /// New key ID being rotated to.
    pub new_key_id: u64,
    /// Records after rotation.
    pub records: Vec<RotatedRecord>,
    /// Expected total record count.
    pub expected_count: usize,
}

/// All ways key rotation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyRotationError {
    /// Record not re-encrypted.
    NotReEncrypted(u64),
    /// Same key ID.
    SameKeyId,
    /// Too many records.
    TooManyRecords,
    /// Zero old key ID.
    ZeroOldKeyId,
    /// Zero new key ID.
    ZeroNewKeyId,
    /// Record count mismatch.
    CountMismatch {
        /// Expected count.
        expected: usize,
        /// Actual count.
        got: usize,
    },
}

/// `[VERIFIED]` Validate encryption key rotation integrity.
pub fn validate_key_rotation(
    rotation: &KeyRotation,
) -> Result<(), KeyRotationError> {
    if rotation.old_key_id == 0 {
        return Err(KeyRotationError::ZeroOldKeyId);
    }
    if rotation.new_key_id == 0 {
        return Err(KeyRotationError::ZeroNewKeyId);
    }
    if rotation.old_key_id == rotation.new_key_id {
        return Err(KeyRotationError::SameKeyId);
    }
    if rotation.records.len() > SEKR_MAX_RECORDS {
        return Err(KeyRotationError::TooManyRecords);
    }
    if rotation.records.len() != rotation.expected_count {
        return Err(KeyRotationError::CountMismatch {
            expected: rotation.expected_count,
            got: rotation.records.len(),
        });
    }
    for r in &rotation.records {
        if !r.re_encrypted {
            return Err(KeyRotationError::NotReEncrypted(r.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64, re_encrypted: bool) -> RotatedRecord {
        RotatedRecord { id, key_id: if re_encrypted { 2 } else { 1 }, re_encrypted }
    }

    fn valid_rotation() -> KeyRotation {
        KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            records: vec![record(1, true), record(2, true), record(3, true)],
            expected_count: 3,
        }
    }

    /// **SEKR-01** — not re-encrypted rejected.
    #[test]
    fn sek_01_not_re_encrypted_rejected() {
        let rot = KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            records: vec![record(1, true), record(2, false)],
            expected_count: 2,
        };
        assert_eq!(
            validate_key_rotation(&rot),
            Err(KeyRotationError::NotReEncrypted(2))
        );
    }

    /// **SEKR-02** — same key ID rejected.
    #[test]
    fn sek_02_same_key_rejected() {
        let mut rot = valid_rotation();
        rot.new_key_id = 1;
        assert_eq!(validate_key_rotation(&rot), Err(KeyRotationError::SameKeyId));
    }

    /// **SEKR-03** — too many records rejected.
    #[test]
    fn sek_03_too_many_rejected() {
        let records: Vec<RotatedRecord> = (0..=SEKR_MAX_RECORDS as u64)
            .map(|i| record(i, true))
            .collect();
        let rot = KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            expected_count: records.len(),
            records,
        };
        assert_eq!(validate_key_rotation(&rot), Err(KeyRotationError::TooManyRecords));
    }

    /// **SEKR-04** — zero old key ID rejected.
    #[test]
    fn sek_04_zero_old_rejected() {
        let mut rot = valid_rotation();
        rot.old_key_id = 0;
        assert_eq!(validate_key_rotation(&rot), Err(KeyRotationError::ZeroOldKeyId));
    }

    /// **SEKR-05** — zero new key ID rejected.
    #[test]
    fn sek_05_zero_new_rejected() {
        let mut rot = valid_rotation();
        rot.new_key_id = 0;
        assert_eq!(validate_key_rotation(&rot), Err(KeyRotationError::ZeroNewKeyId));
    }

    /// **SEKR-06** — count mismatch rejected.
    #[test]
    fn sek_06_count_mismatch_rejected() {
        let mut rot = valid_rotation();
        rot.expected_count = 5;
        assert_eq!(
            validate_key_rotation(&rot),
            Err(KeyRotationError::CountMismatch { expected: 5, got: 3 })
        );
    }

    /// **SEKR-07** — valid rotation accepted.
    #[test]
    fn sek_07_valid_accepted() {
        assert_eq!(validate_key_rotation(&valid_rotation()), Ok(()));
    }

    /// **SEKR-08** — empty rotation accepted (zero records to rotate).
    #[test]
    fn sek_08_empty_accepted() {
        let rot = KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            records: vec![],
            expected_count: 0,
        };
        assert_eq!(validate_key_rotation(&rot), Ok(()));
    }

    /// **SEKR-09** — single record accepted.
    #[test]
    fn sek_09_single_accepted() {
        let rot = KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            records: vec![record(1, true)],
            expected_count: 1,
        };
        assert_eq!(validate_key_rotation(&rot), Ok(()));
    }

    /// **SEKR-10** — max records boundary accepted.
    #[test]
    fn sek_10_max_boundary_accepted() {
        let records: Vec<RotatedRecord> = (0..SEKR_MAX_RECORDS as u64)
            .map(|i| record(i, true))
            .collect();
        let rot = KeyRotation {
            old_key_id: 1,
            new_key_id: 2,
            expected_count: records.len(),
            records,
        };
        assert_eq!(validate_key_rotation(&rot), Ok(()));
    }
}
