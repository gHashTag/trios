//! # CR-CHAT-02 — Message key derivation binding guard (Wave-76 Lane B)
//!
//! RATCHET — message key must be bound to (epoch, chain_index), R-CHAT-2.
//!
//! Each message key in the double ratchet is derived from (epoch,
//! chain_index). If the binding is missing or incorrect:
//!
//! * **Key cross-use** — same message key encrypts two messages at
//!   different chain positions, enabling nonce reuse attacks.
//! * **Epoch confusion** — a message key from epoch N is used in
//!   epoch N+1, breaking forward secrecy.
//! * **Index collision** — two chain positions derive the same key,
//!   causing decryption ambiguity.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Each (epoch, chain_index) maps to exactly one message key.
//! 2. Epoch > 0.
//! 3. Chain index < `MKDB_MAX_CHAIN_INDEX`.
//! 4. Message key length == `MKDB_KEY_LEN`.
//! 5. Message key is not all-zeros.
//! 6. No duplicate (epoch, chain_index) pairs.
//!
//! Tests **MKDB-01..10**. Error enum [`MsgKeyBindError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * MSG-KEY-BINDING`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum chain index per epoch.
pub const MKDB_MAX_CHAIN_INDEX: u32 = 1024;

/// Message key length.
pub const MKDB_KEY_LEN: usize = 32;

/// All ways message key binding validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum MsgKeyBindError {
    /// Duplicate (epoch, chain_index) with different key.
    BindingCollision,
    /// Zero epoch.
    ZeroEpoch,
    /// Chain index out of bounds.
    ChainIndexOutOfBounds,
    /// Key length wrong.
    KeyLengthWrong,
    /// Zero key.
    ZeroKey,
    /// Duplicate key across different bindings.
    DuplicateKey,
}

/// A message key binding.
#[derive(Debug, Clone)]
pub struct MsgKeyBinding {
    /// Epoch number.
    pub epoch: u64,
    /// Chain index within epoch.
    pub chain_index: u32,
    /// Derived message key.
    pub key: Vec<u8>,
}

/// `[VERIFIED]` Validate that message keys are uniquely bound to (epoch, chain_index).
pub fn validate_msg_key_binding(
    bindings: &[MsgKeyBinding],
) -> Result<(), MsgKeyBindError> {
    let mut seen_slots = BTreeSet::new();
    let mut seen_keys = BTreeSet::new();
    for b in bindings {
        if b.epoch == 0 {
            return Err(MsgKeyBindError::ZeroEpoch);
        }
        if b.chain_index >= MKDB_MAX_CHAIN_INDEX {
            return Err(MsgKeyBindError::ChainIndexOutOfBounds);
        }
        if b.key.len() != MKDB_KEY_LEN {
            return Err(MsgKeyBindError::KeyLengthWrong);
        }
        if b.key.iter().all(|&byte| byte == 0) {
            return Err(MsgKeyBindError::ZeroKey);
        }
        let slot = (b.epoch, b.chain_index);
        if !seen_slots.insert(slot) {
            return Err(MsgKeyBindError::BindingCollision);
        }
        if !seen_keys.insert(b.key.clone()) {
            return Err(MsgKeyBindError::DuplicateKey);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> Vec<u8> {
        vec![byte; MKDB_KEY_LEN]
    }

    fn binding(epoch: u64, chain_index: u32, byte: u8) -> MsgKeyBinding {
        MsgKeyBinding { epoch, chain_index, key: key(byte) }
    }

    fn valid_bindings() -> Vec<MsgKeyBinding> {
        vec![
            binding(1, 0, 0x01),
            binding(1, 1, 0x02),
            binding(2, 0, 0x03),
        ]
    }

    /// **MKDB-01** — binding collision rejected.
    #[test]
    fn mkdb_01_collision_rejected() {
        let bindings = vec![
            binding(1, 0, 0x01),
            binding(1, 0, 0x02),
        ];
        assert_eq!(
            validate_msg_key_binding(&bindings),
            Err(MsgKeyBindError::BindingCollision)
        );
    }

    /// **MKDB-02** — zero epoch rejected.
    #[test]
    fn mkdb_02_zero_epoch_rejected() {
        assert_eq!(
            validate_msg_key_binding(&[binding(0, 0, 0x01)]),
            Err(MsgKeyBindError::ZeroEpoch)
        );
    }

    /// **MKDB-03** — chain index out of bounds rejected.
    #[test]
    fn mkdb_03_chain_oob_rejected() {
        assert_eq!(
            validate_msg_key_binding(&[binding(1, MKDB_MAX_CHAIN_INDEX, 0x01)]),
            Err(MsgKeyBindError::ChainIndexOutOfBounds)
        );
    }

    /// **MKDB-04** — key length wrong rejected.
    #[test]
    fn mkdb_04_key_len_rejected() {
        let b = MsgKeyBinding { epoch: 1, chain_index: 0, key: vec![0x01; 16] };
        assert_eq!(
            validate_msg_key_binding(&[b]),
            Err(MsgKeyBindError::KeyLengthWrong)
        );
    }

    /// **MKDB-05** — zero key rejected.
    #[test]
    fn mkdb_05_zero_key_rejected() {
        assert_eq!(
            validate_msg_key_binding(&[binding(1, 0, 0x00)]),
            Err(MsgKeyBindError::ZeroKey)
        );
    }

    /// **MKDB-06** — duplicate key across bindings rejected.
    #[test]
    fn mkdb_06_dup_key_rejected() {
        let bindings = vec![
            binding(1, 0, 0xAA),
            binding(2, 0, 0xAA),
        ];
        assert_eq!(
            validate_msg_key_binding(&bindings),
            Err(MsgKeyBindError::DuplicateKey)
        );
    }

    /// **MKDB-07** — valid bindings accepted.
    #[test]
    fn mkdb_07_valid_accepted() {
        assert_eq!(validate_msg_key_binding(&valid_bindings()), Ok(()));
    }

    /// **MKDB-08** — empty accepted.
    #[test]
    fn mkdb_08_empty_accepted() {
        assert_eq!(validate_msg_key_binding(&[]), Ok(()));
    }

    /// **MKDB-09** — max chain index accepted.
    #[test]
    fn mkdb_09_max_chain_accepted() {
        assert_eq!(
            validate_msg_key_binding(&[binding(1, MKDB_MAX_CHAIN_INDEX - 1, 0x01)]),
            Ok(())
        );
    }

    /// **MKDB-10** — same epoch different chains accepted.
    #[test]
    fn mkdb_10_same_epoch_diff_chains_accepted() {
        let bindings = vec![
            binding(1, 0, 0x01),
            binding(1, 1, 0x02),
            binding(1, 2, 0x03),
        ];
        assert_eq!(validate_msg_key_binding(&bindings), Ok(()));
    }
}
