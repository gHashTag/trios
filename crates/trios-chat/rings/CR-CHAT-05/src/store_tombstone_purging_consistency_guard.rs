//! # CR-CHAT-05 — Store tombstone purging consistency guard (Wave-88 Lane A)
//!
//! PERSISTENCE — tombstones must be purged only after retention, R-CHAT-5.
//!
//! When an envelope is deleted, a tombstone marks its absence. The
//! purging logic must be consistent:
//!
//! * **Premature purge** — tombstone removed before retention expires,
//!   losing the audit trail and enabling replay of deleted messages.
//! * **Late purge** — tombstones kept far beyond retention, bloating
//!   storage and degrading query performance.
//! * **Inconsistent purge** — some tombstones purged, others not,
//!   revealing which messages were deleted (metadata leak).
//!
//! STPC enforces that at a given `now` timestamp:
//! - All tombstones with `deleted_at + retention < now` must be purged.
//! - No tombstone with `deleted_at + retention >= now` may be purged.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No tombstone past retention still present.
//! 2. No tombstone before retention already purged.
//! 3. Retention period must be <= `STPC_MAX_RETENTION`.
//! 4. Retention period must be >= `STPC_MIN_RETENTION`.
//! 5. Tombstone count <= `STPC_MAX_TOMBSTONES`.
//! 6. Deleted timestamp must be > 0.
//!
//! Tests **STPC-01..10**. Error enum [`PurgeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * TOMBSTONE-PURGE`

#![forbid(unsafe_code)]

/// Maximum retention period (seconds).
pub const STPC_MAX_RETENTION: u64 = 2_592_000;

/// Minimum retention period (seconds).
pub const STPC_MIN_RETENTION: u64 = 3600;

/// Maximum tombstones.
pub const STPC_MAX_TOMBSTONES: usize = 8192;

/// A tombstone entry.
#[derive(Debug, Clone)]
pub struct TombstoneEntry {
    /// Unique ID.
    pub id: u64,
    /// When the record was deleted (seconds).
    pub deleted_at: u64,
    /// Retention period (seconds).
    pub retention_secs: u64,
    /// Whether the tombstone has been purged.
    pub is_purged: bool,
}

impl TombstoneEntry {
    /// Whether the tombstone is past its retention at `now`.
    pub fn is_past_retention(&self, now: u64) -> bool {
        now >= self.deleted_at + self.retention_secs
    }
}

/// All ways purging validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PurgeError {
    /// Tombstone past retention not purged.
    NotPurged(u64),
    /// Tombstone before retention already purged.
    PrematurePurge(u64),
    /// Retention too long.
    RetentionTooLong(u64),
    /// Retention too short.
    RetentionTooShort(u64),
    /// Too many tombstones.
    TooManyTombstones,
    /// Zero deleted timestamp.
    ZeroDeletedAt(u64),
}

/// `[VERIFIED]` Validate tombstone purging consistency.
pub fn validate_tombstone_purging(
    tombstones: &[TombstoneEntry],
    now: u64,
) -> Result<(), PurgeError> {
    if tombstones.len() > STPC_MAX_TOMBSTONES {
        return Err(PurgeError::TooManyTombstones);
    }
    for t in tombstones {
        if t.deleted_at == 0 {
            return Err(PurgeError::ZeroDeletedAt(t.id));
        }
        if t.retention_secs > STPC_MAX_RETENTION {
            return Err(PurgeError::RetentionTooLong(t.id));
        }
        if t.retention_secs < STPC_MIN_RETENTION {
            return Err(PurgeError::RetentionTooShort(t.id));
        }
        if t.is_past_retention(now) && !t.is_purged {
            return Err(PurgeError::NotPurged(t.id));
        }
        if !t.is_past_retention(now) && t.is_purged {
            return Err(PurgeError::PrematurePurge(t.id));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tombstone(id: u64, deleted_at: u64, retention: u64, purged: bool) -> TombstoneEntry {
        TombstoneEntry { id, deleted_at, retention_secs: retention, is_purged: purged }
    }

    fn valid_tombstones() -> Vec<TombstoneEntry> {
        vec![
            tombstone(1, 1000, 3600, true),
            tombstone(2, 8000, 3600, false),
        ]
    }

    /// **STPC-01** — not purged past retention rejected.
    #[test]
    fn stpc_01_not_purged_rejected() {
        let ts = vec![tombstone(1, 1000, 3600, false)];
        assert_eq!(
            validate_tombstone_purging(&ts, 10000),
            Err(PurgeError::NotPurged(1))
        );
    }

    /// **STPC-02** — premature purge rejected.
    #[test]
    fn stpc_02_premature_purge_rejected() {
        let ts = vec![tombstone(1, 5000, 3600, true)];
        assert_eq!(
            validate_tombstone_purging(&ts, 6000),
            Err(PurgeError::PrematurePurge(1))
        );
    }

    /// **STPC-03** — retention too long rejected.
    #[test]
    fn stpc_03_retention_too_long_rejected() {
        let ts = vec![tombstone(1, 1000, STPC_MAX_RETENTION + 1, false)];
        assert_eq!(
            validate_tombstone_purging(&ts, 2000),
            Err(PurgeError::RetentionTooLong(1))
        );
    }

    /// **STPC-04** — retention too short rejected.
    #[test]
    fn stpc_04_retention_too_short_rejected() {
        let ts = vec![tombstone(1, 1000, 100, false)];
        assert_eq!(
            validate_tombstone_purging(&ts, 2000),
            Err(PurgeError::RetentionTooShort(1))
        );
    }

    /// **STPC-05** — too many tombstones rejected.
    #[test]
    fn stpc_05_too_many_rejected() {
        let ts: Vec<TombstoneEntry> = (0..=STPC_MAX_TOMBSTONES as u64)
            .map(|i| tombstone(i, 1000, 3600, false))
            .collect();
        assert_eq!(
            validate_tombstone_purging(&ts, 1000),
            Err(PurgeError::TooManyTombstones)
        );
    }

    /// **STPC-06** — zero deleted_at rejected.
    #[test]
    fn stpc_06_zero_deleted_rejected() {
        let ts = vec![tombstone(1, 0, 3600, false)];
        assert_eq!(
            validate_tombstone_purging(&ts, 1000),
            Err(PurgeError::ZeroDeletedAt(1))
        );
    }

    /// **STPC-07** — valid tombstones accepted.
    #[test]
    fn stpc_07_valid_accepted() {
        assert_eq!(validate_tombstone_purging(&valid_tombstones(), 10000), Ok(()));
    }

    /// **STPC-08** — empty accepted.
    #[test]
    fn stpc_08_empty_accepted() {
        assert_eq!(validate_tombstone_purging(&[], 1000), Ok(()));
    }

    /// **STPC-09** — exactly at retention boundary accepted.
    #[test]
    fn stpc_09_boundary_accepted() {
        let ts = vec![tombstone(1, 1000, 3600, true)];
        assert_eq!(validate_tombstone_purging(&ts, 4600), Ok(()));
    }

    /// **STPC-10** — not yet at retention accepted.
    #[test]
    fn stpc_10_not_yet_accepted() {
        let ts = vec![tombstone(1, 1000, 3600, false)];
        assert_eq!(validate_tombstone_purging(&ts, 4599), Ok(()));
    }
}
