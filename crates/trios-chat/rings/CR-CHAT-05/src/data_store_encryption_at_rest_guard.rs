//! # CR-CHAT-05 — Data store encryption at rest guard (Wave-81 Lane B)
//!
//! PERSISTENCE — stored data must be encrypted, R-CHAT-5.
//!
//! If the persistence store writes plaintext data to disk:
//!
//! * **Data breach** — any process with disk access reads all chat
//!   history, keys, and metadata.
//! * **Snapshot extraction** — cloud backups capture unencrypted
//!   data, leaking it outside the trust boundary.
//! * **Forensic recovery** — deleted but unencrypted data is
//!   recoverable from disk sectors.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Every stored record has a non-empty ciphertext.
//! 2. No record has a non-empty plaintext field.
//! 3. Ciphertext length >= `DSER_MIN_CT_LEN`.
//! 4. Ciphertext length <= `DSER_MAX_CT_LEN`.
//! 5. Record count <= `DSER_MAX_RECORDS`.
//! 6. No record key is zero/empty.
//!
//! Tests **DSER-01..10**. Error enum [`EncryptAtRestError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DATA-STORE-ENCRYPT`

#![forbid(unsafe_code)]

/// Minimum ciphertext length.
pub const DSER_MIN_CT_LEN: usize = 16;

/// Maximum ciphertext length.
pub const DSER_MAX_CT_LEN: usize = 1_048_576;

/// Maximum records in a batch.
pub const DSER_MAX_RECORDS: usize = 1024;

/// A stored record.
#[derive(Debug, Clone)]
pub struct StoredRecord {
    /// Record key (identifier).
    pub key: Vec<u8>,
    /// Ciphertext (must be non-empty).
    pub ciphertext: Vec<u8>,
    /// Plaintext (must be empty if encryption is enforced).
    pub plaintext: Vec<u8>,
}

/// All ways encryption-at-rest validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EncryptAtRestError {
    /// Empty ciphertext.
    EmptyCiphertext,
    /// Non-empty plaintext found.
    PlaintextFound(usize),
    /// Ciphertext too short.
    CiphertextTooShort,
    /// Ciphertext too long.
    CiphertextTooLong,
    /// Too many records.
    TooManyRecords,
    /// Empty key.
    EmptyKey,
}

/// `[VERIFIED]` Validate that all stored records are encrypted at rest.
pub fn validate_encrypt_at_rest(
    records: &[StoredRecord],
) -> Result<(), EncryptAtRestError> {
    if records.len() > DSER_MAX_RECORDS {
        return Err(EncryptAtRestError::TooManyRecords);
    }
    for record in records {
        if record.key.is_empty() {
            return Err(EncryptAtRestError::EmptyKey);
        }
        if record.ciphertext.is_empty() {
            return Err(EncryptAtRestError::EmptyCiphertext);
        }
        if record.ciphertext.len() < DSER_MIN_CT_LEN {
            return Err(EncryptAtRestError::CiphertextTooShort);
        }
        if record.ciphertext.len() > DSER_MAX_CT_LEN {
            return Err(EncryptAtRestError::CiphertextTooLong);
        }
        if !record.plaintext.is_empty() {
            return Err(EncryptAtRestError::PlaintextFound(record.plaintext.len()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encrypted_record() -> StoredRecord {
        StoredRecord {
            key: vec![0x01],
            ciphertext: vec![0xAB; 32],
            plaintext: vec![],
        }
    }

    fn valid_records() -> Vec<StoredRecord> {
        vec![
            StoredRecord { key: vec![1], ciphertext: vec![0xAA; 32], plaintext: vec![] },
            StoredRecord { key: vec![2], ciphertext: vec![0xBB; 64], plaintext: vec![] },
        ]
    }

    /// **DSER-01** — empty ciphertext rejected.
    #[test]
    fn dser_01_empty_ct_rejected() {
        let r = StoredRecord { key: vec![1], ciphertext: vec![], plaintext: vec![] };
        assert_eq!(
            validate_encrypt_at_rest(&[r]),
            Err(EncryptAtRestError::EmptyCiphertext)
        );
    }

    /// **DSER-02** — plaintext found rejected.
    #[test]
    fn dser_02_plaintext_rejected() {
        let r = StoredRecord {
            key: vec![1],
            ciphertext: vec![0xAA; 32],
            plaintext: vec![0xDE, 0xAD],
        };
        assert_eq!(
            validate_encrypt_at_rest(&[r]),
            Err(EncryptAtRestError::PlaintextFound(2))
        );
    }

    /// **DSER-03** — ciphertext too short rejected.
    #[test]
    fn dser_03_ct_short_rejected() {
        let r = StoredRecord { key: vec![1], ciphertext: vec![0xAA; 4], plaintext: vec![] };
        assert_eq!(
            validate_encrypt_at_rest(&[r]),
            Err(EncryptAtRestError::CiphertextTooShort)
        );
    }

    /// **DSER-04** — ciphertext too long rejected.
    #[test]
    fn dser_04_ct_long_rejected() {
        let r = StoredRecord {
            key: vec![1],
            ciphertext: vec![0xAA; DSER_MAX_CT_LEN + 1],
            plaintext: vec![],
        };
        assert_eq!(
            validate_encrypt_at_rest(&[r]),
            Err(EncryptAtRestError::CiphertextTooLong)
        );
    }

    /// **DSER-05** — too many records rejected.
    #[test]
    fn dser_05_too_many_rejected() {
        let records: Vec<StoredRecord> = (0..=DSER_MAX_RECORDS)
            .map(|i| StoredRecord {
                key: vec![i as u8],
                ciphertext: vec![0xAA; 32],
                plaintext: vec![],
            })
            .collect();
        assert_eq!(
            validate_encrypt_at_rest(&records),
            Err(EncryptAtRestError::TooManyRecords)
        );
    }

    /// **DSER-06** — empty key rejected.
    #[test]
    fn dser_06_empty_key_rejected() {
        let r = StoredRecord { key: vec![], ciphertext: vec![0xAA; 32], plaintext: vec![] };
        assert_eq!(
            validate_encrypt_at_rest(&[r]),
            Err(EncryptAtRestError::EmptyKey)
        );
    }

    /// **DSER-07** — valid records accepted.
    #[test]
    fn dser_07_valid_accepted() {
        assert_eq!(validate_encrypt_at_rest(&valid_records()), Ok(()));
    }

    /// **DSER-08** — single record accepted.
    #[test]
    fn dser_08_single_accepted() {
        assert_eq!(validate_encrypt_at_rest(&[encrypted_record()]), Ok(()));
    }

    /// **DSER-09** — empty batch accepted.
    #[test]
    fn dser_09_empty_accepted() {
        assert_eq!(validate_encrypt_at_rest(&[]), Ok(()));
    }

    /// **DSER-10** — min ciphertext length accepted.
    #[test]
    fn dser_10_min_ct_accepted() {
        let r = StoredRecord {
            key: vec![1],
            ciphertext: vec![0xAA; DSER_MIN_CT_LEN],
            plaintext: vec![],
        };
        assert_eq!(validate_encrypt_at_rest(&[r]), Ok(()));
    }
}
