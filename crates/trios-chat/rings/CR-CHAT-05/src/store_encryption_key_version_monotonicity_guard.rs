//! # CR-CHAT-05 — Store encryption key version monotonicity guard (Wave-130 Lane A)
//!
//! PERSISTENCE — encryption key versions must increase monotonically;
//! version regressions indicate rollback or key management errors.
//!
//! When the store rotates its encryption key, the version number must
//! always increase. A version regression:
//!
//! * **Key rollback** — reverting to an old key version means data
//!   encrypted with the newer key may not be decryptable.
//! * **Replay window** — a regression reopens the window for replay
//!   attacks with data encrypted under the older key.
//! * **Audit confusion** — non-monotonic versions make forensic
//!   analysis of key rotation events unreliable.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Versions must be strictly increasing.
//! 2. Version must start at `SEKV_GENESIS`.
//! 3. Key hash must not be zero.
//! 4. No duplicate versions.
//! 5. Version must be <= `SEKV_MAX_VERSION`.
//! 6. Total versions <= `SEKV_MAX_ENTRIES`.
//!
//! Tests **SEKV-01..10**. Error enum [`KeyVersionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * VERSION-MONOTONIC`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Genesis version.
pub const SEKV_GENESIS: u32 = 1;

/// Maximum version.
pub const SEKV_MAX_VERSION: u32 = 10000;

/// Maximum entries per batch.
pub const SEKV_MAX_ENTRIES: usize = 1024;

/// Key hash length.
pub const SEKV_HASH_LEN: usize = 32;

/// An encryption key version entry.
#[derive(Debug, Clone)]
pub struct KeyVersionEntry {
    /// Key version number.
    pub version: u32,
    /// Hash of the key material.
    pub key_hash: [u8; SEKV_HASH_LEN],
}

/// All ways key version validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyVersionError {
    /// Non-monotonic version.
    NonMonotonic {
        /// Index of the offending entry.
        idx: usize,
        /// Previous version.
        prev: u32,
        /// Current version.
        current: u32,
    },
    /// Not at genesis.
    NotGenesis {
        /// Index of the offending entry.
        idx: usize,
        /// Version found.
        version: u32,
        /// Expected genesis version.
        expected: u32,
    },
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Duplicate version.
    DuplicateVersion {
        /// Index of the duplicate.
        idx: usize,
        /// Duplicate version number.
        version: u32,
    },
    /// Version exceeds maximum.
    TooHigh {
        /// Index of the offending entry.
        idx: usize,
        /// Version found.
        version: u32,
        /// Maximum allowed version.
        max: u32,
    },
    /// Too many entries.
    TooMany {
        /// Count received.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store encryption key version monotonicity.
pub fn validate_key_version_monotonicity(
    entries: &[KeyVersionEntry],
) -> Result<(), KeyVersionError> {
    if entries.len() > SEKV_MAX_ENTRIES {
        return Err(KeyVersionError::TooMany {
            got: entries.len(),
            max: SEKV_MAX_ENTRIES,
        });
    }
    let mut seen: BTreeSet<u32> = BTreeSet::new();
    let mut prev_ver: u32 = 0;
    for (i, e) in entries.iter().enumerate() {
        if e.key_hash == [0u8; SEKV_HASH_LEN] {
            return Err(KeyVersionError::ZeroKeyHash(i));
        }
        if e.version > SEKV_MAX_VERSION {
            return Err(KeyVersionError::TooHigh {
                idx: i,
                version: e.version,
                max: SEKV_MAX_VERSION,
            });
        }
        if i == 0 && e.version != SEKV_GENESIS {
            return Err(KeyVersionError::NotGenesis {
                idx: 0,
                version: e.version,
                expected: SEKV_GENESIS,
            });
        }
        if !seen.insert(e.version) {
            return Err(KeyVersionError::DuplicateVersion {
                idx: i,
                version: e.version,
            });
        }
        if i > 0 && e.version <= prev_ver {
            return Err(KeyVersionError::NonMonotonic {
                idx: i,
                prev: prev_ver,
                current: e.version,
            });
        }
        prev_ver = e.version;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; SEKV_HASH_LEN] {
        [byte; SEKV_HASH_LEN]
    }

    fn entry(ver: u32, h: u8) -> KeyVersionEntry {
        KeyVersionEntry { version: ver, key_hash: hash(h) }
    }

    fn valid_chain() -> Vec<KeyVersionEntry> {
        vec![
            entry(1, 0xA1),
            entry(2, 0xA2),
            entry(3, 0xA3),
        ]
    }

    /// **SEKV-01** — non-monotonic rejected.
    #[test]
    fn sekv_01_non_monotonic_rejected() {
        let es = vec![
            entry(1, 0xA1),
            entry(5, 0xA2),
            entry(3, 0xA3),
        ];
        assert_eq!(
            validate_key_version_monotonicity(&es),
            Err(KeyVersionError::NonMonotonic { idx: 2, prev: 5, current: 3 })
        );
    }

    /// **SEKV-02** — not genesis rejected.
    #[test]
    fn sekv_02_not_genesis_rejected() {
        let es = vec![entry(5, 0xA1)];
        assert_eq!(
            validate_key_version_monotonicity(&es),
            Err(KeyVersionError::NotGenesis { idx: 0, version: 5, expected: SEKV_GENESIS })
        );
    }

    /// **SEKV-03** — zero key hash rejected.
    #[test]
    fn sekv_03_zero_hash_rejected() {
        let e = KeyVersionEntry { version: 1, key_hash: [0u8; SEKV_HASH_LEN] };
        assert_eq!(
            validate_key_version_monotonicity(&[e]),
            Err(KeyVersionError::ZeroKeyHash(0))
        );
    }

    /// **SEKV-04** — duplicate version rejected.
    #[test]
    fn sekv_04_duplicate_rejected() {
        let es = vec![
            entry(1, 0xA1),
            entry(1, 0xA2),
        ];
        assert_eq!(
            validate_key_version_monotonicity(&es),
            Err(KeyVersionError::DuplicateVersion { idx: 1, version: 1 })
        );
    }

    /// **SEKV-05** — too high rejected.
    #[test]
    fn sekv_05_too_high_rejected() {
        let es = vec![KeyVersionEntry { version: SEKV_MAX_VERSION + 1, key_hash: hash(0xAA) }];
        assert_eq!(
            validate_key_version_monotonicity(&es),
            Err(KeyVersionError::TooHigh { idx: 0, version: SEKV_MAX_VERSION + 1, max: SEKV_MAX_VERSION })
        );
    }

    /// **SEKV-06** — too many rejected.
    #[test]
    fn sekv_06_too_many_rejected() {
        let es: Vec<KeyVersionEntry> = (0..=SEKV_MAX_ENTRIES)
            .map(|i| {
                let mut h = [0u8; SEKV_HASH_LEN];
                h[0] = (i as u8).wrapping_add(1);
                KeyVersionEntry { version: (i as u32) + 1, key_hash: h }
            })
            .collect();
        assert_eq!(
            validate_key_version_monotonicity(&es),
            Err(KeyVersionError::TooMany {
                got: SEKV_MAX_ENTRIES + 1,
                max: SEKV_MAX_ENTRIES,
            })
        );
    }

    /// **SEKV-07** — valid accepted.
    #[test]
    fn sekv_07_valid_accepted() {
        assert_eq!(validate_key_version_monotonicity(&valid_chain()), Ok(()));
    }

    /// **SEKV-08** — empty accepted.
    #[test]
    fn sekv_08_empty_accepted() {
        assert_eq!(validate_key_version_monotonicity(&[]), Ok(()));
    }

    /// **SEKV-09** — single genesis accepted.
    #[test]
    fn sekv_09_single_genesis_accepted() {
        assert_eq!(validate_key_version_monotonicity(&[entry(1, 0xAA)]), Ok(()));
    }

    /// **SEKV-10** — max version accepted.
    #[test]
    fn sekv_10_max_version_accepted() {
        let es = vec![
            entry(1, 0xA1),
            entry(SEKV_MAX_VERSION, 0xA2),
        ];
        assert_eq!(validate_key_version_monotonicity(&es), Ok(()));
    }
}
