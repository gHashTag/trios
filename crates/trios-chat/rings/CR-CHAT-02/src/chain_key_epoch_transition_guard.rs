//! # CR-CHAT-02 — Chain key epoch transition guard (Wave-91 Lane A)
//!
//! RATCHET — chain key must be re-derived on epoch transition, R-CHAT-2.
//!
//! When a DH ratchet step occurs (epoch transition), the root key
//! rotates and a new chain key is derived. If the transition is not
//! clean:
//!
//! * **Stale chain material** — the old chain key persists into the
//!   new epoch, allowing an attacker who compromised the old root to
//!   derive new epoch keys.
//! * **Cross-epoch derivation** — chain keys from different epochs
//!   are mixed, breaking the forward-secrecy boundary.
//! * **Missing transition** — epoch advances without re-deriving the
//!   chain key, leaving the old chain active under a new epoch number.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each epoch must have a unique chain key.
//! 2. Epoch numbers must be strictly increasing.
//! 3. Chain key must not be all zeros.
//! 4. Root key must differ between consecutive epochs.
//! 5. Maximum epochs <= `CKET_MAX_EPOCHS`.
//! 6. Chain key length must be `CKET_KEY_LEN`.
//!
//! Tests **CKET-01..10**. Error enum [`EpochTransitionError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * EPOCH-TRANSITION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum epochs to track.
pub const CKET_MAX_EPOCHS: usize = 256;

/// Chain key length.
pub const CKET_KEY_LEN: usize = 32;

/// An epoch transition record.
#[derive(Debug, Clone)]
pub struct EpochTransition {
    /// Epoch number.
    pub epoch: u64,
    /// Root key hash for this epoch.
    pub root_key_hash: [u8; CKET_KEY_LEN],
    /// Chain key for this epoch.
    pub chain_key: [u8; CKET_KEY_LEN],
}

/// All ways epoch transition validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochTransitionError {
    /// Duplicate chain key across epochs.
    DuplicateChainKey(u64),
    /// Epoch not increasing.
    EpochNotIncreasing(u64),
    /// Zero chain key.
    ZeroChainKey(u64),
    /// Same root key across consecutive epochs.
    SameRootKey { from_epoch: u64, to_epoch: u64 },
    /// Too many epochs.
    TooManyEpochs,
}

/// `[VERIFIED]` Validate chain key epoch transitions.
pub fn validate_epoch_transitions(
    transitions: &[EpochTransition],
) -> Result<(), EpochTransitionError> {
    if transitions.len() > CKET_MAX_EPOCHS {
        return Err(EpochTransitionError::TooManyEpochs);
    }
    let mut seen_chain_keys = BTreeSet::new();
    for (i, t) in transitions.iter().enumerate() {
        if t.chain_key == [0u8; CKET_KEY_LEN] {
            return Err(EpochTransitionError::ZeroChainKey(t.epoch));
        }
        if i > 0 && t.epoch <= transitions[i - 1].epoch {
            return Err(EpochTransitionError::EpochNotIncreasing(t.epoch));
        }
        if i > 0 && t.root_key_hash == transitions[i - 1].root_key_hash {
            return Err(EpochTransitionError::SameRootKey {
                from_epoch: transitions[i - 1].epoch,
                to_epoch: t.epoch,
            });
        }
        let ck_arr = t.chain_key;
        if !seen_chain_keys.insert(ck_arr) {
            return Err(EpochTransitionError::DuplicateChainKey(t.epoch));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; CKET_KEY_LEN] {
        [byte; CKET_KEY_LEN]
    }

    fn transition(epoch: u64, root: u8, chain: u8) -> EpochTransition {
        EpochTransition { epoch, root_key_hash: hash(root), chain_key: hash(chain) }
    }

    fn valid_transitions() -> Vec<EpochTransition> {
        vec![
            transition(1, 0xAA, 0x11),
            transition(2, 0xBB, 0x22),
            transition(3, 0xCC, 0x33),
        ]
    }

    /// **CKET-01** — duplicate chain key rejected.
    #[test]
    fn cket_01_duplicate_chain_key_rejected() {
        let ts = vec![transition(1, 0xAA, 0x11), transition(2, 0xBB, 0x11)];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::DuplicateChainKey(2))
        );
    }

    /// **CKET-02** — epoch not increasing rejected.
    #[test]
    fn cket_02_epoch_not_increasing_rejected() {
        let ts = vec![transition(2, 0xAA, 0x11), transition(1, 0xBB, 0x22)];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::EpochNotIncreasing(1))
        );
    }

    /// **CKET-03** — zero chain key rejected.
    #[test]
    fn cket_03_zero_chain_key_rejected() {
        let t = EpochTransition { epoch: 1, root_key_hash: hash(0xAA), chain_key: [0u8; CKET_KEY_LEN] };
        assert_eq!(
            validate_epoch_transitions(&[t]),
            Err(EpochTransitionError::ZeroChainKey(1))
        );
    }

    /// **CKET-04** — same root key rejected.
    #[test]
    fn cket_04_same_root_key_rejected() {
        let ts = vec![transition(1, 0xAA, 0x11), transition(2, 0xAA, 0x22)];
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::SameRootKey { from_epoch: 1, to_epoch: 2 })
        );
    }

    /// **CKET-05** — too many epochs rejected.
    #[test]
    fn cket_05_too_many_rejected() {
        let ts: Vec<EpochTransition> = (0..=CKET_MAX_EPOCHS as u64)
            .map(|i| {
                let mut r = [0u8; CKET_KEY_LEN];
                let mut c = [0u8; CKET_KEY_LEN];
                r[0] = (i % 200) as u8;
                r[1] = ((i / 200) % 200) as u8;
                c[0] = (i % 200) as u8;
                c[1] = ((i / 200 + 1) % 200) as u8;
                EpochTransition { epoch: i, root_key_hash: r, chain_key: c }
            })
            .collect();
        assert_eq!(
            validate_epoch_transitions(&ts),
            Err(EpochTransitionError::TooManyEpochs)
        );
    }

    /// **CKET-06** — valid transitions accepted.
    #[test]
    fn cket_06_valid_accepted() {
        assert_eq!(validate_epoch_transitions(&valid_transitions()), Ok(()));
    }

    /// **CKET-07** — empty accepted.
    #[test]
    fn cket_07_empty_accepted() {
        assert_eq!(validate_epoch_transitions(&[]), Ok(()));
    }

    /// **CKET-08** — single accepted.
    #[test]
    fn cket_08_single_accepted() {
        assert_eq!(validate_epoch_transitions(&[transition(1, 0xAA, 0xBB)]), Ok(()));
    }

    /// **CKET-09** — max epochs boundary accepted.
    #[test]
    fn cket_09_max_boundary_accepted() {
        let ts: Vec<EpochTransition> = (0..CKET_MAX_EPOCHS as u64)
            .map(|i| {
                let mut r = [0u8; CKET_KEY_LEN];
                let mut c = [0u8; CKET_KEY_LEN];
                let ib = i.to_le_bytes();
                r[..8].copy_from_slice(&ib);
                c[..8].copy_from_slice(&ib);
                c[8] = 0x01;
                EpochTransition { epoch: i, root_key_hash: r, chain_key: c }
            })
            .collect();
        assert_eq!(validate_epoch_transitions(&ts), Ok(()));
    }

    /// **CKET-10** — consecutive epochs with distinct roots accepted.
    #[test]
    fn cket_10_distinct_roots_accepted() {
        let ts = vec![
            transition(1, 0x01, 0x11),
            transition(2, 0x02, 0x12),
            transition(3, 0x03, 0x13),
            transition(4, 0x04, 0x14),
        ];
        assert_eq!(validate_epoch_transitions(&ts), Ok(()));
    }
}
