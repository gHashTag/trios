//! # CR-CHAT-03 — Transcript hash chain integrity guard (Wave-44 Lane A)
//!
//! RFC 9420 §8.3 — MLS transcript hash chain verification.
//!
//! The MLS key schedule chains transcript hashes across epochs: each epoch's
//! confirmed transcript hash depends on the previous epoch's hash. An
//! attacker who can modify or replay a transcript hash can:
//!
//! * **Break key derivation** — cause two members to derive different
//!   epoch secrets from the same commits.
//! * **Roll back the transcript** — reuse an old hash to force key reuse.
//! * **Fork the group** — two subgroups with different transcript chains
//!   cannot decrypt each other's messages.
//!
//! trios-chat enforces **7 rules**:
//!
//! 1. Confirmed hash is non-empty.
//! 2. Confirmed hash length is canonical (32 bytes).
//! 3. Interim hash is non-empty.
//! 4. Interim hash length is canonical (32 bytes).
//! 5. Epoch numbers are strictly monotonic.
//! 6. No confirmed hash is reused across epochs.
//! 7. Confirmed hash differs from interim hash within the same epoch.
//!
//! Tests **THASH-01..10**. Error enum [`TranscriptHashError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · TRANSCRIPT-HASH`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical hash length (SHA-256 output).
pub const THASH_HASH_LEN: usize = 32;

/// One epoch's transcript hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptHashEntry {
    /// Epoch number.
    pub epoch: u64,
    /// Confirmed transcript hash.
    pub confirmed: Vec<u8>,
    /// Interim transcript hash.
    pub interim: Vec<u8>,
}

/// All ways a transcript hash chain can be invalid.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TranscriptHashError {
    /// Confirmed hash is empty.
    EmptyConfirmedHash,
    /// Confirmed hash length is not 32 bytes.
    NonCanonicalConfirmedLength,
    /// Interim hash is empty.
    EmptyInterimHash,
    /// Interim hash length is not 32 bytes.
    NonCanonicalInterimLength,
    /// Epoch is not strictly increasing.
    EpochNotMonotonic,
    /// Confirmed hash reused across epochs.
    ConfirmedHashReuse,
    /// Confirmed hash equals interim hash (must differ).
    ConfirmedEqualsInterim,
}

/// `[VERIFIED]` Validate a transcript hash chain. Returns `Ok(())` if
/// all rules pass.
pub fn validate_transcript_hash_chain(
    entries: &[TranscriptHashEntry],
) -> Result<(), TranscriptHashError> {
    let mut seen_confirmed = BTreeSet::new();
    let mut prev_epoch = 0u64;
    let mut first = true;

    for entry in entries {
        if entry.confirmed.is_empty() {
            return Err(TranscriptHashError::EmptyConfirmedHash);
        }
        if entry.confirmed.len() != THASH_HASH_LEN {
            return Err(TranscriptHashError::NonCanonicalConfirmedLength);
        }
        if entry.interim.is_empty() {
            return Err(TranscriptHashError::EmptyInterimHash);
        }
        if entry.interim.len() != THASH_HASH_LEN {
            return Err(TranscriptHashError::NonCanonicalInterimLength);
        }
        if !first && entry.epoch <= prev_epoch {
            return Err(TranscriptHashError::EpochNotMonotonic);
        }
        prev_epoch = entry.epoch;
        first = false;
        if !seen_confirmed.insert(entry.confirmed.clone()) {
            return Err(TranscriptHashError::ConfirmedHashReuse);
        }
        if entry.confirmed == entry.interim {
            return Err(TranscriptHashError::ConfirmedEqualsInterim);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(epoch: u64, cf_byte: u8, it_byte: u8) -> TranscriptHashEntry {
        TranscriptHashEntry {
            epoch,
            confirmed: vec![cf_byte; 32],
            interim: vec![it_byte; 32],
        }
    }

    fn good_chain() -> Vec<TranscriptHashEntry> {
        vec![
            entry(1, 0xAA, 0xBB),
            entry(2, 0xCC, 0xDD),
            entry(3, 0xEE, 0xFF),
        ]
    }

    /// **THASH-01** — empty confirmed hash rejected.
    #[test]
    fn thash_01_empty_confirmed_rejected() {
        let mut chain = good_chain();
        chain[0].confirmed = vec![];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::EmptyConfirmedHash)
        );
    }

    /// **THASH-02** — non-canonical confirmed length rejected.
    #[test]
    fn thash_02_non_canonical_confirmed_rejected() {
        let mut chain = good_chain();
        chain[0].confirmed = vec![0xAA; 16];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::NonCanonicalConfirmedLength)
        );
    }

    /// **THASH-03** — empty interim hash rejected.
    #[test]
    fn thash_03_empty_interim_rejected() {
        let mut chain = good_chain();
        chain[0].interim = vec![];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::EmptyInterimHash)
        );
    }

    /// **THASH-04** — non-canonical interim length rejected.
    #[test]
    fn thash_04_non_canonical_interim_rejected() {
        let mut chain = good_chain();
        chain[0].interim = vec![0xBB; 64];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::NonCanonicalInterimLength)
        );
    }

    /// **THASH-05** — non-monotonic epoch rejected.
    #[test]
    fn thash_05_epoch_not_monotonic_rejected() {
        let chain = vec![
            entry(2, 0xAA, 0xBB),
            entry(1, 0xCC, 0xDD),
        ];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::EpochNotMonotonic)
        );
    }

    /// **THASH-06** — confirmed hash reuse rejected.
    #[test]
    fn thash_06_confirmed_hash_reuse_rejected() {
        let chain = vec![
            entry(1, 0xAA, 0xBB),
            entry(2, 0xAA, 0xCC),
        ];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::ConfirmedHashReuse)
        );
    }

    /// **THASH-07** — confirmed equals interim rejected.
    #[test]
    fn thash_07_confirmed_equals_interim_rejected() {
        let chain = vec![
            TranscriptHashEntry {
                epoch: 1,
                confirmed: vec![0xAA; 32],
                interim: vec![0xAA; 32],
            },
        ];
        assert_eq!(
            validate_transcript_hash_chain(&chain),
            Err(TranscriptHashError::ConfirmedEqualsInterim)
        );
    }

    /// **THASH-08** — valid chain accepted.
    #[test]
    fn thash_08_valid_chain_accepted() {
        assert_eq!(validate_transcript_hash_chain(&good_chain()), Ok(()));
    }

    /// **THASH-09** — single entry accepted.
    #[test]
    fn thash_09_single_entry_accepted() {
        let chain = vec![entry(1, 0xAA, 0xBB)];
        assert_eq!(validate_transcript_hash_chain(&chain), Ok(()));
    }

    /// **THASH-10** — empty chain accepted (nothing to validate).
    #[test]
    fn thash_10_empty_chain_accepted() {
        assert_eq!(validate_transcript_hash_chain(&[]), Ok(()));
    }
}
