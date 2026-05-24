//! # CR-CHAT-05 — Store record deletion integrity guard (Wave-98 Lane A)
//!
//! PERSISTENCE — deleted records must be irrecoverable, R-CHAT-1.
//!
//! When a record is deleted from the store, it must be truly gone:
//!
//! * **Forensic recovery** — if deleted data remains on disk (e.g. in
//!   free pages), an attacker with physical access can recover it.
//! * **Snapshot leakage** — backup snapshots taken before secure
//!   deletion retain the "deleted" data indefinitely.
//! * **Index residue** — the record is removed from the primary index
//!   but remains in a secondary index, enabling data reconstruction.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Deleted record must be marked as purged.
//! 2. Ciphertext must be zeroed.
//! 3. Deletion timestamp must be > 0.
//! 4. Record ID must be > 0.
//! 5. Deletion must have a verified signature.
//! 6. Batch size <= `SRDI_MAX_RECORDS`.
//!
//! Tests **SRDI-01..10**. Error enum [`DeletionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DELETE-INTEGRITY`

#![forbid(unsafe_code)]

/// Maximum records per batch.
pub const SRDI_MAX_RECORDS: usize = 4096;

/// A deleted record verification entry.
#[derive(Debug, Clone)]
pub struct DeletedRecord {
    /// Record ID.
    pub id: u64,
    /// Whether the record is marked as purged.
    pub purged: bool,
    /// Whether the ciphertext has been zeroed.
    pub ciphertext_zeroed: bool,
    /// Deletion timestamp (seconds).
    pub deleted_at: u64,
    /// Whether the deletion was signed.
    pub deletion_signed: bool,
}

/// All ways deletion validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DeletionError {
    /// Not purged.
    NotPurged(u64),
    /// Ciphertext not zeroed.
    CiphertextNotZeroed(u64),
    /// Zero timestamp.
    ZeroTimestamp(u64),
    /// Zero ID.
    ZeroId,
    /// Not signed.
    NotSigned(u64),
    /// Too many records.
    TooManyRecords,
}

/// `[VERIFIED]` Validate store record deletion integrity.
pub fn validate_deletion_integrity(
    records: &[DeletedRecord],
) -> Result<(), DeletionError> {
    if records.len() > SRDI_MAX_RECORDS {
        return Err(DeletionError::TooManyRecords);
    }
    for r in records {
        if r.id == 0 {
            return Err(DeletionError::ZeroId);
        }
        if r.deleted_at == 0 {
            return Err(DeletionError::ZeroTimestamp(r.id));
        }
        if !r.deletion_signed {
            return Err(DeletionError::NotSigned(r.id));
        }
        if !r.purged {
            return Err(DeletionError::NotPurged(r.id));
        }
        if !r.ciphertext_zeroed {
            return Err(DeletionError::CiphertextNotZeroed(r.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: u64, purged: bool, ct_zeroed: bool, signed: bool) -> DeletedRecord {
        DeletedRecord {
            id,
            purged,
            ciphertext_zeroed: ct_zeroed,
            deleted_at: 1000,
            deletion_signed: signed,
        }
    }

    fn valid_records() -> Vec<DeletedRecord> {
        vec![rec(1, true, true, true), rec(2, true, true, true)]
    }

    /// **SRDI-01** — not purged rejected.
    #[test]
    fn srdi_01_not_purged_rejected() {
        assert_eq!(
            validate_deletion_integrity(&[rec(1, false, true, true)]),
            Err(DeletionError::NotPurged(1))
        );
    }

    /// **SRDI-02** — ciphertext not zeroed rejected.
    #[test]
    fn srdi_02_ct_not_zeroed_rejected() {
        assert_eq!(
            validate_deletion_integrity(&[rec(1, true, false, true)]),
            Err(DeletionError::CiphertextNotZeroed(1))
        );
    }

    /// **SRDI-03** — zero timestamp rejected.
    #[test]
    fn srdi_03_zero_ts_rejected() {
        let mut r = rec(1, true, true, true);
        r.deleted_at = 0;
        assert_eq!(
            validate_deletion_integrity(&[r]),
            Err(DeletionError::ZeroTimestamp(1))
        );
    }

    /// **SRDI-04** — zero ID rejected.
    #[test]
    fn srdi_04_zero_id_rejected() {
        let r = DeletedRecord { id: 0, purged: true, ciphertext_zeroed: true, deleted_at: 1000, deletion_signed: true };
        assert_eq!(validate_deletion_integrity(&[r]), Err(DeletionError::ZeroId));
    }

    /// **SRDI-05** — not signed rejected.
    #[test]
    fn srdi_05_not_signed_rejected() {
        assert_eq!(
            validate_deletion_integrity(&[rec(1, true, true, false)]),
            Err(DeletionError::NotSigned(1))
        );
    }

    /// **SRDI-06** — too many records rejected.
    #[test]
    fn srdi_06_too_many_rejected() {
        let rs: Vec<DeletedRecord> = (1..=SRDI_MAX_RECORDS as u64 + 1)
            .map(|i| rec(i, true, true, true))
            .collect();
        assert_eq!(validate_deletion_integrity(&rs), Err(DeletionError::TooManyRecords));
    }

    /// **SRDI-07** — valid records accepted.
    #[test]
    fn srdi_07_valid_accepted() {
        assert_eq!(validate_deletion_integrity(&valid_records()), Ok(()));
    }

    /// **SRDI-08** — empty accepted.
    #[test]
    fn srdi_08_empty_accepted() {
        assert_eq!(validate_deletion_integrity(&[]), Ok(()));
    }

    /// **SRDI-09** — single accepted.
    #[test]
    fn srdi_09_single_accepted() {
        assert_eq!(validate_deletion_integrity(&[rec(1, true, true, true)]), Ok(()));
    }

    /// **SRDI-10** — max boundary accepted.
    #[test]
    fn srdi_10_max_boundary_accepted() {
        let rs: Vec<DeletedRecord> = (1..=SRDI_MAX_RECORDS as u64)
            .map(|i| rec(i, true, true, true))
            .collect();
        assert_eq!(validate_deletion_integrity(&rs), Ok(()));
    }
}
