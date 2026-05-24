//! # CR-CHAT-01 — Identity key rotation age freshness guard (Wave-145 Lane B)
//!
//! IDENTITY — identity keys must be rotated within a maximum age;
//! stale keys enable long-term compromise.
//!
//! Each identity key pair has a creation timestamp. If the key is
//! not rotated within the maximum allowed age:
//!
//! * **Long-term compromise** — the longer a key is in use, the
//!   more ciphertext is available for cryptanalysis.
//! * **Key wear** — extended key usage increases the attack surface
//!   for side-channel and known-plaintext attacks.
//! * **Rotation gap** — missing the rotation window means the old
//!   key remains trusted even if its security margin has eroded.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Key age <= `IKRF_MAX_AGE_MS`.
//! 2. Key ID must not be zero.
//! 3. No duplicate key IDs.
//! 4. Created timestamp must be > 0.
//! 5. Key must not be from the future.
//! 6. Batch size <= `IKRF_MAX_KEYS`.
//!
//! Tests **IKRF-01..10**. Error enum [`RotationFreshnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KEY-FRESH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum key age in milliseconds (90 days).
pub const IKRF_MAX_AGE_MS: u64 = 90 * 24 * 60 * 60 * 1000;

/// Maximum keys per batch.
pub const IKRF_MAX_KEYS: usize = 128;

/// Key ID length.
pub const IKRF_KEY_ID_LEN: usize = 32;

/// An identity key rotation record.
#[derive(Debug, Clone)]
pub struct KeyRotationRecord {
    /// Key identifier.
    pub key_id: [u8; IKRF_KEY_ID_LEN],
    /// Creation timestamp (ms since epoch).
    pub created_ms: u64,
}

/// All ways rotation freshness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RotationFreshnessError {
    /// Key too old.
    TooOld { idx: usize, age_ms: u64, max_ms: u64 },
    /// Zero key ID.
    ZeroKeyId(usize),
    /// Duplicate key ID.
    DuplicateKeyId { idx: usize },
    /// Zero created timestamp.
    ZeroCreated(usize),
    /// Future timestamp.
    FutureTs { idx: usize, created_ms: u64, now_ms: u64 },
    /// Too many keys.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate identity key rotation age freshness.
pub fn validate_rotation_freshness(
    keys: &[KeyRotationRecord],
    now_ms: u64,
) -> Result<(), RotationFreshnessError> {
    if keys.len() > IKRF_MAX_KEYS {
        return Err(RotationFreshnessError::TooMany {
            got: keys.len(),
            max: IKRF_MAX_KEYS,
        });
    }
    let mut seen: BTreeSet<[u8; IKRF_KEY_ID_LEN]> = BTreeSet::new();
    for (i, k) in keys.iter().enumerate() {
        if k.key_id == [0u8; IKRF_KEY_ID_LEN] {
            return Err(RotationFreshnessError::ZeroKeyId(i));
        }
        if !seen.insert(k.key_id) {
            return Err(RotationFreshnessError::DuplicateKeyId { idx: i });
        }
        if k.created_ms == 0 {
            return Err(RotationFreshnessError::ZeroCreated(i));
        }
        if k.created_ms > now_ms {
            return Err(RotationFreshnessError::FutureTs {
                idx: i,
                created_ms: k.created_ms,
                now_ms,
            });
        }
        let age = now_ms - k.created_ms;
        if age > IKRF_MAX_AGE_MS {
            return Err(RotationFreshnessError::TooOld {
                idx: i,
                age_ms: age,
                max_ms: IKRF_MAX_AGE_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kid(byte: u8) -> [u8; IKRF_KEY_ID_LEN] {
        [byte; IKRF_KEY_ID_LEN]
    }

    fn krec(id: u8, created: u64) -> KeyRotationRecord {
        KeyRotationRecord { key_id: kid(id), created_ms: created }
    }

    const NOW: u64 = 10_000_000_000;

    fn valid_keys() -> Vec<KeyRotationRecord> {
        vec![
            krec(0x01, NOW - 1000),
            krec(0x02, NOW - 5000),
        ]
    }

    /// **IKRF-01** — too old rejected.
    #[test]
    fn ikrf_01_too_old_rejected() {
        let k = krec(0x01, NOW - IKRF_MAX_AGE_MS - 1);
        assert_eq!(
            validate_rotation_freshness(&[k], NOW),
            Err(RotationFreshnessError::TooOld {
                idx: 0,
                age_ms: IKRF_MAX_AGE_MS + 1,
                max_ms: IKRF_MAX_AGE_MS,
            })
        );
    }

    /// **IKRF-02** — zero key ID rejected.
    #[test]
    fn ikrf_02_zero_key_rejected() {
        let k = KeyRotationRecord { key_id: [0u8; IKRF_KEY_ID_LEN], created_ms: NOW - 1000 };
        assert_eq!(
            validate_rotation_freshness(&[k], NOW),
            Err(RotationFreshnessError::ZeroKeyId(0))
        );
    }

    /// **IKRF-03** — duplicate key ID rejected.
    #[test]
    fn ikrf_03_duplicate_rejected() {
        let ks = vec![
            krec(0x01, NOW - 1000),
            krec(0x01, NOW - 2000),
        ];
        assert_eq!(
            validate_rotation_freshness(&ks, NOW),
            Err(RotationFreshnessError::DuplicateKeyId { idx: 1 })
        );
    }

    /// **IKRF-04** — zero created rejected.
    #[test]
    fn ikrf_04_zero_created_rejected() {
        let k = KeyRotationRecord { key_id: kid(0x01), created_ms: 0 };
        assert_eq!(
            validate_rotation_freshness(&[k], NOW),
            Err(RotationFreshnessError::ZeroCreated(0))
        );
    }

    /// **IKRF-05** — future timestamp rejected.
    #[test]
    fn ikrf_05_future_rejected() {
        let k = krec(0x01, NOW + 1000);
        assert_eq!(
            validate_rotation_freshness(&[k], NOW),
            Err(RotationFreshnessError::FutureTs {
                idx: 0,
                created_ms: NOW + 1000,
                now_ms: NOW,
            })
        );
    }

    /// **IKRF-06** — too many keys rejected.
    #[test]
    fn ikrf_06_too_many_rejected() {
        let ks: Vec<KeyRotationRecord> = (0..=IKRF_MAX_KEYS)
            .map(|i| {
                let mut id = [0u8; IKRF_KEY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                KeyRotationRecord { key_id: id, created_ms: NOW - 1000 }
            })
            .collect();
        assert_eq!(
            validate_rotation_freshness(&ks, NOW),
            Err(RotationFreshnessError::TooMany {
                got: IKRF_MAX_KEYS + 1,
                max: IKRF_MAX_KEYS,
            })
        );
    }

    /// **IKRF-07** — valid accepted.
    #[test]
    fn ikrf_07_valid_accepted() {
        assert_eq!(validate_rotation_freshness(&valid_keys(), NOW), Ok(()));
    }

    /// **IKRF-08** — empty accepted.
    #[test]
    fn ikrf_08_empty_accepted() {
        assert_eq!(validate_rotation_freshness(&[], NOW), Ok(()));
    }

    /// **IKRF-09** — boundary age accepted.
    #[test]
    fn ikrf_09_boundary_age_accepted() {
        let k = krec(0x01, NOW - IKRF_MAX_AGE_MS);
        assert_eq!(validate_rotation_freshness(&[k], NOW), Ok(()));
    }

    /// **IKRF-10** — exact now accepted.
    #[test]
    fn ikrf_10_exact_now_accepted() {
        let k = krec(0x01, NOW);
        assert_eq!(validate_rotation_freshness(&[k], NOW), Ok(()));
    }
}
