//! # CR-CHAT-05 — Store encryption nonce uniqueness guard (Wave-132 Lane B)
//!
//! PERSISTENCE — encryption nonces must be unique across all stored
//! records; nonce reuse with the same key enables catastrophic key
//! recovery.
//!
//! AEAD encryption requires a unique nonce for each encryption under
//! the same key. If a nonce is reused:
//!
//! * **Key recovery** — nonce reuse with AES-GCM or ChaCha20-Poly1305
//!   allows an attacker to recover the authentication key.
//! * **Plaintext recovery** — with two ciphertexts encrypted under
//!   the same nonce and key, XOR of ciphertexts = XOR of plaintexts.
//! * **Authentication bypass** — nonce reuse allows forgery of valid
//!   authenticated ciphertexts.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate nonces.
//! 2. Nonce must not be zero.
//! 3. Key hash must not be zero.
//! 4. No duplicate record IDs.
//! 5. Key hash must match across batch (same key).
//! 6. Total records <= `SENU_MAX_RECORDS`.
//!
//! Tests **SENU-01..10**. Error enum [`NonceUniquenessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * NONCE-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum records per batch.
pub const SENU_MAX_RECORDS: usize = 4096;

/// Nonce length.
pub const SENU_NONCE_LEN: usize = 12;

/// Key hash length.
pub const SENU_KEY_HASH_LEN: usize = 32;

/// Record ID length.
pub const SENU_RECORD_ID_LEN: usize = 32;

/// A stored record with its encryption nonce.
#[derive(Debug, Clone)]
pub struct NonceRecord {
    /// Record identifier.
    pub record_id: [u8; SENU_RECORD_ID_LEN],
    /// Encryption nonce.
    pub nonce: [u8; SENU_NONCE_LEN],
    /// Hash of the encryption key used.
    pub key_hash: [u8; SENU_KEY_HASH_LEN],
}

/// All ways nonce uniqueness validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NonceUniquenessError {
    /// Duplicate nonce.
    DuplicateNonce {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Zero nonce.
    ZeroNonce(usize),
    /// Zero key hash.
    ZeroKeyHash(usize),
    /// Duplicate record ID.
    DuplicateRecordId {
        /// Index of the duplicate.
        idx: usize,
    },
    /// Key hash mismatch (different keys in batch).
    KeyHashMismatch {
        /// Index of the mismatched record.
        idx: usize,
        /// Expected key hash.
        expected: [u8; SENU_KEY_HASH_LEN],
        /// Found key hash.
        found: [u8; SENU_KEY_HASH_LEN],
    },
    /// Too many records.
    TooMany {
        /// Count received.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate store encryption nonce uniqueness.
pub fn validate_nonce_uniqueness(
    records: &[NonceRecord],
) -> Result<(), NonceUniquenessError> {
    if records.len() > SENU_MAX_RECORDS {
        return Err(NonceUniquenessError::TooMany {
            got: records.len(),
            max: SENU_MAX_RECORDS,
        });
    }
    let mut seen_nonces: BTreeSet<[u8; SENU_NONCE_LEN]> = BTreeSet::new();
    let mut seen_ids: BTreeSet<[u8; SENU_RECORD_ID_LEN]> = BTreeSet::new();
    let mut canonical_key: Option<[u8; SENU_KEY_HASH_LEN]> = None;
    for (i, r) in records.iter().enumerate() {
        if r.nonce == [0u8; SENU_NONCE_LEN] {
            return Err(NonceUniquenessError::ZeroNonce(i));
        }
        if r.key_hash == [0u8; SENU_KEY_HASH_LEN] {
            return Err(NonceUniquenessError::ZeroKeyHash(i));
        }
        if !seen_ids.insert(r.record_id) {
            return Err(NonceUniquenessError::DuplicateRecordId { idx: i });
        }
        match canonical_key {
            None => canonical_key = Some(r.key_hash),
            Some(expected) if expected != r.key_hash => {
                return Err(NonceUniquenessError::KeyHashMismatch {
                    idx: i,
                    expected,
                    found: r.key_hash,
                });
            }
            _ => {}
        }
        if !seen_nonces.insert(r.nonce) {
            return Err(NonceUniquenessError::DuplicateNonce { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(byte: u8) -> [u8; SENU_RECORD_ID_LEN] {
        [byte; SENU_RECORD_ID_LEN]
    }

    fn nonce(a: u8, b: u8) -> [u8; SENU_NONCE_LEN] {
        let mut n = [0u8; SENU_NONCE_LEN];
        n[0] = a;
        n[1] = b;
        n
    }

    fn khash(byte: u8) -> [u8; SENU_KEY_HASH_LEN] {
        [byte; SENU_KEY_HASH_LEN]
    }

    fn nrecord(id: u8, n: (u8, u8), key: u8) -> NonceRecord {
        NonceRecord { record_id: rid(id), nonce: nonce(n.0, n.1), key_hash: khash(key) }
    }

    fn valid_records() -> Vec<NonceRecord> {
        vec![
            nrecord(0x01, (0x01, 0x00), 0xAA),
            nrecord(0x02, (0x02, 0x00), 0xAA),
            nrecord(0x03, (0x03, 0x00), 0xAA),
        ]
    }

    /// **SENU-01** — duplicate nonce rejected.
    #[test]
    fn senu_01_duplicate_nonce_rejected() {
        let rs = vec![
            nrecord(0x01, (0x01, 0x00), 0xAA),
            nrecord(0x02, (0x01, 0x00), 0xAA),
        ];
        assert_eq!(
            validate_nonce_uniqueness(&rs),
            Err(NonceUniquenessError::DuplicateNonce { idx: 1 })
        );
    }

    /// **SENU-02** — zero nonce rejected.
    #[test]
    fn senu_02_zero_nonce_rejected() {
        let r = NonceRecord { record_id: rid(0x01), nonce: [0u8; SENU_NONCE_LEN], key_hash: khash(0xAA) };
        assert_eq!(
            validate_nonce_uniqueness(&[r]),
            Err(NonceUniquenessError::ZeroNonce(0))
        );
    }

    /// **SENU-03** — zero key hash rejected.
    #[test]
    fn senu_03_zero_key_rejected() {
        let r = NonceRecord { record_id: rid(0x01), nonce: nonce(0x01, 0x00), key_hash: [0u8; SENU_KEY_HASH_LEN] };
        assert_eq!(
            validate_nonce_uniqueness(&[r]),
            Err(NonceUniquenessError::ZeroKeyHash(0))
        );
    }

    /// **SENU-04** — duplicate record ID rejected.
    #[test]
    fn senu_04_duplicate_id_rejected() {
        let rs = vec![
            nrecord(0x01, (0x01, 0x00), 0xAA),
            nrecord(0x01, (0x02, 0x00), 0xAA),
        ];
        assert_eq!(
            validate_nonce_uniqueness(&rs),
            Err(NonceUniquenessError::DuplicateRecordId { idx: 1 })
        );
    }

    /// **SENU-05** — key hash mismatch rejected.
    #[test]
    fn senu_05_key_mismatch_rejected() {
        let rs = vec![
            nrecord(0x01, (0x01, 0x00), 0xAA),
            nrecord(0x02, (0x02, 0x00), 0xBB),
        ];
        assert_eq!(
            validate_nonce_uniqueness(&rs),
            Err(NonceUniquenessError::KeyHashMismatch {
                idx: 1,
                expected: khash(0xAA),
                found: khash(0xBB),
            })
        );
    }

    /// **SENU-06** — too many rejected.
    #[test]
    fn senu_06_too_many_rejected() {
        let rs: Vec<NonceRecord> = (0..=SENU_MAX_RECORDS)
            .map(|i| {
                let mut id = [0u8; SENU_RECORD_ID_LEN];
                let val = (i as u64) + 1;
                id[0..8].copy_from_slice(&val.to_be_bytes());
                let mut n = [0u8; SENU_NONCE_LEN];
                n[0..8].copy_from_slice(&val.to_be_bytes());
                NonceRecord { record_id: id, nonce: n, key_hash: khash(0xAA) }
            })
            .collect();
        assert_eq!(
            validate_nonce_uniqueness(&rs),
            Err(NonceUniquenessError::TooMany {
                got: SENU_MAX_RECORDS + 1,
                max: SENU_MAX_RECORDS,
            })
        );
    }

    /// **SENU-07** — valid accepted.
    #[test]
    fn senu_07_valid_accepted() {
        assert_eq!(validate_nonce_uniqueness(&valid_records()), Ok(()));
    }

    /// **SENU-08** — empty accepted.
    #[test]
    fn senu_08_empty_accepted() {
        assert_eq!(validate_nonce_uniqueness(&[]), Ok(()));
    }

    /// **SENU-09** — single accepted.
    #[test]
    fn senu_09_single_accepted() {
        assert_eq!(validate_nonce_uniqueness(&[nrecord(0x01, (0x01, 0x00), 0xAA)]), Ok(()));
    }

    /// **SENU-10** — large batch accepted.
    #[test]
    fn senu_10_large_batch_accepted() {
        let rs: Vec<NonceRecord> = (0..256u64)
            .map(|i| {
                let mut id = [0u8; SENU_RECORD_ID_LEN];
                id[0..8].copy_from_slice(&(i + 1).to_be_bytes());
                let mut n = [0u8; SENU_NONCE_LEN];
                n[0..8].copy_from_slice(&(i + 1).to_be_bytes());
                NonceRecord { record_id: id, nonce: n, key_hash: khash(0xAA) }
            })
            .collect();
        assert_eq!(validate_nonce_uniqueness(&rs), Ok(()));
    }
}
