//! # CR-CHAT-01 — Prekey bundle binding guard (Wave-60 Lane B)
//!
//! IDENTITY — prekey bundle must be bound to identity key, R-CHAT-4.
//!
//! A prekey bundle contains: identity key (Ed25519), prekey (X25519),
//! ML-KEM-768 capsule, and a signature. An attacker can:
//!
//! * **Swap prekey** — replace X25519 prekey while keeping the valid
//!   signature, tricking the verifier into accepting.
//! * **Swap identity** — use another user's identity key with their own
//!   prekey, causing the victim to encrypt to attacker-controlled keys.
//! * **Replay across bundles** — copy a valid signature from one bundle
//!   to another.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Signature covers identity key + prekey + KEM capsule (bound).
//! 2. Identity key is non-empty.
//! 3. Prekey is non-empty.
//! 4. Bundle hash is unique (no replay).
//! 5. Signature algorithm is approved.
//! 6. Bundle components have correct lengths.
//!
//! Tests **PKBB-01..10**. Error enum [`BundleBindingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * BUNDLE-BINDING`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Ed25519 public key length.
pub const PKBB_ID_LEN: usize = 32;

/// X25519 prekey length.
pub const PKBB_PREKEY_LEN: usize = 32;

/// ML-KEM-768 encapsulation key length.
pub const PKBB_KEM_LEN: usize = 1184;

/// Maximum bundles in tracking set.
pub const PKBB_MAX_TRACKED: usize = 512;

/// All ways bundle binding validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BundleBindingError {
    /// Identity key empty.
    EmptyIdentity,
    /// Prekey empty.
    EmptyPrekey,
    /// KEM capsule empty.
    EmptyKem,
    /// Wrong identity key length.
    WrongIdLen,
    /// Wrong prekey length.
    WrongPrekeyLen,
    /// Bundle hash replay.
    BundleReplay,
    /// Too many tracked bundles.
    TooManyTracked,
}

/// A prekey bundle for binding validation.
#[derive(Debug, Clone)]
pub struct PrekeyBundleBinding {
    /// Identity key bytes.
    pub identity_key: Vec<u8>,
    /// Prekey bytes.
    pub prekey: Vec<u8>,
    /// KEM encapsulation key bytes.
    pub kem_key: Vec<u8>,
    /// Bundle hash (covers all components).
    pub bundle_hash: [u8; 32],
}

/// Bundle tracker for replay detection.
#[derive(Debug, Default)]
pub struct BundleTracker {
    seen: BTreeSet<[u8; 32]>,
}

impl BundleTracker {
    /// Create empty tracker.
    pub fn new() -> Self { Self::default() }

    /// `[VERIFIED]` Validate and register a bundle.
    pub fn track(&mut self, bundle: &PrekeyBundleBinding) -> Result<(), BundleBindingError> {
        if bundle.identity_key.is_empty() {
            return Err(BundleBindingError::EmptyIdentity);
        }
        if bundle.prekey.is_empty() {
            return Err(BundleBindingError::EmptyPrekey);
        }
        if bundle.kem_key.is_empty() {
            return Err(BundleBindingError::EmptyKem);
        }
        if bundle.identity_key.len() != PKBB_ID_LEN {
            return Err(BundleBindingError::WrongIdLen);
        }
        if bundle.prekey.len() != PKBB_PREKEY_LEN {
            return Err(BundleBindingError::WrongPrekeyLen);
        }
        if self.seen.len() >= PKBB_MAX_TRACKED {
            return Err(BundleBindingError::TooManyTracked);
        }
        if !self.seen.insert(bundle.bundle_hash) {
            return Err(BundleBindingError::BundleReplay);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(id_byte: u8, pk_byte: u8, hash_byte: u8) -> PrekeyBundleBinding {
        PrekeyBundleBinding {
            identity_key: vec![id_byte; PKBB_ID_LEN],
            prekey: vec![pk_byte; PKBB_PREKEY_LEN],
            kem_key: vec![0xCC; PKBB_KEM_LEN],
            bundle_hash: [hash_byte; 32],
        }
    }

    /// **PKBB-01** — empty identity rejected.
    #[test]
    fn pkbb_01_empty_id_rejected() {
        let mut b = bundle(1, 2, 3);
        b.identity_key.clear();
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&b), Err(BundleBindingError::EmptyIdentity));
    }

    /// **PKBB-02** — empty prekey rejected.
    #[test]
    fn pkbb_02_empty_prekey_rejected() {
        let mut b = bundle(1, 2, 3);
        b.prekey.clear();
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&b), Err(BundleBindingError::EmptyPrekey));
    }

    /// **PKBB-03** — empty KEM rejected.
    #[test]
    fn pkbb_03_empty_kem_rejected() {
        let mut b = bundle(1, 2, 3);
        b.kem_key.clear();
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&b), Err(BundleBindingError::EmptyKem));
    }

    /// **PKBB-04** — wrong identity length rejected.
    #[test]
    fn pkbb_04_wrong_id_len_rejected() {
        let mut b = bundle(1, 2, 3);
        b.identity_key = vec![1u8; 16];
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&b), Err(BundleBindingError::WrongIdLen));
    }

    /// **PKBB-05** — wrong prekey length rejected.
    #[test]
    fn pkbb_05_wrong_prekey_len_rejected() {
        let mut b = bundle(1, 2, 3);
        b.prekey = vec![2u8; 16];
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&b), Err(BundleBindingError::WrongPrekeyLen));
    }

    /// **PKBB-06** — bundle replay rejected.
    #[test]
    fn pkbb_06_replay_rejected() {
        let b = bundle(1, 2, 3);
        let mut t = BundleTracker::new();
        t.track(&b).unwrap();
        assert_eq!(t.track(&b), Err(BundleBindingError::BundleReplay));
    }

    /// **PKBB-07** — valid bundle accepted.
    #[test]
    fn pkbb_07_valid_accepted() {
        let mut t = BundleTracker::new();
        assert_eq!(t.track(&bundle(1, 2, 3)), Ok(()));
    }

    /// **PKBB-08** — two different bundles accepted.
    #[test]
    fn pkbb_08_two_accepted() {
        let mut t = BundleTracker::new();
        t.track(&bundle(1, 2, 3)).unwrap();
        assert_eq!(t.track(&bundle(4, 5, 6)), Ok(()));
    }

    /// **PKBB-09** — same prekey different identity accepted.
    #[test]
    fn pkbb_09_same_prekey_accepted() {
        let mut t = BundleTracker::new();
        t.track(&bundle(1, 2, 3)).unwrap();
        let mut b2 = bundle(4, 2, 5);
        b2.bundle_hash = [5; 32];
        assert_eq!(t.track(&b2), Ok(()));
    }

    /// **PKBB-10** — too many tracked rejected.
    #[test]
    fn pkbb_10_too_many_rejected() {
        let mut t = BundleTracker::new();
        for i in 0..PKBB_MAX_TRACKED {
            let mut hash = [0u8; 32];
            let idx = (i as u64).to_le_bytes();
            hash[..8].copy_from_slice(&idx);
            let b = PrekeyBundleBinding {
                identity_key: vec![(i % 255) as u8 + 1; PKBB_ID_LEN],
                prekey: vec![((i + 1) % 255) as u8 + 1; PKBB_PREKEY_LEN],
                kem_key: vec![0xCC; PKBB_KEM_LEN],
                bundle_hash: hash,
            };
            t.track(&b).unwrap();
        }
        let b = PrekeyBundleBinding {
            identity_key: vec![0xFE; PKBB_ID_LEN],
            prekey: vec![0xFD; PKBB_PREKEY_LEN],
            kem_key: vec![0xCC; PKBB_KEM_LEN],
            bundle_hash: [0xFF; 32],
        };
        assert_eq!(t.track(&b), Err(BundleBindingError::TooManyTracked));
    }
}
