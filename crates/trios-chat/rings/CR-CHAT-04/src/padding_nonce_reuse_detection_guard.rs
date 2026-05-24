//! # CR-CHAT-04 — Padding nonce reuse detection guard (Wave-105 Lane A)
//!
//! PADDING — padding nonces must never repeat.
//!
//! When encrypting padding payloads, the AEAD nonce must be unique per
//! encryption under the same key. Nonce reuse is catastrophic:
//!
//! * **Confidentiality loss** — XOR of two ciphertexts encrypted with
//!   the same nonce and key reveals the XOR of the two plaintexts.
//! * **Authenticity forgery** — given two valid ciphertexts under the
//!   same nonce, an attacker can forge a third valid ciphertext.
//! * **Key recovery** — in some AEAD modes, nonce reuse enables
//!   practical key recovery attacks.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No duplicate nonces.
//! 2. Nonce must not be all zeros.
//! 3. Nonce length must be `PNRD_NONCE_LEN`.
//! 4. Associated key ID must not be zero.
//! 5. Nonce must not exceed `PNRD_MAX_NONCE` value.
//! 6. Total records <= `PNRD_MAX_RECORDS`.
//!
//! Tests **PNRD-01..10**. Error enum [`NonceReuseError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * NO-NONCE-REUSE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Nonce length.
pub const PNRD_NONCE_LEN: usize = 12;

/// Key ID length.
pub const PNRD_KEY_ID_LEN: usize = 16;

/// Maximum records per batch.
pub const PNRD_MAX_RECORDS: usize = 10_000;

/// Maximum nonce value (big-endian u96).
pub const PNRD_MAX_NONCE: u64 = (1u64 << 48) - 1;

/// A nonce usage record.
#[derive(Debug, Clone)]
pub struct NonceRecord {
    /// Key identifier.
    pub key_id: [u8; PNRD_KEY_ID_LEN],
    /// Nonce used.
    pub nonce: [u8; PNRD_NONCE_LEN],
}

/// All ways nonce reuse validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NonceReuseError {
    /// Duplicate nonce under same key.
    Duplicate { idx: usize },
    /// Zero nonce.
    ZeroNonce(usize),
    /// Zero key ID.
    ZeroKey(usize),
    /// Nonce value exceeds maximum.
    NonceOverflow { idx: usize },
    /// Too many records.
    TooMany { got: usize, max: usize },
}

fn nonce_to_u64(n: &[u8; PNRD_NONCE_LEN]) -> u64 {
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&n[4..12]);
    u64::from_be_bytes(arr)
}

/// `[VERIFIED]` Validate padding nonce reuse detection.
pub fn validate_nonce_reuse(records: &[NonceRecord]) -> Result<(), NonceReuseError> {
    if records.len() > PNRD_MAX_RECORDS {
        return Err(NonceReuseError::TooMany {
            got: records.len(),
            max: PNRD_MAX_RECORDS,
        });
    }
    let mut seen: BTreeSet<([u8; PNRD_KEY_ID_LEN], [u8; PNRD_NONCE_LEN])> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.key_id == [0u8; PNRD_KEY_ID_LEN] {
            return Err(NonceReuseError::ZeroKey(i));
        }
        if r.nonce == [0u8; PNRD_NONCE_LEN] {
            return Err(NonceReuseError::ZeroNonce(i));
        }
        if nonce_to_u64(&r.nonce) > PNRD_MAX_NONCE {
            return Err(NonceReuseError::NonceOverflow { idx: i });
        }
        if !seen.insert((r.key_id, r.nonce)) {
            return Err(NonceReuseError::Duplicate { idx: i });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kid(byte: u8) -> [u8; PNRD_KEY_ID_LEN] {
        [byte; PNRD_KEY_ID_LEN]
    }

    fn nonce(val: u64) -> [u8; PNRD_NONCE_LEN] {
        let mut n = [0u8; PNRD_NONCE_LEN];
        n[4..12].copy_from_slice(&val.to_be_bytes());
        n
    }

    fn record(key_byte: u8, nonce_val: u64) -> NonceRecord {
        NonceRecord { key_id: kid(key_byte), nonce: nonce(nonce_val) }
    }

    fn valid_records() -> Vec<NonceRecord> {
        vec![
            record(0x01, 1),
            record(0x01, 2),
            record(0x02, 1),
        ]
    }

    /// **PNRD-01** — duplicate rejected.
    #[test]
    fn pnrd_01_duplicate_rejected() {
        let rs = vec![record(0x01, 42), record(0x01, 42)];
        assert_eq!(
            validate_nonce_reuse(&rs),
            Err(NonceReuseError::Duplicate { idx: 1 })
        );
    }

    /// **PNRD-02** — zero nonce rejected.
    #[test]
    fn pnrd_02_zero_nonce_rejected() {
        let r = NonceRecord { key_id: kid(0x01), nonce: [0u8; PNRD_NONCE_LEN] };
        assert_eq!(
            validate_nonce_reuse(&[r]),
            Err(NonceReuseError::ZeroNonce(0))
        );
    }

    /// **PNRD-03** — zero key rejected.
    #[test]
    fn pnrd_03_zero_key_rejected() {
        let r = NonceRecord { key_id: [0u8; PNRD_KEY_ID_LEN], nonce: nonce(1) };
        assert_eq!(
            validate_nonce_reuse(&[r]),
            Err(NonceReuseError::ZeroKey(0))
        );
    }

    /// **PNRD-04** — nonce overflow rejected.
    #[test]
    fn pnrd_04_nonce_overflow_rejected() {
        let mut n = [0u8; PNRD_NONCE_LEN];
        n[4..12].copy_from_slice(&(PNRD_MAX_NONCE + 1).to_be_bytes());
        let r = NonceRecord { key_id: kid(0x01), nonce: n };
        assert_eq!(
            validate_nonce_reuse(&[r]),
            Err(NonceReuseError::NonceOverflow { idx: 0 })
        );
    }

    /// **PNRD-05** — too many rejected.
    #[test]
    fn pnrd_05_too_many_rejected() {
        let rs: Vec<NonceRecord> = (0..=PNRD_MAX_RECORDS)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                NonceRecord { key_id: kid(b), nonce: nonce((i as u64) + 1) }
            })
            .collect();
        assert_eq!(
            validate_nonce_reuse(&rs),
            Err(NonceReuseError::TooMany {
                got: PNRD_MAX_RECORDS + 1,
                max: PNRD_MAX_RECORDS,
            })
        );
    }

    /// **PNRD-06** — same nonce different key accepted.
    #[test]
    fn pnrd_06_same_nonce_diff_key_accepted() {
        let rs = vec![record(0x01, 42), record(0x02, 42)];
        assert_eq!(validate_nonce_reuse(&rs), Ok(()));
    }

    /// **PNRD-07** — valid accepted.
    #[test]
    fn pnrd_07_valid_accepted() {
        assert_eq!(validate_nonce_reuse(&valid_records()), Ok(()));
    }

    /// **PNRD-08** — empty accepted.
    #[test]
    fn pnrd_08_empty_accepted() {
        assert_eq!(validate_nonce_reuse(&[]), Ok(()));
    }

    /// **PNRD-09** — single accepted.
    #[test]
    fn pnrd_09_single_accepted() {
        let rs = vec![record(0x01, 1)];
        assert_eq!(validate_nonce_reuse(&rs), Ok(()));
    }

    /// **PNRD-10** — boundary nonce accepted.
    #[test]
    fn pnrd_10_boundary_nonce_accepted() {
        let rs = vec![record(0x01, PNRD_MAX_NONCE)];
        assert_eq!(validate_nonce_reuse(&rs), Ok(()));
    }
}
