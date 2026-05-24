//! # CR-CHAT-02 — Sender chain fork detection guard (Wave-69 Lane B)
//!
//! RATCHET — two messages at same epoch+sender with different chain_key = fork, R-CHAT-2.
//!
//! In a double-ratchet protocol, each `(epoch, sender_index)` pair must
//! map to exactly one chain key. If two messages claim the same
//! `(epoch, sender_index)` but carry different chain keys, one of them
//! is a fork attempt:
//!
//! * **Concurrent send** — two devices send from the same epoch without
//!   syncing, creating divergent chains.
//! * **Replay with modification** — attacker captures a message, modifies
//!   the chain key, and re-injects.
//! * **Key compromise** — compromised key holder generates messages that
//!   conflict with the legitimate holder's chain.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each `(epoch, sender_index)` maps to exactly one chain key.
//! 2. Epoch is non-zero.
//! 3. Sender index < `SCFD_MAX_SENDERS`.
//! 4. Chain key length == `SCFD_KEY_LEN`.
//! 5. Chain key is not all-zeros.
//! 6. Number of entries <= `SCFD_MAX_ENTRIES`.
//!
//! Tests **SCFD-01..10**. Error enum [`ChainForkError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SENDER-CHAIN-FORK`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Maximum sender index.
pub const SCFD_MAX_SENDERS: u32 = 256;

/// Chain key length.
pub const SCFD_KEY_LEN: usize = 32;

/// Maximum entries to track.
pub const SCFD_MAX_ENTRIES: usize = 1024;

/// All ways sender chain fork detection can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainForkError {
    /// Fork detected — same (epoch, sender) with different chain key.
    ForkDetected,
    /// Zero epoch.
    ZeroEpoch,
    /// Sender index out of bounds.
    SenderOutOfBounds,
    /// Chain key length wrong.
    KeyLengthWrong,
    /// Zero chain key.
    ZeroKey,
    /// Too many entries.
    TooManyEntries,
}

/// `[VERIFIED]` Detect sender chain forks by checking unique chain keys per (epoch, sender).
pub fn detect_chain_fork(
    entries: &[(u64, u32, &[u8])],
) -> Result<(), ChainForkError> {
    if entries.len() > SCFD_MAX_ENTRIES {
        return Err(ChainForkError::TooManyEntries);
    }
    let mut seen: BTreeMap<(u64, u32), Vec<u8>> = BTreeMap::new();
    for &(epoch, sender, key) in entries {
        if epoch == 0 {
            return Err(ChainForkError::ZeroEpoch);
        }
        if sender >= SCFD_MAX_SENDERS {
            return Err(ChainForkError::SenderOutOfBounds);
        }
        if key.len() != SCFD_KEY_LEN {
            return Err(ChainForkError::KeyLengthWrong);
        }
        if key.iter().all(|&b| b == 0) {
            return Err(ChainForkError::ZeroKey);
        }
        if let Some(existing) = seen.get(&(epoch, sender)) {
            if existing != key {
                return Err(ChainForkError::ForkDetected);
            }
        } else {
            seen.insert((epoch, sender), key.to_vec());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; SCFD_KEY_LEN]
    }

    fn valid_entries() -> Vec<(u64, u32, Vec<u8>)> {
        vec![
            (1, 0, key(0x01)),
            (1, 1, key(0x02)),
            (2, 0, key(0x03)),
        ]
    }

    /// **SCFD-01** — fork detected.
    #[test]
    fn scfd_01_fork_detected() {
        let k1 = key(0x01);
        let k2 = key(0x02);
        assert_eq!(
            detect_chain_fork(&[(1, 0, k1.as_slice()), (1, 0, k2.as_slice())]),
            Err(ChainForkError::ForkDetected)
        );
    }

    /// **SCFD-02** — zero epoch rejected.
    #[test]
    fn scfd_02_zero_epoch_rejected() {
        assert_eq!(
            detect_chain_fork(&[(0, 0, key(0x01).as_slice())]),
            Err(ChainForkError::ZeroEpoch)
        );
    }

    /// **SCFD-03** — sender out of bounds rejected.
    #[test]
    fn scfd_03_sender_oob_rejected() {
        assert_eq!(
            detect_chain_fork(&[(1, SCFD_MAX_SENDERS, key(0x01).as_slice())]),
            Err(ChainForkError::SenderOutOfBounds)
        );
    }

    /// **SCFD-04** — key length wrong rejected.
    #[test]
    fn scfd_04_key_len_rejected() {
        assert_eq!(
            detect_chain_fork(&[(1, 0, &[0x01u8; 16][..])]),
            Err(ChainForkError::KeyLengthWrong)
        );
    }

    /// **SCFD-05** — zero key rejected.
    #[test]
    fn scfd_05_zero_key_rejected() {
        assert_eq!(
            detect_chain_fork(&[(1, 0, &[0u8; 32][..])]),
            Err(ChainForkError::ZeroKey)
        );
    }

    /// **SCFD-06** — too many entries rejected.
    #[test]
    fn scfd_06_too_many_rejected() {
        let entries: Vec<(u64, u32, Vec<u8>)> = (0..=SCFD_MAX_ENTRIES)
            .map(|i| {
                let mut k = key(0x01);
                k[0] = (i % 256) as u8;
                ((i as u64 / SCFD_MAX_SENDERS as u64) + 1, (i as u32) % SCFD_MAX_SENDERS, k)
            })
            .collect();
        let refs: Vec<(u64, u32, &[u8])> = entries.iter()
            .map(|(e, s, k)| (*e, *s, k.as_slice()))
            .collect();
        assert_eq!(
            detect_chain_fork(&refs),
            Err(ChainForkError::TooManyEntries)
        );
    }

    /// **SCFD-07** — valid entries accepted.
    #[test]
    fn scfd_07_valid_accepted() {
        let entries = valid_entries();
        let refs: Vec<(u64, u32, &[u8])> = entries.iter()
            .map(|(e, s, k)| (*e, *s, k.as_slice()))
            .collect();
        assert_eq!(detect_chain_fork(&refs), Ok(()));
    }

    /// **SCFD-08** — same key same slot accepted (idempotent).
    #[test]
    fn scfd_08_same_key_accepted() {
        let k = key(0xAA);
        assert_eq!(
            detect_chain_fork(&[(1, 0, k.as_slice()), (1, 0, k.as_slice())]),
            Ok(())
        );
    }

    /// **SCFD-09** — different senders same epoch accepted.
    #[test]
    fn scfd_09_diff_senders_accepted() {
        assert_eq!(
            detect_chain_fork(&[(1, 0, key(0x01).as_slice()), (1, 1, key(0x02).as_slice())]),
            Ok(())
        );
    }

    /// **SCFD-10** — empty accepted.
    #[test]
    fn scfd_10_empty_accepted() {
        assert_eq!(detect_chain_fork(&[]), Ok(()));
    }
}
