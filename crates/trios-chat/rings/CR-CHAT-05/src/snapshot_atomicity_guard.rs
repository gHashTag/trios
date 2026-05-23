//! # CR-CHAT-05 — Snapshot atomicity guard (Wave-66 Lane A)
//!
//! PERSISTENCE — snapshots must be all-or-nothing, R-CHAT-5.
//!
//! A partial snapshot lets an attacker roll back individual fields:
//!
//! * **Selective rollback** — attacker reverts the epoch but keeps the
//!   new ratchet key, creating a fork.
//! * **Missing field** — a snapshot with fewer fields than expected means
//!   a store reload drops critical state (e.g. consumed welcome set).
//! * **Duplicate field** — two copies of the same field means one is
//!   stale and which one is loaded depends on iteration order.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Required field set is present (no missing fields).
//! 2. No duplicate fields in snapshot.
//! 3. Field count == expected count.
//! 4. Field IDs are in the valid range.
//! 5. No extra/unknown fields.
//! 6. Snapshot size <= `SNAT_MAX_SIZE`.
//!
//! Tests **SNAT-01..10**. Error enum [`SnapshotAtomicityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SNAPSHOT-ATOMICITY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum snapshot size (bytes).
pub const SNAT_MAX_SIZE: usize = 1_048_576;

/// Valid field IDs for a chat persistence snapshot.
pub const SNAT_REQUIRED_FIELDS: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05];

/// All ways snapshot atomicity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SnapshotAtomicityError {
    /// Missing required field.
    MissingField(u8),
    /// Duplicate field.
    DuplicateField(u8),
    /// Field count mismatch.
    FieldCountMismatch,
    /// Invalid field ID.
    InvalidFieldId(u8),
    /// Unknown field ID (not in required or allowed set).
    UnknownField(u8),
    /// Snapshot too large.
    SnapshotTooLarge,
}

/// `[VERIFIED]` Validate that a snapshot is atomic (all required fields, no extras).
pub fn validate_snapshot_atomicity(
    total_size: usize,
    field_ids: &[u8],
) -> Result<(), SnapshotAtomicityError> {
    if total_size > SNAT_MAX_SIZE {
        return Err(SnapshotAtomicityError::SnapshotTooLarge);
    }
    let required: BTreeSet<u8> = SNAT_REQUIRED_FIELDS.iter().copied().collect();
    if field_ids.len() != required.len() {
        return Err(SnapshotAtomicityError::FieldCountMismatch);
    }
    let mut seen = BTreeSet::new();
    for &fid in field_ids {
        if !required.contains(&fid) {
            return Err(SnapshotAtomicityError::UnknownField(fid));
        }
        if !seen.insert(fid) {
            return Err(SnapshotAtomicityError::DuplicateField(fid));
        }
    }
    for &req in &required {
        if !seen.contains(&req) {
            return Err(SnapshotAtomicityError::MissingField(req));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_fields() -> Vec<u8> {
        SNAT_REQUIRED_FIELDS.to_vec()
    }

    /// **SNAT-01** — missing field rejected.
    #[test]
    fn snat_01_missing_field_rejected() {
        let fields = vec![0x01, 0x02, 0x03, 0x04];
        assert_eq!(
            validate_snapshot_atomicity(100, &fields),
            Err(SnapshotAtomicityError::FieldCountMismatch)
        );
    }

    /// **SNAT-02** — duplicate field rejected.
    #[test]
    fn snat_02_duplicate_rejected() {
        let fields = vec![0x01, 0x02, 0x03, 0x03, 0x05];
        assert_eq!(
            validate_snapshot_atomicity(100, &fields),
            Err(SnapshotAtomicityError::DuplicateField(0x03))
        );
    }

    /// **SNAT-03** — field count mismatch rejected.
    #[test]
    fn snat_03_count_mismatch_rejected() {
        let fields = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        assert_eq!(
            validate_snapshot_atomicity(100, &fields),
            Err(SnapshotAtomicityError::FieldCountMismatch)
        );
    }

    /// **SNAT-04** — unknown field rejected.
    #[test]
    fn snat_04_unknown_field_rejected() {
        let fields = vec![0x01, 0x02, 0x03, 0x04, 0xFF];
        assert_eq!(
            validate_snapshot_atomicity(100, &fields),
            Err(SnapshotAtomicityError::UnknownField(0xFF))
        );
    }

    /// **SNAT-05** — snapshot too large rejected.
    #[test]
    fn snat_05_too_large_rejected() {
        assert_eq!(
            validate_snapshot_atomicity(SNAT_MAX_SIZE + 1, &valid_fields()),
            Err(SnapshotAtomicityError::SnapshotTooLarge)
        );
    }

    /// **SNAT-06** — valid snapshot accepted.
    #[test]
    fn snat_06_valid_accepted() {
        assert_eq!(validate_snapshot_atomicity(100, &valid_fields()), Ok(()));
    }

    /// **SNAT-07** — exact max size accepted.
    #[test]
    fn snat_07_max_size_accepted() {
        assert_eq!(
            validate_snapshot_atomicity(SNAT_MAX_SIZE, &valid_fields()),
            Ok(())
        );
    }

    /// **SNAT-08** — zero size accepted.
    #[test]
    fn snat_08_zero_size_accepted() {
        assert_eq!(validate_snapshot_atomicity(0, &valid_fields()), Ok(()));
    }

    /// **SNAT-09** — fields in different order accepted.
    #[test]
    fn snat_09_reorder_accepted() {
        let fields = vec![0x05, 0x03, 0x01, 0x04, 0x02];
        assert_eq!(validate_snapshot_atomicity(100, &fields), Ok(()));
    }

    /// **SNAT-10** — empty fields rejected.
    #[test]
    fn snat_10_empty_rejected() {
        assert_eq!(
            validate_snapshot_atomicity(0, &[]),
            Err(SnapshotAtomicityError::FieldCountMismatch)
        );
    }
}
