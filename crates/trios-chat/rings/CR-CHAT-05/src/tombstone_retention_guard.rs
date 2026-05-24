//! # CR-CHAT-05 — Tombstone retention guard (Wave-71 Lane A)
//!
//! PERSISTENCE — tombstones must be retained for a minimum period, R-CHAT-5.
//!
//! When a record is deleted, a tombstone marks it as deleted. If the
//! tombstone is purged too early:
//!
//! * **Resurrected record** — a late-arriving replication packet
//!   re-inserts the deleted record because the tombstone is gone.
//! * **Compaction hazard** — tombstone purged before all replicas
//!   have acknowledged the deletion.
//! * **Audit gap** — compliance requires deletion records to exist
//!   for a minimum period.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Tombstone age >= `TSRT_MIN_RETENTION_SECS`.
//! 2. Tombstone age <= `TSRT_MAX_RETENTION_SECS`.
//! 3. Tombstone timestamp <= now.
//! 4. Tombstone has a valid key (non-empty).
//! 5. No duplicate tombstone keys.
//! 6. Tombstone count <= `TSRT_MAX_TOMBSTONES`.
//!
//! Tests **TSRT-01..10**. Error enum [`TombstoneRetentionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOMBSTONE-RETENTION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum tombstone retention (seconds).
pub const TSRT_MIN_RETENTION_SECS: u64 = 3600;

/// Maximum tombstone retention (seconds).
pub const TSRT_MAX_RETENTION_SECS: u64 = 30 * 24 * 3600;

/// Maximum tombstones in a batch.
pub const TSRT_MAX_TOMBSTONES: usize = 1024;

/// A tombstone entry.
#[derive(Debug, Clone)]
pub struct Tombstone {
    /// Tombstone key (deleted record identifier).
    pub key: Vec<u8>,
    /// Deletion timestamp (seconds since epoch).
    pub deleted_at: u64,
}

/// All ways tombstone retention validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TombstoneRetentionError {
    /// Retention period too short (not yet eligible for purge).
    RetentionTooShort,
    /// Retention period too long.
    RetentionTooLong,
    /// Future timestamp.
    FutureTimestamp,
    /// Empty key.
    EmptyKey,
    /// Duplicate key.
    DuplicateKey,
    /// Too many tombstones.
    TooManyTombstones,
}

/// `[VERIFIED]` Validate tombstone retention period.
pub fn validate_tombstone_retention(
    now_secs: u64,
    tombstones: &[Tombstone],
) -> Result<(), TombstoneRetentionError> {
    if tombstones.len() > TSRT_MAX_TOMBSTONES {
        return Err(TombstoneRetentionError::TooManyTombstones);
    }
    let mut seen = BTreeSet::new();
    for ts in tombstones {
        if ts.key.is_empty() {
            return Err(TombstoneRetentionError::EmptyKey);
        }
        if !seen.insert(ts.key.clone()) {
            return Err(TombstoneRetentionError::DuplicateKey);
        }
        if ts.deleted_at > now_secs {
            return Err(TombstoneRetentionError::FutureTimestamp);
        }
        let age = now_secs - ts.deleted_at;
        if age < TSRT_MIN_RETENTION_SECS {
            return Err(TombstoneRetentionError::RetentionTooShort);
        }
        if age > TSRT_MAX_RETENTION_SECS {
            return Err(TombstoneRetentionError::RetentionTooLong);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 10_000_000;

    fn valid_tombstone() -> Tombstone {
        Tombstone { key: vec![0x01], deleted_at: NOW - 7200 }
    }

    /// **TSRT-01** — retention too short rejected.
    #[test]
    fn tsrt_01_too_short_rejected() {
        let ts = Tombstone { key: vec![0x01], deleted_at: NOW - 100 };
        assert_eq!(
            validate_tombstone_retention(NOW, &[ts]),
            Err(TombstoneRetentionError::RetentionTooShort)
        );
    }

    /// **TSRT-02** — retention too long rejected.
    #[test]
    fn tsrt_02_too_long_rejected() {
        let ts = Tombstone {
            key: vec![0x01],
            deleted_at: NOW - TSRT_MAX_RETENTION_SECS - 1,
        };
        assert_eq!(
            validate_tombstone_retention(NOW, &[ts]),
            Err(TombstoneRetentionError::RetentionTooLong)
        );
    }

    /// **TSRT-03** — future timestamp rejected.
    #[test]
    fn tsrt_03_future_rejected() {
        let ts = Tombstone { key: vec![0x01], deleted_at: NOW + 1000 };
        assert_eq!(
            validate_tombstone_retention(NOW, &[ts]),
            Err(TombstoneRetentionError::FutureTimestamp)
        );
    }

    /// **TSRT-04** — empty key rejected.
    #[test]
    fn tsrt_04_empty_key_rejected() {
        let ts = Tombstone { key: vec![], deleted_at: NOW - 7200 };
        assert_eq!(
            validate_tombstone_retention(NOW, &[ts]),
            Err(TombstoneRetentionError::EmptyKey)
        );
    }

    /// **TSRT-05** — duplicate key rejected.
    #[test]
    fn tsrt_05_duplicate_rejected() {
        let ts1 = Tombstone { key: vec![0x01], deleted_at: NOW - 7200 };
        let ts2 = Tombstone { key: vec![0x01], deleted_at: NOW - 8000 };
        assert_eq!(
            validate_tombstone_retention(NOW, &[ts1, ts2]),
            Err(TombstoneRetentionError::DuplicateKey)
        );
    }

    /// **TSRT-06** — too many tombstones rejected.
    #[test]
    fn tsrt_06_too_many_rejected() {
        let tombstones: Vec<Tombstone> = (0..=TSRT_MAX_TOMBSTONES)
            .map(|i| Tombstone { key: vec![i as u8], deleted_at: NOW - 7200 })
            .collect();
        assert_eq!(
            validate_tombstone_retention(NOW, &tombstones),
            Err(TombstoneRetentionError::TooManyTombstones)
        );
    }

    /// **TSRT-07** — valid tombstone accepted.
    #[test]
    fn tsrt_07_valid_accepted() {
        assert_eq!(validate_tombstone_retention(NOW, &[valid_tombstone()]), Ok(()));
    }

    /// **TSRT-08** — exact min retention accepted.
    #[test]
    fn tsrt_08_min_retention_accepted() {
        let ts = Tombstone {
            key: vec![0x01],
            deleted_at: NOW - TSRT_MIN_RETENTION_SECS,
        };
        assert_eq!(validate_tombstone_retention(NOW, &[ts]), Ok(()));
    }

    /// **TSRT-09** — exact max retention accepted.
    #[test]
    fn tsrt_09_max_retention_accepted() {
        let ts = Tombstone {
            key: vec![0x01],
            deleted_at: NOW - TSRT_MAX_RETENTION_SECS,
        };
        assert_eq!(validate_tombstone_retention(NOW, &[ts]), Ok(()));
    }

    /// **TSRT-10** — empty batch accepted.
    #[test]
    fn tsrt_10_empty_accepted() {
        assert_eq!(validate_tombstone_retention(NOW, &[]), Ok(()));
    }
}
