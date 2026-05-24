//! # CR-CHAT-02 — Root key forward secrecy verification guard (Wave-120 Lane A)
//!
//! RATCHET — after each DH ratchet step, the old root key must be
//! irrecoverable; retained root keys break forward secrecy.
//!
//! The Double Ratchet derives a new root key at every DH step. If the
//! old root key is not properly erased:
//!
//! * **Forward secrecy break** — compromising any single root key
//!   lets the attacker derive all future root keys if old keys are
//!   retained in memory.
//! * **Key chain linkage** — retained root keys create a chain that
//!   can be walked backwards to recover past session keys.
//! * **Memory forensic risk** — leftover root key material in memory
//!   is recoverable via core dumps or side-channel attacks.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. No root key hash may appear more than once.
//! 2. Root key hash must not be zero.
//! 3. Epoch must be strictly increasing.
//! 4. DH output hash must not be zero.
//! 5. No duplicate epochs.
//! 6. Total steps <= `RKFS_MAX_STEPS`.
//!
//! Tests **RKFS-01..10**. Error enum [`ForwardSecrecyError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * FORWARD-SECRET`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum DH ratchet steps per batch.
pub const RKFS_MAX_STEPS: usize = 1024;

/// Root key hash length.
pub const RKFS_HASH_LEN: usize = 32;

/// Epoch length.
pub const RKFS_EPOCH_LEN: usize = 8;

/// A DH ratchet step record.
#[derive(Debug, Clone)]
pub struct RatchetStep {
    /// Epoch number.
    pub epoch: u64,
    /// Hash of the new root key (must be unique).
    pub root_key_hash: [u8; RKFS_HASH_LEN],
    /// Hash of the DH output used in this step.
    pub dh_output_hash: [u8; RKFS_HASH_LEN],
}

/// All ways forward secrecy validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ForwardSecrecyError {
    /// Root key hash reused (forward secrecy violation).
    RootKeyReused { idx: usize, epoch: u64 },
    /// Zero root key hash.
    ZeroRootKey(usize),
    /// Non-monotonic epoch.
    NonMonotonicEpoch { idx: usize, prev: u64, current: u64 },
    /// Zero DH output hash.
    ZeroDhOutput(usize),
    /// Duplicate epoch.
    DuplicateEpoch { idx: usize, epoch: u64 },
    /// Too many steps.
    TooMany { got: usize, max: usize },
}

/// `[VERIFIED]` Validate root key forward secrecy.
pub fn validate_forward_secrecy(
    steps: &[RatchetStep],
) -> Result<(), ForwardSecrecyError> {
    if steps.len() > RKFS_MAX_STEPS {
        return Err(ForwardSecrecyError::TooMany {
            got: steps.len(),
            max: RKFS_MAX_STEPS,
        });
    }
    let mut seen_roots: BTreeSet<[u8; RKFS_HASH_LEN]> = BTreeSet::new();
    let mut seen_epochs: BTreeSet<u64> = BTreeSet::new();
    let mut prev_epoch: u64 = 0;
    for (i, s) in steps.iter().enumerate() {
        if s.root_key_hash == [0u8; RKFS_HASH_LEN] {
            return Err(ForwardSecrecyError::ZeroRootKey(i));
        }
        if s.dh_output_hash == [0u8; RKFS_HASH_LEN] {
            return Err(ForwardSecrecyError::ZeroDhOutput(i));
        }
        if !seen_epochs.insert(s.epoch) {
            return Err(ForwardSecrecyError::DuplicateEpoch { idx: i, epoch: s.epoch });
        }
        if i > 0 && s.epoch <= prev_epoch {
            return Err(ForwardSecrecyError::NonMonotonicEpoch {
                idx: i,
                prev: prev_epoch,
                current: s.epoch,
            });
        }
        if !seen_roots.insert(s.root_key_hash) {
            return Err(ForwardSecrecyError::RootKeyReused { idx: i, epoch: s.epoch });
        }
        prev_epoch = s.epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; RKFS_HASH_LEN] {
        [byte; RKFS_HASH_LEN]
    }

    fn step(epoch: u64, rk: u8, dh: u8) -> RatchetStep {
        RatchetStep { epoch, root_key_hash: hash(rk), dh_output_hash: hash(dh) }
    }

    fn valid_steps() -> Vec<RatchetStep> {
        vec![
            step(1, 0xA1, 0xD1),
            step(2, 0xA2, 0xD2),
            step(3, 0xA3, 0xD3),
        ]
    }

    /// **RKFS-01** — root key reuse rejected.
    #[test]
    fn rkfs_01_root_key_reused_rejected() {
        let ss = vec![
            step(1, 0xAA, 0xD1),
            step(2, 0xAA, 0xD2),
        ];
        assert_eq!(
            validate_forward_secrecy(&ss),
            Err(ForwardSecrecyError::RootKeyReused { idx: 1, epoch: 2 })
        );
    }

    /// **RKFS-02** — zero root key rejected.
    #[test]
    fn rkfs_02_zero_root_key_rejected() {
        let s = RatchetStep { epoch: 1, root_key_hash: [0u8; RKFS_HASH_LEN], dh_output_hash: hash(0xDD) };
        assert_eq!(
            validate_forward_secrecy(&[s]),
            Err(ForwardSecrecyError::ZeroRootKey(0))
        );
    }

    /// **RKFS-03** — non-monotonic epoch rejected.
    #[test]
    fn rkfs_03_non_monotonic_rejected() {
        let ss = vec![
            step(5, 0xA1, 0xD1),
            step(3, 0xA2, 0xD2),
        ];
        assert_eq!(
            validate_forward_secrecy(&ss),
            Err(ForwardSecrecyError::NonMonotonicEpoch { idx: 1, prev: 5, current: 3 })
        );
    }

    /// **RKFS-04** — zero DH output rejected.
    #[test]
    fn rkfs_04_zero_dh_rejected() {
        let s = RatchetStep { epoch: 1, root_key_hash: hash(0xAA), dh_output_hash: [0u8; RKFS_HASH_LEN] };
        assert_eq!(
            validate_forward_secrecy(&[s]),
            Err(ForwardSecrecyError::ZeroDhOutput(0))
        );
    }

    /// **RKFS-05** — duplicate epoch rejected.
    #[test]
    fn rkfs_05_duplicate_epoch_rejected() {
        let ss = vec![
            step(1, 0xA1, 0xD1),
            step(1, 0xA2, 0xD2),
        ];
        assert_eq!(
            validate_forward_secrecy(&ss),
            Err(ForwardSecrecyError::DuplicateEpoch { idx: 1, epoch: 1 })
        );
    }

    /// **RKFS-06** — too many rejected.
    #[test]
    fn rkfs_06_too_many_rejected() {
        let ss: Vec<RatchetStep> = (0..=RKFS_MAX_STEPS)
            .map(|i| {
                let mut rk = [0u8; RKFS_HASH_LEN];
                let val = (i as u64) + 1;
                rk[0..8].copy_from_slice(&val.to_be_bytes());
                let mut dh = [0u8; RKFS_HASH_LEN];
                dh[0..8].copy_from_slice(&(val + 10000).to_be_bytes());
                RatchetStep { epoch: val, root_key_hash: rk, dh_output_hash: dh }
            })
            .collect();
        assert_eq!(
            validate_forward_secrecy(&ss),
            Err(ForwardSecrecyError::TooMany {
                got: RKFS_MAX_STEPS + 1,
                max: RKFS_MAX_STEPS,
            })
        );
    }

    /// **RKFS-07** — valid accepted.
    #[test]
    fn rkfs_07_valid_accepted() {
        assert_eq!(validate_forward_secrecy(&valid_steps()), Ok(()));
    }

    /// **RKFS-08** — empty accepted.
    #[test]
    fn rkfs_08_empty_accepted() {
        assert_eq!(validate_forward_secrecy(&[]), Ok(()));
    }

    /// **RKFS-09** — single step accepted.
    #[test]
    fn rkfs_09_single_accepted() {
        let ss = vec![step(1, 0xAA, 0xDD)];
        assert_eq!(validate_forward_secrecy(&ss), Ok(()));
    }

    /// **RKFS-10** — max boundary accepted.
    #[test]
    fn rkfs_10_max_boundary_accepted() {
        let ss: Vec<RatchetStep> = (0..RKFS_MAX_STEPS)
            .map(|i| {
                let val = (i as u64) + 1;
                let mut rk = [0u8; RKFS_HASH_LEN];
                rk[0..8].copy_from_slice(&val.to_be_bytes());
                let mut dh = [0u8; RKFS_HASH_LEN];
                dh[0..8].copy_from_slice(&(val + 50000).to_be_bytes());
                RatchetStep { epoch: val, root_key_hash: rk, dh_output_hash: dh }
            })
            .collect();
        assert_eq!(validate_forward_secrecy(&ss), Ok(()));
    }
}
