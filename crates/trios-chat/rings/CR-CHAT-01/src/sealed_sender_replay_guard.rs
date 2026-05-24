//! # CR-CHAT-01 — Sealed sender replay guard (Wave-82 Lane A)
//!
//! IDENTITY — sealed sender ephemeral keys must be unique per session, R-CHAT-3.
//!
//! Sealed sender envelopes use an ephemeral X25519 key to hide the
//! sender's identity. If the same ephemeral key is reused:
//!
//! * **Sender deanonymization** — two envelopes with the same ephemeral
//!   key can be linked to the same sender.
//! * **Key derivation collision** — reused ephemeral key with different
//!   recipients produces predictable DH outputs.
//! * **Replay detection** — network observer identifies replays by
//!   matching ephemeral public keys.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each ephemeral public key is unique.
//! 2. Key length == `SSRG_KEY_LEN`.
//! 3. Key is not all-zeros.
//! 4. Total keys <= `SSRG_MAX_KEYS`.
//! 5. Key is not the identity key (ephemeral must differ from long-term).
//! 6. Destination hash is non-empty.
//!
//! Tests **SSRG-01..10**. Error enum [`SealedReplayError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SEALED-SENDER-REPLAY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Ephemeral key length.
pub const SSRG_KEY_LEN: usize = 32;

/// Maximum tracked keys.
pub const SSRG_MAX_KEYS: usize = 4096;

/// A sealed sender envelope.
#[derive(Debug, Clone)]
pub struct SealedEnvelope {
    /// Ephemeral public key.
    pub ephemeral_key: Vec<u8>,
    /// Destination hash (16 bytes).
    pub dest_hash: Vec<u8>,
}

/// All ways sealed sender replay validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealedReplayError {
    /// Duplicate ephemeral key.
    DuplicateKey,
    /// Key length wrong.
    KeyLengthWrong,
    /// Zero key.
    ZeroKey,
    /// Too many keys.
    TooManyKeys,
    /// Ephemeral equals identity key.
    EphemeralEqualsIdentity,
    /// Empty destination hash.
    EmptyDestHash,
}

/// `[VERIFIED]` Validate sealed sender envelopes for ephemeral key uniqueness.
pub fn validate_sealed_sender_replay(
    envelopes: &[SealedEnvelope],
    identity_key: &[u8],
) -> Result<(), SealedReplayError> {
    if envelopes.len() > SSRG_MAX_KEYS {
        return Err(SealedReplayError::TooManyKeys);
    }
    let mut seen = BTreeSet::new();
    for env in envelopes {
        if env.dest_hash.is_empty() {
            return Err(SealedReplayError::EmptyDestHash);
        }
        if env.ephemeral_key.len() != SSRG_KEY_LEN {
            return Err(SealedReplayError::KeyLengthWrong);
        }
        if env.ephemeral_key.iter().all(|&b| b == 0) {
            return Err(SealedReplayError::ZeroKey);
        }
        if env.ephemeral_key == identity_key {
            return Err(SealedReplayError::EphemeralEqualsIdentity);
        }
        if !seen.insert(env.ephemeral_key.clone()) {
            return Err(SealedReplayError::DuplicateKey);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; SSRG_KEY_LEN]
    }

    fn identity() -> Vec<u8> {
        key(0xFF)
    }

    fn envelope(eph_byte: u8) -> SealedEnvelope {
        SealedEnvelope {
            ephemeral_key: key(eph_byte),
            dest_hash: vec![0x01; 16],
        }
    }

    fn valid_envelopes() -> Vec<SealedEnvelope> {
        vec![envelope(0x01), envelope(0x02), envelope(0x03)]
    }

    /// **SSRG-01** — duplicate key rejected.
    #[test]
    fn ssrg_01_duplicate_rejected() {
        let envs = vec![envelope(0x01), envelope(0x01)];
        assert_eq!(
            validate_sealed_sender_replay(&envs, &identity()),
            Err(SealedReplayError::DuplicateKey)
        );
    }

    /// **SSRG-02** — key length wrong rejected.
    #[test]
    fn ssrg_02_key_len_rejected() {
        let env = SealedEnvelope {
            ephemeral_key: vec![0x01; 16],
            dest_hash: vec![0x01; 16],
        };
        assert_eq!(
            validate_sealed_sender_replay(&[env], &identity()),
            Err(SealedReplayError::KeyLengthWrong)
        );
    }

    /// **SSRG-03** — zero key rejected.
    #[test]
    fn ssrg_03_zero_key_rejected() {
        let env = SealedEnvelope {
            ephemeral_key: vec![0u8; SSRG_KEY_LEN],
            dest_hash: vec![0x01; 16],
        };
        assert_eq!(
            validate_sealed_sender_replay(&[env], &identity()),
            Err(SealedReplayError::ZeroKey)
        );
    }

    /// **SSRG-04** — too many keys rejected.
    #[test]
    fn ssrg_04_too_many_rejected() {
        let envs: Vec<SealedEnvelope> = (0..=SSRG_MAX_KEYS)
            .map(|i| SealedEnvelope {
                ephemeral_key: {
                    let mut k = vec![0u8; SSRG_KEY_LEN];
                    k[0] = (i % 256) as u8;
                    k[1] = ((i >> 8) % 256) as u8;
                    k
                },
                dest_hash: vec![0x01; 16],
            })
            .collect();
        assert_eq!(
            validate_sealed_sender_replay(&envs, &identity()),
            Err(SealedReplayError::TooManyKeys)
        );
    }

    /// **SSRG-05** — ephemeral equals identity rejected.
    #[test]
    fn ssrg_05_equals_identity_rejected() {
        let env = SealedEnvelope {
            ephemeral_key: identity(),
            dest_hash: vec![0x01; 16],
        };
        assert_eq!(
            validate_sealed_sender_replay(&[env], &identity()),
            Err(SealedReplayError::EphemeralEqualsIdentity)
        );
    }

    /// **SSRG-06** — empty dest hash rejected.
    #[test]
    fn ssrg_06_empty_dest_rejected() {
        let env = SealedEnvelope {
            ephemeral_key: key(0x01),
            dest_hash: vec![],
        };
        assert_eq!(
            validate_sealed_sender_replay(&[env], &identity()),
            Err(SealedReplayError::EmptyDestHash)
        );
    }

    /// **SSRG-07** — valid envelopes accepted.
    #[test]
    fn ssrg_07_valid_accepted() {
        assert_eq!(validate_sealed_sender_replay(&valid_envelopes(), &identity()), Ok(()));
    }

    /// **SSRG-08** — empty batch accepted.
    #[test]
    fn ssrg_08_empty_accepted() {
        assert_eq!(validate_sealed_sender_replay(&[], &identity()), Ok(()));
    }

    /// **SSRG-09** — single envelope accepted.
    #[test]
    fn ssrg_09_single_accepted() {
        assert_eq!(validate_sealed_sender_replay(&[envelope(0x42)], &identity()), Ok(()));
    }

    /// **SSRG-10** — different identity key accepted.
    #[test]
    fn ssrg_10_diff_identity_accepted() {
        assert_eq!(
            validate_sealed_sender_replay(&valid_envelopes(), &key(0xFE)),
            Ok(())
        );
    }
}
