//! # CR-CHAT-01 — Sealed sender recipient hash uniqueness guard (Wave-142 Lane A)
//!
//! IDENTITY — sealed sender envelopes must have unique recipient
//! hashes within a batch; duplicates enable correlation attacks.
//!
//! In the sealed sender protocol, each envelope carries a `dest_hash`
//! (16-byte truncated hash of the recipient's public key). If the
//! same recipient hash appears multiple times in a batch:
//!
//! * **Correlation attack** — an observer linking multiple envelopes
//!   to the same recipient by matching dest_hash values.
//! * **Traffic analysis** — frequency analysis of dest_hash values
//!   reveals which recipients are most active.
//! * **Intersection attack** — over time, repeated dest_hash values
//!   narrow down the anonymity set for each recipient.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. All dest_hash values must be unique within a batch.
//! 2. Dest hash must not be zero.
//! 3. Envelope ID must not be zero.
//! 4. No duplicate envelope IDs.
//! 5. Sender epoch must be > 0.
//! 6. Batch size <= `SSRU_MAX_BATCH`.
//!
//! Tests **SSRU-01..10**. Error enum [`RecipientHashError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * HASH-UNIQUE`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum batch size.
pub const SSRU_MAX_BATCH: usize = 256;

/// Dest hash length.
pub const SSRU_DEST_HASH_LEN: usize = 16;

/// Envelope ID length.
pub const SSRU_ENVELOPE_ID_LEN: usize = 32;

/// A sealed sender recipient hash record.
#[derive(Debug, Clone)]
pub struct RecipientHashRecord {
    /// Envelope identifier.
    pub envelope_id: [u8; SSRU_ENVELOPE_ID_LEN],
    /// Recipient destination hash.
    pub dest_hash: [u8; SSRU_DEST_HASH_LEN],
    /// Sender epoch.
    pub sender_epoch: u64,
}

/// All ways recipient hash validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RecipientHashError {
    /// Duplicate dest_hash.
    DuplicateDestHash {
        /// Index.
        idx: usize,
    },
    /// Zero dest_hash.
    ZeroDestHash(usize),
    /// Zero envelope ID.
    ZeroEnvelopeId(usize),
    /// Duplicate envelope ID.
    DuplicateEnvelopeId {
        /// Index.
        idx: usize,
    },
    /// Zero sender epoch.
    ZeroEpoch(usize),
    /// Batch too large.
    TooLarge {
        /// Actual count.
        got: usize,
        /// Maximum allowed.
        max: usize,
    },
}

/// `[VERIFIED]` Validate sealed sender recipient hash uniqueness.
pub fn validate_recipient_hash_uniqueness(
    envelopes: &[RecipientHashRecord],
) -> Result<(), RecipientHashError> {
    if envelopes.len() > SSRU_MAX_BATCH {
        return Err(RecipientHashError::TooLarge {
            got: envelopes.len(),
            max: SSRU_MAX_BATCH,
        });
    }
    let mut seen_eids: BTreeSet<[u8; SSRU_ENVELOPE_ID_LEN]> = BTreeSet::new();
    let mut seen_dests: BTreeSet<[u8; SSRU_DEST_HASH_LEN]> = BTreeSet::new();
    for (i, e) in envelopes.iter().enumerate() {
        if e.envelope_id == [0u8; SSRU_ENVELOPE_ID_LEN] {
            return Err(RecipientHashError::ZeroEnvelopeId(i));
        }
        if !seen_eids.insert(e.envelope_id) {
            return Err(RecipientHashError::DuplicateEnvelopeId { idx: i });
        }
        if e.dest_hash == [0u8; SSRU_DEST_HASH_LEN] {
            return Err(RecipientHashError::ZeroDestHash(i));
        }
        if !seen_dests.insert(e.dest_hash) {
            return Err(RecipientHashError::DuplicateDestHash { idx: i });
        }
        if e.sender_epoch == 0 {
            return Err(RecipientHashError::ZeroEpoch(i));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(byte: u8) -> [u8; SSRU_ENVELOPE_ID_LEN] {
        [byte; SSRU_ENVELOPE_ID_LEN]
    }

    fn dh(byte: u8) -> [u8; SSRU_DEST_HASH_LEN] {
        [byte; SSRU_DEST_HASH_LEN]
    }

    fn env(eid_byte: u8, dh_byte: u8, epoch: u64) -> RecipientHashRecord {
        RecipientHashRecord { envelope_id: eid(eid_byte), dest_hash: dh(dh_byte), sender_epoch: epoch }
    }

    fn valid_batch() -> Vec<RecipientHashRecord> {
        vec![
            env(0x01, 0xA1, 1),
            env(0x02, 0xA2, 1),
            env(0x03, 0xA3, 2),
        ]
    }

    /// **SSRU-01** — duplicate dest_hash rejected.
    #[test]
    fn ssru_01_duplicate_dest_rejected() {
        let es = vec![
            env(0x01, 0xA1, 1),
            env(0x02, 0xA1, 1),
        ];
        assert_eq!(
            validate_recipient_hash_uniqueness(&es),
            Err(RecipientHashError::DuplicateDestHash { idx: 1 })
        );
    }

    /// **SSRU-02** — zero dest_hash rejected.
    #[test]
    fn ssru_02_zero_dest_rejected() {
        let e = RecipientHashRecord {
            envelope_id: eid(0x01),
            dest_hash: [0u8; SSRU_DEST_HASH_LEN],
            sender_epoch: 1,
        };
        assert_eq!(
            validate_recipient_hash_uniqueness(&[e]),
            Err(RecipientHashError::ZeroDestHash(0))
        );
    }

    /// **SSRU-03** — zero envelope ID rejected.
    #[test]
    fn ssru_03_zero_eid_rejected() {
        let e = RecipientHashRecord {
            envelope_id: [0u8; SSRU_ENVELOPE_ID_LEN],
            dest_hash: dh(0xA1),
            sender_epoch: 1,
        };
        assert_eq!(
            validate_recipient_hash_uniqueness(&[e]),
            Err(RecipientHashError::ZeroEnvelopeId(0))
        );
    }

    /// **SSRU-04** — duplicate envelope ID rejected.
    #[test]
    fn ssru_04_duplicate_eid_rejected() {
        let es = vec![
            env(0x01, 0xA1, 1),
            env(0x01, 0xA2, 2),
        ];
        assert_eq!(
            validate_recipient_hash_uniqueness(&es),
            Err(RecipientHashError::DuplicateEnvelopeId { idx: 1 })
        );
    }

    /// **SSRU-05** — zero epoch rejected.
    #[test]
    fn ssru_05_zero_epoch_rejected() {
        let e = RecipientHashRecord { envelope_id: eid(0x01), dest_hash: dh(0xA1), sender_epoch: 0 };
        assert_eq!(
            validate_recipient_hash_uniqueness(&[e]),
            Err(RecipientHashError::ZeroEpoch(0))
        );
    }

    /// **SSRU-06** — batch too large rejected.
    #[test]
    fn ssru_06_too_large_rejected() {
        let es: Vec<RecipientHashRecord> = (0..=SSRU_MAX_BATCH)
            .map(|i| {
                let mut eid_val = [0u8; SSRU_ENVELOPE_ID_LEN];
                let mut dh_val = [0u8; SSRU_DEST_HASH_LEN];
                let v = (i as u64) + 1;
                eid_val[0..8].copy_from_slice(&v.to_be_bytes());
                dh_val[0..8].copy_from_slice(&v.to_be_bytes());
                RecipientHashRecord { envelope_id: eid_val, dest_hash: dh_val, sender_epoch: 1 }
            })
            .collect();
        assert_eq!(
            validate_recipient_hash_uniqueness(&es),
            Err(RecipientHashError::TooLarge {
                got: SSRU_MAX_BATCH + 1,
                max: SSRU_MAX_BATCH,
            })
        );
    }

    /// **SSRU-07** — valid accepted.
    #[test]
    fn ssru_07_valid_accepted() {
        assert_eq!(validate_recipient_hash_uniqueness(&valid_batch()), Ok(()));
    }

    /// **SSRU-08** — empty accepted.
    #[test]
    fn ssru_08_empty_accepted() {
        assert_eq!(validate_recipient_hash_uniqueness(&[]), Ok(()));
    }

    /// **SSRU-09** — single envelope accepted.
    #[test]
    fn ssru_09_single_accepted() {
        assert_eq!(validate_recipient_hash_uniqueness(&[env(0x01, 0xA1, 1)]), Ok(()));
    }

    /// **SSRU-10** — many unique dests accepted.
    #[test]
    fn ssru_10_many_unique_accepted() {
        let es: Vec<RecipientHashRecord> = (0..20u8)
            .map(|i| env(i + 1, i + 0xA0, 1))
            .collect();
        assert_eq!(validate_recipient_hash_uniqueness(&es), Ok(()));
    }
}
