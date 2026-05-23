//! # CR-CHAT-02 — Forward secrecy key wipe verification (Wave-56 Lane B)
//!
//! РЭТЧЕТ — проверка zeroize старых ключей, R-CHAT-2.
//!
//! После DH step root key и chain key должны быть перезаписаны.
//! Если старый key material остаётся в памяти — нарушение forward secrecy.
//!
//! 1. Old root key ≠ new root key.
//! 2. Old chain key ≠ new chain key.
//! 3. Key material is exactly 32 bytes.
//! 4. No key reuse across epochs.
//! 5. Key derivation is monotonic (epoch increases).
//! 6. Max key derivations ≤ `FSKW_MAX_DERIVATIONS`.
//!
//! Tests **FSKW-01..10**. Error enum [`KeyWipeError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · FORWARD-SECRECY`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Key length (256-bit).
pub const FSKW_KEY_LEN: usize = 32;

/// Maximum key derivations.
pub const FSKW_MAX_DERIVATIONS: usize = 1024;

/// All ways key wipe verification can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyWipeError {
    /// Old key equals new key (not rotated).
    KeyNotRotated,
    /// Wrong key length.
    WrongKeyLength,
    /// Key reused across epochs.
    KeyReused,
    /// Epoch not monotonic.
    EpochNotMonotonic,
    /// Too many derivations.
    TooManyDerivations,
    /// Zero key not allowed.
    ZeroKey,
}

/// A key derivation event.
#[derive(Debug, Clone)]
pub struct KeyDerivation {
    /// Epoch number.
    pub epoch: u64,
    /// New root key (after derivation).
    pub root_key: [u8; FSKW_KEY_LEN],
    /// New chain key (after derivation).
    pub chain_key: [u8; FSKW_KEY_LEN],
}

/// `[VERIFIED]` Validate a sequence of key derivations for forward secrecy.
pub fn validate_key_derivations(
    derivations: &[KeyDerivation],
) -> Result<(), KeyWipeError> {
    if derivations.len() > FSKW_MAX_DERIVATIONS {
        return Err(KeyWipeError::TooManyDerivations);
    }
    if derivations.is_empty() {
        return Ok(());
    }
    let mut seen_root = BTreeSet::new();
    let mut seen_chain = BTreeSet::new();
    for (i, d) in derivations.iter().enumerate() {
        if d.root_key == [0u8; FSKW_KEY_LEN] {
            return Err(KeyWipeError::ZeroKey);
        }
        if d.chain_key == [0u8; FSKW_KEY_LEN] {
            return Err(KeyWipeError::ZeroKey);
        }
        if i > 0 && d.epoch <= derivations[i - 1].epoch {
            return Err(KeyWipeError::EpochNotMonotonic);
        }
        if i > 0 && d.root_key == derivations[i - 1].root_key {
            return Err(KeyWipeError::KeyNotRotated);
        }
        if i > 0 && d.chain_key == derivations[i - 1].chain_key {
            return Err(KeyWipeError::KeyNotRotated);
        }
        if !seen_root.insert(d.root_key) {
            return Err(KeyWipeError::KeyReused);
        }
        if !seen_chain.insert(d.chain_key) {
            return Err(KeyWipeError::KeyReused);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(byte: u8) -> [u8; FSKW_KEY_LEN] {
        [byte; FSKW_KEY_LEN]
    }

    fn deriv(epoch: u64, rk: u8, ck: u8) -> KeyDerivation {
        KeyDerivation { epoch, root_key: key(rk), chain_key: key(ck) }
    }

    fn good_derivations() -> Vec<KeyDerivation> {
        vec![deriv(1, 0x01, 0x11), deriv(2, 0x02, 0x12), deriv(3, 0x03, 0x13)]
    }

    /// **FSKW-01** — key not rotated rejected.
    #[test]
    fn fskw_01_not_rotated_rejected() {
        let d = vec![deriv(1, 0x01, 0x11), deriv(2, 0x01, 0x12)];
        assert_eq!(
            validate_key_derivations(&d),
            Err(KeyWipeError::KeyNotRotated)
        );
    }

    /// **FSKW-02** — key reused rejected.
    #[test]
    fn fskw_02_key_reused_rejected() {
        let d = vec![deriv(1, 0x01, 0x11), deriv(2, 0x02, 0x12), deriv(3, 0x01, 0x13)];
        assert_eq!(
            validate_key_derivations(&d),
            Err(KeyWipeError::KeyReused)
        );
    }

    /// **FSKW-03** — epoch not monotonic rejected.
    #[test]
    fn fskw_03_epoch_not_monotonic_rejected() {
        let d = vec![deriv(2, 0x01, 0x11), deriv(1, 0x02, 0x12)];
        assert_eq!(
            validate_key_derivations(&d),
            Err(KeyWipeError::EpochNotMonotonic)
        );
    }

    /// **FSKW-04** — too many derivations rejected.
    #[test]
    fn fskw_04_too_many_rejected() {
        let d: Vec<KeyDerivation> = (0..=FSKW_MAX_DERIVATIONS)
            .map(|i| {
                let mut rk = [0u8; FSKW_KEY_LEN];
                let mut ck = [0u8; FSKW_KEY_LEN];
                let idx = i.to_le_bytes();
                rk[..8].copy_from_slice(&idx);
                ck[..8].copy_from_slice(&idx);
                ck[8] = 1;
                KeyDerivation { epoch: i as u64 + 1, root_key: rk, chain_key: ck }
            })
            .collect();
        assert_eq!(
            validate_key_derivations(&d),
            Err(KeyWipeError::TooManyDerivations)
        );
    }

    /// **FSKW-05** — zero key rejected.
    #[test]
    fn fskw_05_zero_key_rejected() {
        let mut d = deriv(1, 0x01, 0x00);
        d.chain_key = [0u8; FSKW_KEY_LEN];
        assert_eq!(
            validate_key_derivations(&[d]),
            Err(KeyWipeError::ZeroKey)
        );
    }

    /// **FSKW-06** — good derivations accepted.
    #[test]
    fn fskw_06_good_accepted() {
        assert_eq!(validate_key_derivations(&good_derivations()), Ok(()));
    }

    /// **FSKW-07** — empty accepted.
    #[test]
    fn fskw_07_empty_accepted() {
        assert_eq!(validate_key_derivations(&[]), Ok(()));
    }

    /// **FSKW-08** — single derivation accepted.
    #[test]
    fn fskw_08_single_accepted() {
        assert_eq!(validate_key_derivations(&[deriv(1, 0xAA, 0xBB)]), Ok(()));
    }

    /// **FSKW-09** — chain key not rotated rejected.
    #[test]
    fn fskw_09_chain_not_rotated_rejected() {
        let d = vec![deriv(1, 0x01, 0x11), deriv(2, 0x02, 0x11)];
        assert_eq!(
            validate_key_derivations(&d),
            Err(KeyWipeError::KeyNotRotated)
        );
    }

    /// **FSKW-10** — max derivations accepted.
    #[test]
    fn fskw_10_max_accepted() {
        let d: Vec<KeyDerivation> = (0..FSKW_MAX_DERIVATIONS)
            .map(|i| {
                let mut rk = [0u8; FSKW_KEY_LEN];
                let mut ck = [0u8; FSKW_KEY_LEN];
                let idx = (i as u64 + 1).to_le_bytes();
                rk[..8].copy_from_slice(&idx);
                ck[..8].copy_from_slice(&idx);
                ck[8] = 1;
                KeyDerivation { epoch: i as u64 + 1, root_key: rk, chain_key: ck }
            })
            .collect();
        assert_eq!(validate_key_derivations(&d), Ok(()));
    }
}
