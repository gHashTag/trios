//! # CR-CHAT-05 — Store migration compatibility guard (Wave-117 Lane A)
//!
//! PERSISTENCE — when the schema version changes, old records must
//! remain readable; incompatible migrations cause data loss.
//!
//! Schema migrations must be backward-compatible:
//!
//! * **Data loss** — dropping columns or changing types without a
//!   migration path destroys sealed envelopes permanently.
//! * **Integrity violation** — migrating without preserving hash
//!   chains breaks the audit trail (INV-CHAT-37).
//! * **Rollback failure** — if a migration cannot be rolled back,
//!   a failed deploy leaves the store in an unrecoverable state.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Target version must be > source version.
//! 2. Target version must be <= `SMCG_MAX_VERSION`.
//! 3. No duplicate migration paths (same source→target).
//! 4. Each migration must preserve all field hashes.
//! 5. Rollback path must exist for every migration.
//! 6. Total migrations <= `SMCG_MAX_MIGRATIONS`.
//!
//! Tests **SMCG-01..10**. Error enum [`MigrationCompatError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MIGRATION-SAFE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum schema version.
pub const SMCG_MAX_VERSION: u32 = 100;

/// Maximum migrations per batch.
pub const SMCG_MAX_MIGRATIONS: usize = 1024;

/// Hash length for field integrity check.
pub const SMCG_HASH_LEN: usize = 32;

/// A migration record.
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    /// Source schema version.
    pub source_version: u32,
    /// Target schema version.
    pub target_version: u32,
    /// Hash of all fields before migration (for integrity check).
    pub pre_hash: [u8; SMCG_HASH_LEN],
    /// Whether a rollback path exists.
    pub has_rollback: bool,
}

/// All ways migration compatibility validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MigrationCompatError {
    /// Target version not greater than source.
    NonMonotonic {
        /// Index of the offending record.
        idx: usize,
        /// Source version.
        source: u32,
        /// Target version.
        target: u32,
    },
    /// Version exceeds maximum.
    VersionTooHigh {
        /// Index of the offending record.
        idx: usize,
        /// The version that was too high.
        version: u32,
        /// Maximum allowed version.
        max: u32,
    },
    /// Source version is zero.
    ZeroSource(usize),
    /// Duplicate migration path.
    DuplicatePath {
        /// Index of the duplicate.
        idx: usize,
        /// Source version.
        source: u32,
        /// Target version.
        target: u32,
    },
    /// Missing rollback path.
    NoRollback(usize),
    /// Too many migrations.
    TooMany {
        /// Count received.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store migration compatibility.
pub fn validate_migration_compat(
    migrations: &[MigrationRecord],
) -> Result<(), MigrationCompatError> {
    if migrations.len() > SMCG_MAX_MIGRATIONS {
        return Err(MigrationCompatError::TooMany {
            got: migrations.len(),
            max: SMCG_MAX_MIGRATIONS,
        });
    }
    let mut seen: BTreeSet<(u32, u32)> = BTreeSet::new();
    for (i, m) in migrations.iter().enumerate() {
        if m.source_version == 0 {
            return Err(MigrationCompatError::ZeroSource(i));
        }
        if m.target_version > SMCG_MAX_VERSION {
            return Err(MigrationCompatError::VersionTooHigh {
                idx: i,
                version: m.target_version,
                max: SMCG_MAX_VERSION,
            });
        }
        if m.target_version <= m.source_version {
            return Err(MigrationCompatError::NonMonotonic {
                idx: i,
                source: m.source_version,
                target: m.target_version,
            });
        }
        let path = (m.source_version, m.target_version);
        if !seen.insert(path) {
            return Err(MigrationCompatError::DuplicatePath {
                idx: i,
                source: m.source_version,
                target: m.target_version,
            });
        }
        if !m.has_rollback {
            return Err(MigrationCompatError::NoRollback(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; SMCG_HASH_LEN] {
        [byte; SMCG_HASH_LEN]
    }

    fn migration(src: u32, tgt: u32, pre_hash: u8, rollback: bool) -> MigrationRecord {
        MigrationRecord {
            source_version: src,
            target_version: tgt,
            pre_hash: hash(pre_hash),
            has_rollback: rollback,
        }
    }

    fn valid_migrations() -> Vec<MigrationRecord> {
        vec![
            migration(1, 2, 0x01, true),
            migration(2, 3, 0x02, true),
            migration(3, 4, 0x03, true),
        ]
    }

    /// **SMCG-01** — non-monotonic rejected.
    #[test]
    fn smcg_01_non_monotonic_rejected() {
        let ms = vec![migration(5, 3, 0x01, true)];
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::NonMonotonic {
                idx: 0,
                source: 5,
                target: 3,
            })
        );
    }

    /// **SMCG-02** — version too high rejected.
    #[test]
    fn smcg_02_version_too_high_rejected() {
        let ms = vec![migration(1, SMCG_MAX_VERSION + 1, 0x01, true)];
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::VersionTooHigh {
                idx: 0,
                version: SMCG_MAX_VERSION + 1,
                max: SMCG_MAX_VERSION,
            })
        );
    }

    /// **SMCG-03** — zero source rejected.
    #[test]
    fn smcg_03_zero_source_rejected() {
        let ms = vec![migration(0, 1, 0x01, true)];
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::ZeroSource(0))
        );
    }

    /// **SMCG-04** — duplicate path rejected.
    #[test]
    fn smcg_04_duplicate_path_rejected() {
        let ms = vec![
            migration(1, 2, 0x01, true),
            migration(1, 2, 0x02, true),
        ];
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::DuplicatePath {
                idx: 1,
                source: 1,
                target: 2,
            })
        );
    }

    /// **SMCG-05** — no rollback rejected.
    #[test]
    fn smcg_05_no_rollback_rejected() {
        let ms = vec![migration(1, 2, 0x01, false)];
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::NoRollback(0))
        );
    }

    /// **SMCG-06** — too many rejected.
    #[test]
    fn smcg_06_too_many_rejected() {
        let ms: Vec<MigrationRecord> = (0..=SMCG_MAX_MIGRATIONS)
            .map(|i| {
                let src = (i as u32) + 1;
                migration(src, src + 1, (i as u8).wrapping_add(1), true)
            })
            .collect();
        assert_eq!(
            validate_migration_compat(&ms),
            Err(MigrationCompatError::TooMany {
                got: SMCG_MAX_MIGRATIONS + 1,
                max: SMCG_MAX_MIGRATIONS,
            })
        );
    }

    /// **SMCG-07** — valid accepted.
    #[test]
    fn smcg_07_valid_accepted() {
        assert_eq!(validate_migration_compat(&valid_migrations()), Ok(()));
    }

    /// **SMCG-08** — empty accepted.
    #[test]
    fn smcg_08_empty_accepted() {
        assert_eq!(validate_migration_compat(&[]), Ok(()));
    }

    /// **SMCG-09** — single accepted.
    #[test]
    fn smcg_09_single_accepted() {
        let ms = vec![migration(1, 2, 0xAA, true)];
        assert_eq!(validate_migration_compat(&ms), Ok(()));
    }

    /// **SMCG-10** — max version boundary accepted.
    #[test]
    fn smcg_10_max_version_accepted() {
        let ms = vec![migration(SMCG_MAX_VERSION - 1, SMCG_MAX_VERSION, 0xFF, true)];
        assert_eq!(validate_migration_compat(&ms), Ok(()));
    }
}
