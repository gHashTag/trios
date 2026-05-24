//! # CR-CHAT-05 — Store write atomicity guard (Wave-94 Lane B)
//!
//! PERSISTENCE — store writes must be atomic, R-CHAT-5.
//!
//! When writing multiple records to the store, the write must either
//! complete entirely or fail entirely. Without atomicity:
//!
//! * **Partial write** — only some records are persisted, leaving the
//!   store in an inconsistent state where related records are split.
//! * **Recovery ambiguity** — after a crash, it's unclear which
//!   records from a batch were written and which were not.
//! * **Referential integrity loss** — a record referencing another
//!   that wasn't written creates a dangling reference.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All writes in a batch must share the same batch ID.
//! 2. Batch must be marked as complete.
//! 3. Expected count must equal actual count.
//! 4. Batch size <= `SWAT_MAX_BATCH`.
//! 5. Batch ID must be > 0.
//! 6. All records must be marked as committed.
//!
//! Tests **SWAT-01..10**. Error enum [`AtomicityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * WRITE-ATOMIC`

#![forbid(unsafe_code)]

/// Maximum batch size.
pub const SWAT_MAX_BATCH: usize = 4096;

/// A write record in a batch.
#[derive(Debug, Clone)]
pub struct WriteRecord {
    /// Record ID.
    pub id: u64,
    /// Batch ID this record belongs to.
    pub batch_id: u64,
    /// Whether the record was committed.
    pub committed: bool,
}

/// A write batch summary.
#[derive(Debug, Clone)]
pub struct WriteBatch {
    /// Batch ID.
    pub batch_id: u64,
    /// Whether the batch is marked complete.
    pub complete: bool,
    /// Records in the batch.
    pub records: Vec<WriteRecord>,
    /// Expected record count.
    pub expected_count: usize,
}

/// All ways atomicity validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AtomicityError {
    /// Batch ID mismatch.
    BatchIdMismatch {
        /// Expected batch ID.
        expected: u64,
        /// Actual batch ID.
        got: u64,
    },
    /// Batch not complete.
    BatchNotComplete(u64),
    /// Count mismatch.
    CountMismatch {
        /// Batch ID.
        batch_id: u64,
        /// Expected count.
        expected: usize,
        /// Actual count.
        got: usize,
    },
    /// Batch too large.
    BatchTooLarge,
    /// Zero batch ID.
    ZeroBatchId,
    /// Record not committed.
    NotCommitted {
        /// Batch ID.
        batch_id: u64,
        /// Record ID.
        record_id: u64,
    },
}

/// `[VERIFIED]` Validate store write atomicity.
pub fn validate_write_atomicity(
    batch: &WriteBatch,
) -> Result<(), AtomicityError> {
    if batch.batch_id == 0 {
        return Err(AtomicityError::ZeroBatchId);
    }
    if batch.records.len() > SWAT_MAX_BATCH {
        return Err(AtomicityError::BatchTooLarge);
    }
    if !batch.complete {
        return Err(AtomicityError::BatchNotComplete(batch.batch_id));
    }
    if batch.records.len() != batch.expected_count {
        return Err(AtomicityError::CountMismatch {
            batch_id: batch.batch_id,
            expected: batch.expected_count,
            got: batch.records.len(),
        });
    }
    for r in &batch.records {
        if r.batch_id != batch.batch_id {
            return Err(AtomicityError::BatchIdMismatch {
                expected: batch.batch_id,
                got: r.batch_id,
            });
        }
        if !r.committed {
            return Err(AtomicityError::NotCommitted {
                batch_id: batch.batch_id,
                record_id: r.id,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: u64, batch_id: u64) -> WriteRecord {
        WriteRecord { id, batch_id, committed: true }
    }

    fn valid_batch() -> WriteBatch {
        WriteBatch {
            batch_id: 1,
            complete: true,
            records: vec![record(1, 1), record(2, 1), record(3, 1)],
            expected_count: 3,
        }
    }

    /// **SWAT-01** — batch ID mismatch rejected.
    #[test]
    fn swat_01_batch_id_mismatch_rejected() {
        let mut b = valid_batch();
        b.records[1].batch_id = 99;
        assert_eq!(
            validate_write_atomicity(&b),
            Err(AtomicityError::BatchIdMismatch { expected: 1, got: 99 })
        );
    }

    /// **SWAT-02** — batch not complete rejected.
    #[test]
    fn swat_02_not_complete_rejected() {
        let mut b = valid_batch();
        b.complete = false;
        assert_eq!(
            validate_write_atomicity(&b),
            Err(AtomicityError::BatchNotComplete(1))
        );
    }

    /// **SWAT-03** — count mismatch rejected.
    #[test]
    fn swat_03_count_mismatch_rejected() {
        let mut b = valid_batch();
        b.expected_count = 5;
        assert_eq!(
            validate_write_atomicity(&b),
            Err(AtomicityError::CountMismatch { batch_id: 1, expected: 5, got: 3 })
        );
    }

    /// **SWAT-04** — batch too large rejected.
    #[test]
    fn swat_04_too_large_rejected() {
        let records: Vec<WriteRecord> = (0..=SWAT_MAX_BATCH as u64)
            .map(|i| record(i, 1))
            .collect();
        let b = WriteBatch {
            batch_id: 1,
            complete: true,
            expected_count: records.len(),
            records,
        };
        assert_eq!(validate_write_atomicity(&b), Err(AtomicityError::BatchTooLarge));
    }

    /// **SWAT-05** — zero batch ID rejected.
    #[test]
    fn swat_05_zero_batch_rejected() {
        let mut b = valid_batch();
        b.batch_id = 0;
        assert_eq!(validate_write_atomicity(&b), Err(AtomicityError::ZeroBatchId));
    }

    /// **SWAT-06** — record not committed rejected.
    #[test]
    fn swat_06_not_committed_rejected() {
        let mut b = valid_batch();
        b.records[1].committed = false;
        assert_eq!(
            validate_write_atomicity(&b),
            Err(AtomicityError::NotCommitted { batch_id: 1, record_id: 2 })
        );
    }

    /// **SWAT-07** — valid batch accepted.
    #[test]
    fn swat_07_valid_accepted() {
        assert_eq!(validate_write_atomicity(&valid_batch()), Ok(()));
    }

    /// **SWAT-08** — empty batch accepted.
    #[test]
    fn swat_08_empty_accepted() {
        let b = WriteBatch {
            batch_id: 1,
            complete: true,
            records: vec![],
            expected_count: 0,
        };
        assert_eq!(validate_write_atomicity(&b), Ok(()));
    }

    /// **SWAT-09** — single record accepted.
    #[test]
    fn swat_09_single_accepted() {
        let b = WriteBatch {
            batch_id: 1,
            complete: true,
            records: vec![record(1, 1)],
            expected_count: 1,
        };
        assert_eq!(validate_write_atomicity(&b), Ok(()));
    }

    /// **SWAT-10** — max batch boundary accepted.
    #[test]
    fn swat_10_max_boundary_accepted() {
        let records: Vec<WriteRecord> = (0..SWAT_MAX_BATCH as u64)
            .map(|i| record(i, 1))
            .collect();
        let b = WriteBatch {
            batch_id: 1,
            complete: true,
            expected_count: records.len(),
            records,
        };
        assert_eq!(validate_write_atomicity(&b), Ok(()));
    }
}
