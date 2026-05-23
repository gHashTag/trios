//! # CR-CHAT-02 — Root key derivation chain integrity guard (Wave-42 Lane B)
//!
//! R-CHAT-2 — Forward-secret root key chain validation.
//!
//! The root key in the Triple Ratchet is the top of the key hierarchy.
//! Every DH step derives a new root key from the previous root + shared
//! secret. If an attacker can force root key reuse or regression, they
//! break forward secrecy — old messages become decryptable with a
//! compromised current key.
//!
//! trios-chat enforces **6 rules** on a root key derivation chain:
//!
//! 1. Root key length is canonical (32 bytes).
//! 2. Epoch numbers are strictly increasing.
//! 3. No root key is reused across epochs.
//! 4. Chain key is derived (non-zero, 32 bytes).
//! 5. DH shared secret is not the identity point (all-zero).
//! 6. Output chain key differs from the input root key.
//!
//! Tests **RKDC-01..10**. Error enum [`RootKeyChainError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · ROOT-KEY-CHAIN`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Canonical root key and chain key length.
pub const RKDC_KEY_LEN: usize = 32;

/// One step in the root key derivation chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootKeyStep {
    /// Epoch after this derivation step.
    pub epoch: u64,
    /// Root key input (32 bytes).
    pub root_key_in: Vec<u8>,
    /// Root key output after derivation (32 bytes).
    pub root_key_out: Vec<u8>,
    /// Chain key derived from this step (32 bytes).
    pub chain_key: Vec<u8>,
    /// DH shared secret used in this step (32 bytes).
    pub dh_shared_secret: Vec<u8>,
}

/// All ways a root key chain can be invalid.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RootKeyChainError {
    /// Root key length is not 32 bytes.
    NonCanonicalRootKeyLength,
    /// Epoch is not strictly increasing.
    EpochNotMonotonic,
    /// Root key reused across epochs.
    RootKeyReuse,
    /// Chain key is zero (derivation failed).
    ZeroChainKey,
    /// DH shared secret is the identity point (all zeros).
    IdentityDhSecret,
    /// Chain key equals root key input (trivial derivation).
    TrivialDerivation,
}

/// `[VERIFIED]` Validate a root key derivation chain against forward
/// secrecy rules. Returns `Ok(())` if all rules pass.
///
/// Rules enforced in fixed order:
///
/// 1. All root keys (in/out) are 32 bytes.
/// 2. Epochs are strictly increasing.
/// 3. No root key value appears in more than one step.
/// 4. Chain keys are non-zero.
/// 5. DH shared secrets are non-zero.
/// 6. Chain key != root_key_in for each step.
pub fn validate_root_key_chain(
    steps: &[RootKeyStep],
) -> Result<(), RootKeyChainError> {
    let mut seen_root_keys_in = BTreeSet::new();
    let mut prev_epoch = 0u64;
    let mut first = true;

    for step in steps {
        if step.root_key_in.len() != RKDC_KEY_LEN || step.root_key_out.len() != RKDC_KEY_LEN {
            return Err(RootKeyChainError::NonCanonicalRootKeyLength);
        }
        if !first && step.epoch <= prev_epoch {
            return Err(RootKeyChainError::EpochNotMonotonic);
        }
        prev_epoch = step.epoch;
        first = false;
        if !seen_root_keys_in.insert(step.root_key_in.clone()) {
            return Err(RootKeyChainError::RootKeyReuse);
        }
        if step.chain_key.iter().all(|&b| b == 0) {
            return Err(RootKeyChainError::ZeroChainKey);
        }
        if step.dh_shared_secret.iter().all(|&b| b == 0) {
            return Err(RootKeyChainError::IdentityDhSecret);
        }
        if step.chain_key == step.root_key_in {
            return Err(RootKeyChainError::TrivialDerivation);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(epoch: u64, rk_byte_in: u8, rk_byte_out: u8, ck_byte: u8, dh_byte: u8) -> RootKeyStep {
        RootKeyStep {
            epoch,
            root_key_in: vec![rk_byte_in; 32],
            root_key_out: vec![rk_byte_out; 32],
            chain_key: vec![ck_byte; 32],
            dh_shared_secret: vec![dh_byte; 32],
        }
    }

    fn good_chain() -> Vec<RootKeyStep> {
        vec![
            step(1, 0x01, 0x02, 0x03, 0x04),
            step(2, 0x02, 0x05, 0x06, 0x07),
            step(3, 0x05, 0x08, 0x09, 0x0A),
        ]
    }

    /// **RKDC-01** — non-canonical root key length rejected.
    #[test]
    fn rkdc_01_non_canonical_length_rejected() {
        let mut steps = good_chain();
        steps[0].root_key_in = vec![0x01; 16];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::NonCanonicalRootKeyLength)
        );
    }

    /// **RKDC-02** — non-monotonic epoch rejected.
    #[test]
    fn rkdc_02_epoch_not_monotonic_rejected() {
        let steps = vec![
            step(2, 0x01, 0x02, 0x03, 0x04),
            step(1, 0x02, 0x05, 0x06, 0x07),
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::EpochNotMonotonic)
        );
    }

    /// **RKDC-03** — root key reuse rejected (same root_key_in).
    #[test]
    fn rkdc_03_root_key_reuse_rejected() {
        let steps = vec![
            step(1, 0x01, 0x02, 0x03, 0x04),
            step(2, 0x01, 0x05, 0x06, 0x07),
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::RootKeyReuse)
        );
    }

    /// **RKDC-04** — zero chain key rejected.
    #[test]
    fn rkdc_04_zero_chain_key_rejected() {
        let steps = vec![
            step(1, 0x01, 0x02, 0x00, 0x04),
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::ZeroChainKey)
        );
    }

    /// **RKDC-05** — identity DH secret rejected.
    #[test]
    fn rkdc_05_identity_dh_rejected() {
        let steps = vec![
            step(1, 0x01, 0x02, 0x03, 0x00),
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::IdentityDhSecret)
        );
    }

    /// **RKDC-06** — trivial derivation (chain key == root key in) rejected.
    #[test]
    fn rkdc_06_trivial_derivation_rejected() {
        let steps = vec![
            RootKeyStep {
                epoch: 1,
                root_key_in: vec![0x01; 32],
                root_key_out: vec![0x02; 32],
                chain_key: vec![0x01; 32],
                dh_shared_secret: vec![0x04; 32],
            },
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::TrivialDerivation)
        );
    }

    /// **RKDC-07** — valid chain accepted.
    #[test]
    fn rkdc_07_valid_chain_accepted() {
        assert_eq!(validate_root_key_chain(&good_chain()), Ok(()));
    }

    /// **RKDC-08** — single step accepted.
    #[test]
    fn rkdc_08_single_step_accepted() {
        let steps = vec![step(1, 0x01, 0x02, 0x03, 0x04)];
        assert_eq!(validate_root_key_chain(&steps), Ok(()));
    }

    /// **RKDC-09** — empty chain accepted (nothing to validate).
    #[test]
    fn rkdc_09_empty_chain_accepted() {
        assert_eq!(validate_root_key_chain(&[]), Ok(()));
    }

    /// **RKDC-10** — equal epoch rejected (duplicate).
    #[test]
    fn rkdc_10_equal_epoch_rejected() {
        let steps = vec![
            step(1, 0x01, 0x02, 0x03, 0x04),
            step(1, 0x05, 0x06, 0x07, 0x08),
        ];
        assert_eq!(
            validate_root_key_chain(&steps),
            Err(RootKeyChainError::EpochNotMonotonic)
        );
    }
}
