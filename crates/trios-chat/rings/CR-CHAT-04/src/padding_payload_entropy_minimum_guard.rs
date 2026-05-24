//! # CR-CHAT-04 — Padding payload entropy minimum guard (Wave-116 Lane B)
//!
//! PADDING — padded payloads must have minimum entropy.
//!
//! When a payload is padded, the padding bytes must have sufficient
//! entropy. Low-entropy padding is distinguishable from real
//! ciphertext:
//!
//! * **Statistical test** — chi-squared test on byte frequencies
//!   distinguishes all-zero padding from high-entropy ciphertext.
//! * **Pattern detection** — repeating patterns in padding (e.g.,
//!   0x00 or 0xFF runs) create a fingerprint for cover traffic.
//! * **Compression oracle** — compressibility of padding differs
//!   from ciphertext, enabling a compression side-channel.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Entropy >= `PPEM_MIN_ENTROPY` bits per byte.
//! 2. Payload length must be >= `PPEM_MIN_LEN`.
//! 3. Payload length must be <= `PPEM_MAX_LEN`.
//! 4. No zero-length payloads.
//! 5. No duplicate payload hashes.
//! 6. Total payloads <= `PPEM_MAX_PAYLOADS`.
//!
//! Tests **PPEM-01..10**. Error enum [`PayloadEntropyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * PAYLOAD-ENTROPY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Minimum entropy in bits per byte.
pub const PPEM_MIN_ENTROPY: f64 = 3.0;

/// Minimum payload length.
pub const PPEM_MIN_LEN: usize = 32;

/// Maximum payload length.
pub const PPEM_MAX_LEN: usize = 16384;

/// Maximum payloads per batch.
pub const PPEM_MAX_PAYLOADS: usize = 1024;

/// Hash length for dedup.
pub const PPEM_HASH_LEN: usize = 32;

/// A padded payload record.
#[derive(Debug, Clone)]
pub struct PayloadRecord {
    /// Payload data.
    pub payload: Vec<u8>,
    /// Hash of the payload for dedup.
    pub payload_hash: [u8; PPEM_HASH_LEN],
}

/// All ways payload entropy validation can fail.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum PayloadEntropyError {
    /// Entropy below minimum.
    BelowMin { idx: usize, entropy: f64, min: f64 },
    /// Payload too short.
    TooShort { idx: usize, len: usize, min: usize },
    /// Payload too long.
    TooLong { idx: usize, len: usize, max: usize },
    /// Zero length.
    ZeroLength(usize),
    /// Duplicate hash.
    DuplicateHash(usize),
    /// Too many payloads.
    TooMany { got: usize, max: usize },
}

fn compute_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut freq = [0usize; 256];
    for &b in data {
        freq[b as usize] += 1;
    }
    let len = data.len() as f64;
    let mut entropy = 0.0;
    for &f in &freq {
        if f > 0 {
            let p = f as f64 / len;
            entropy -= p * p.log2();
        }
    }
    entropy
}

/// `[VERIFIED]` Validate padding payload entropy minimum.
pub fn validate_payload_entropy(
    records: &[PayloadRecord],
) -> Result<(), PayloadEntropyError> {
    if records.len() > PPEM_MAX_PAYLOADS {
        return Err(PayloadEntropyError::TooMany {
            got: records.len(),
            max: PPEM_MAX_PAYLOADS,
        });
    }
    let mut seen: BTreeSet<[u8; PPEM_HASH_LEN]> = BTreeSet::new();
    for (i, r) in records.iter().enumerate() {
        if r.payload.is_empty() {
            return Err(PayloadEntropyError::ZeroLength(i));
        }
        if r.payload.len() < PPEM_MIN_LEN {
            return Err(PayloadEntropyError::TooShort {
                idx: i,
                len: r.payload.len(),
                min: PPEM_MIN_LEN,
            });
        }
        if r.payload.len() > PPEM_MAX_LEN {
            return Err(PayloadEntropyError::TooLong {
                idx: i,
                len: r.payload.len(),
                max: PPEM_MAX_LEN,
            });
        }
        let entropy = compute_entropy(&r.payload);
        if entropy < PPEM_MIN_ENTROPY {
            return Err(PayloadEntropyError::BelowMin {
                idx: i,
                entropy,
                min: PPEM_MIN_ENTROPY,
            });
        }
        if !seen.insert(r.payload_hash) {
            return Err(PayloadEntropyError::DuplicateHash(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; PPEM_HASH_LEN] {
        [byte; PPEM_HASH_LEN]
    }

    fn high_entropy_payload(len: usize, seed: u8) -> Vec<u8> {
        (0..len).map(|i| ((i as u8).wrapping_mul(79).wrapping_add(seed))).collect()
    }

    fn low_entropy_payload(len: usize) -> Vec<u8> {
        vec![0x00u8; len]
    }

    fn record(payload: Vec<u8>, hash_byte: u8) -> PayloadRecord {
        PayloadRecord { payload, payload_hash: hash(hash_byte) }
    }

    /// **PPEM-01** — below min entropy rejected.
    #[test]
    fn ppem_01_below_min_rejected() {
        let r = record(low_entropy_payload(256), 0x01);
        assert!(matches!(
            validate_payload_entropy(&[r]),
            Err(PayloadEntropyError::BelowMin { .. })
        ));
    }

    /// **PPEM-02** — too short rejected.
    #[test]
    fn ppem_02_too_short_rejected() {
        let r = record(high_entropy_payload(16, 0x42), 0x01);
        assert_eq!(
            validate_payload_entropy(&[r]),
            Err(PayloadEntropyError::TooShort {
                idx: 0,
                len: 16,
                min: PPEM_MIN_LEN,
            })
        );
    }

    /// **PPEM-03** — too long rejected.
    #[test]
    fn ppem_03_too_long_rejected() {
        let r = record(high_entropy_payload(PPEM_MAX_LEN + 1, 0x42), 0x01);
        assert_eq!(
            validate_payload_entropy(&[r]),
            Err(PayloadEntropyError::TooLong {
                idx: 0,
                len: PPEM_MAX_LEN + 1,
                max: PPEM_MAX_LEN,
            })
        );
    }

    /// **PPEM-04** — zero length rejected.
    #[test]
    fn ppem_04_zero_length_rejected() {
        let r = PayloadRecord { payload: vec![], payload_hash: hash(0x01) };
        assert_eq!(
            validate_payload_entropy(&[r]),
            Err(PayloadEntropyError::ZeroLength(0))
        );
    }

    /// **PPEM-05** — duplicate hash rejected.
    #[test]
    fn ppem_05_duplicate_rejected() {
        let rs = vec![
            record(high_entropy_payload(256, 0x42), 0x01),
            record(high_entropy_payload(256, 0x99), 0x01),
        ];
        assert_eq!(
            validate_payload_entropy(&rs),
            Err(PayloadEntropyError::DuplicateHash(1))
        );
    }

    /// **PPEM-06** — too many rejected.
    #[test]
    fn ppem_06_too_many_rejected() {
        let rs: Vec<PayloadRecord> = (0..=PPEM_MAX_PAYLOADS)
            .map(|i| {
                let b = (i as u8).wrapping_add(1);
                let mut h = [0u8; PPEM_HASH_LEN];
                h[0] = b;
                PayloadRecord { payload: high_entropy_payload(64, b), payload_hash: h }
            })
            .collect();
        assert_eq!(
            validate_payload_entropy(&rs),
            Err(PayloadEntropyError::TooMany {
                got: PPEM_MAX_PAYLOADS + 1,
                max: PPEM_MAX_PAYLOADS,
            })
        );
    }

    /// **PPEM-07** — valid accepted.
    #[test]
    fn ppem_07_valid_accepted() {
        let rs = vec![
            record(high_entropy_payload(256, 0x42), 0x01),
            record(high_entropy_payload(1024, 0x55), 0x02),
        ];
        assert_eq!(validate_payload_entropy(&rs), Ok(()));
    }

    /// **PPEM-08** — empty accepted.
    #[test]
    fn ppem_08_empty_accepted() {
        assert_eq!(validate_payload_entropy(&[]), Ok(()));
    }

    /// **PPEM-09** — min length boundary accepted.
    #[test]
    fn ppem_09_min_length_accepted() {
        let r = record(high_entropy_payload(PPEM_MIN_LEN, 0x42), 0x01);
        assert_eq!(validate_payload_entropy(&[r]), Ok(()));
    }

    /// **PPEM-10** — max length accepted.
    #[test]
    fn ppem_10_max_length_accepted() {
        let r = record(high_entropy_payload(PPEM_MAX_LEN, 0x42), 0x01);
        assert_eq!(validate_payload_entropy(&[r]), Ok(()));
    }
}
