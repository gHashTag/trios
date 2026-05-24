//! # CR-CHAT-02 — Message key usage lifetime bound guard (Wave-143 Lane B)
//!
//! RATCHET — each message key must be used within a bounded lifetime;
//! exceeding the lifetime enables delayed-decryption attacks.
//!
//! In the Double Ratchet, each message key has an implicit lifetime
//! bounded by the next DH ratchet step. If a message key is used
//! beyond its intended lifetime:
//!
//! * **Delayed decryption** — an attacker who compromises a message
//!   key can decrypt all messages encrypted with it, even historical
//!   ones if the key hasn't been rotated.
//! * **Key wear** — the longer a key is active, the more ciphertext
//!   is available for cryptanalysis.
//! * **Forward secrecy gap** — old keys must be purged promptly;
//!   retaining them extends the compromise window.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Key age <= `MKUL_MAX_AGE_MS`.
//! 2. Key ID must not be zero.
//! 3. No duplicate key IDs.
//! 4. Created timestamp must be > 0.
//! 5. Usage count <= `MKUL_MAX_USAGES`.
//! 6. Batch size <= `MKUL_MAX_KEYS`.
//!
//! Tests **MKUL-01..10**. Error enum [`KeyLifetimeError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * KEY-EXPIRED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum key age in milliseconds (1 hour).
pub const MKUL_MAX_AGE_MS: u64 = 3_600_000;

/// Maximum usages per key.
pub const MKUL_MAX_USAGES: u64 = 100;

/// Maximum keys per batch.
pub const MKUL_MAX_KEYS: usize = 512;

/// Key ID length.
pub const MKUL_KEY_ID_LEN: usize = 32;

/// A message key lifetime record.
#[derive(Debug, Clone)]
pub struct KeyLifetimeRecord {
    /// Key identifier.
    pub key_id: [u8; MKUL_KEY_ID_LEN],
    /// Creation timestamp (ms since epoch).
    pub created_ms: u64,
    /// Number of times this key has been used.
    pub usage_count: u64,
}

/// All ways key lifetime validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyLifetimeError {
    /// Key too old.
    TooOld { idx: usize, age_ms: u64, max_ms: u64 },
    /// Zero key ID.
    ZeroKeyId(usize),
    /// Duplicate key ID.
    DuplicateKeyId { idx: usize },
    /// Zero created timestamp.
    ZeroCreated(usize),
    /// Too many usages.
    TooManyUsages { idx: usize, got: u64, max: u64 },
    /// Too many keys.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate message key usage lifetime bound.
pub fn validate_key_lifetime(
    keys: &[KeyLifetimeRecord],
    now_ms: u64,
) -> Result<(), KeyLifetimeError> {
    if keys.len() > MKUL_MAX_KEYS {
        return Err(KeyLifetimeError::TooMany {
            got: keys.len(),
            max: MKUL_MAX_KEYS,
        });
    }
    let mut seen: BTreeSet<[u8; MKUL_KEY_ID_LEN]> = BTreeSet::new();
    for (i, k) in keys.iter().enumerate() {
        if k.key_id == [0u8; MKUL_KEY_ID_LEN] {
            return Err(KeyLifetimeError::ZeroKeyId(i));
        }
        if !seen.insert(k.key_id) {
            return Err(KeyLifetimeError::DuplicateKeyId { idx: i });
        }
        if k.created_ms == 0 {
            return Err(KeyLifetimeError::ZeroCreated(i));
        }
        if k.usage_count > MKUL_MAX_USAGES {
            return Err(KeyLifetimeError::TooManyUsages {
                idx: i,
                got: k.usage_count,
                max: MKUL_MAX_USAGES,
            });
        }
        if k.created_ms > now_ms {
            return Err(KeyLifetimeError::ZeroCreated(i));
        }
        let age = now_ms - k.created_ms;
        if age > MKUL_MAX_AGE_MS {
            return Err(KeyLifetimeError::TooOld {
                idx: i,
                age_ms: age,
                max_ms: MKUL_MAX_AGE_MS,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kid(byte: u8) -> [u8; MKUL_KEY_ID_LEN] {
        [byte; MKUL_KEY_ID_LEN]
    }

    fn keyrec(id: u8, created: u64, usages: u64) -> KeyLifetimeRecord {
        KeyLifetimeRecord { key_id: kid(id), created_ms: created, usage_count: usages }
    }

    const NOW: u64 = 10_000_000_000;

    fn valid_keys() -> Vec<KeyLifetimeRecord> {
        vec![
            keyrec(0x01, NOW - 1000, 10),
            keyrec(0x02, NOW - 5000, 50),
        ]
    }

    /// **MKUL-01** — too old rejected.
    #[test]
    fn mkul_01_too_old_rejected() {
        let k = keyrec(0x01, NOW - MKUL_MAX_AGE_MS - 1, 5);
        assert_eq!(
            validate_key_lifetime(&[k], NOW),
            Err(KeyLifetimeError::TooOld {
                idx: 0,
                age_ms: MKUL_MAX_AGE_MS + 1,
                max_ms: MKUL_MAX_AGE_MS,
            })
        );
    }

    /// **MKUL-02** — zero key ID rejected.
    #[test]
    fn mkul_02_zero_key_rejected() {
        let k = KeyLifetimeRecord { key_id: [0u8; MKUL_KEY_ID_LEN], created_ms: NOW - 1000, usage_count: 5 };
        assert_eq!(
            validate_key_lifetime(&[k], NOW),
            Err(KeyLifetimeError::ZeroKeyId(0))
        );
    }

    /// **MKUL-03** — duplicate key ID rejected.
    #[test]
    fn mkul_03_duplicate_rejected() {
        let ks = vec![
            keyrec(0x01, NOW - 1000, 5),
            keyrec(0x01, NOW - 2000, 10),
        ];
        assert_eq!(
            validate_key_lifetime(&ks, NOW),
            Err(KeyLifetimeError::DuplicateKeyId { idx: 1 })
        );
    }

    /// **MKUL-04** — zero created rejected.
    #[test]
    fn mkul_04_zero_created_rejected() {
        let k = KeyLifetimeRecord { key_id: kid(0x01), created_ms: 0, usage_count: 5 };
        assert_eq!(
            validate_key_lifetime(&[k], NOW),
            Err(KeyLifetimeError::ZeroCreated(0))
        );
    }

    /// **MKUL-05** — too many usages rejected.
    #[test]
    fn mkul_05_too_many_usages_rejected() {
        let k = keyrec(0x01, NOW - 1000, MKUL_MAX_USAGES + 1);
        assert_eq!(
            validate_key_lifetime(&[k], NOW),
            Err(KeyLifetimeError::TooManyUsages {
                idx: 0,
                got: MKUL_MAX_USAGES + 1,
                max: MKUL_MAX_USAGES,
            })
        );
    }

    /// **MKUL-06** — too many keys rejected.
    #[test]
    fn mkul_06_too_many_rejected() {
        let ks: Vec<KeyLifetimeRecord> = (0..=MKUL_MAX_KEYS)
            .map(|i| {
                let mut id = [0u8; MKUL_KEY_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                KeyLifetimeRecord { key_id: id, created_ms: NOW - 1000, usage_count: 1 }
            })
            .collect();
        assert_eq!(
            validate_key_lifetime(&ks, NOW),
            Err(KeyLifetimeError::TooMany {
                got: MKUL_MAX_KEYS + 1,
                max: MKUL_MAX_KEYS,
            })
        );
    }

    /// **MKUL-07** — valid accepted.
    #[test]
    fn mkul_07_valid_accepted() {
        assert_eq!(validate_key_lifetime(&valid_keys(), NOW), Ok(()));
    }

    /// **MKUL-08** — empty accepted.
    #[test]
    fn mkul_08_empty_accepted() {
        assert_eq!(validate_key_lifetime(&[], NOW), Ok(()));
    }

    /// **MKUL-09** — boundary age accepted.
    #[test]
    fn mkul_09_boundary_age_accepted() {
        let k = keyrec(0x01, NOW - MKUL_MAX_AGE_MS, 1);
        assert_eq!(validate_key_lifetime(&[k], NOW), Ok(()));
    }

    /// **MKUL-10** — boundary usages accepted.
    #[test]
    fn mkul_10_boundary_usages_accepted() {
        let k = keyrec(0x01, NOW - 1000, MKUL_MAX_USAGES);
        assert_eq!(validate_key_lifetime(&[k], NOW), Ok(()));
    }
}
