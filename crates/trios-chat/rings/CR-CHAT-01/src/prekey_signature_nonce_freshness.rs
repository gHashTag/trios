//! # CR-CHAT-01 — Prekey signature nonce freshness guard (Wave-62 Lane A)
//!
//! IDENTITY — signature nonce must be unique per bundle, R-CHAT-4.
//!
//! Each prekey bundle signature includes a nonce to ensure uniqueness.
//! An attacker who observes a valid (nonce, signature) pair can attempt
//! to reuse it with a modified payload:
//!
//! * **Nonce reuse** — same nonce with different prekey → signature
//!   appears valid if the verifier doesn't track nonces.
//! * **Counter replay** — reuse an old nonce to make a stale bundle
//!   look fresh.
//! * **Multi-bundle collision** — two bundles signed with the same
//!   nonce under the same identity key.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Nonce is non-empty.
//! 2. Nonce length = `PSNF_NONCE_LEN`.
//! 3. No nonce reuse under the same identity key.
//! 4. Nonce is not all-zeros.
//! 5. Max tracked nonces <= `PSNF_MAX_TRACKED`.
//! 6. Nonce must be unique globally (cross-identity).
//!
//! Tests **PSNF-01..10**. Error enum [`NonceFreshnessError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * NONCE-FRESHNESS`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Expected nonce length.
pub const PSNF_NONCE_LEN: usize = 16;

/// Maximum tracked nonces.
pub const PSNF_MAX_TRACKED: usize = 512;

/// All ways nonce freshness can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NonceFreshnessError {
    /// Nonce is empty.
    EmptyNonce,
    /// Wrong nonce length.
    WrongLength,
    /// Nonce reused under same identity key.
    NonceReuse,
    /// All-zero nonce.
    ZeroNonce,
    /// Too many tracked nonces.
    TooManyTracked,
    /// Global nonce collision.
    GlobalCollision,
}

/// Nonce tracker.
#[derive(Debug, Default)]
pub struct NonceTracker {
    per_key: BTreeSet<([u8; 32], [u8; PSNF_NONCE_LEN])>,
    global: BTreeSet<[u8; PSNF_NONCE_LEN]>,
}

impl NonceTracker {
    /// Create empty tracker.
    pub fn new() -> Self { Self::default() }

    /// `[VERIFIED]` Validate and register a nonce.
    pub fn track(
        &mut self,
        identity_key: &[u8; 32],
        nonce: &[u8],
    ) -> Result<(), NonceFreshnessError> {
        if nonce.is_empty() {
            return Err(NonceFreshnessError::EmptyNonce);
        }
        if nonce.len() != PSNF_NONCE_LEN {
            return Err(NonceFreshnessError::WrongLength);
        }
        let mut nonce_arr = [0u8; PSNF_NONCE_LEN];
        nonce_arr.copy_from_slice(nonce);
        if nonce_arr == [0u8; PSNF_NONCE_LEN] {
            return Err(NonceFreshnessError::ZeroNonce);
        }
        if self.per_key.len() >= PSNF_MAX_TRACKED {
            return Err(NonceFreshnessError::TooManyTracked);
        }
        let pair = (*identity_key, nonce_arr);
        if self.per_key.contains(&pair) {
            return Err(NonceFreshnessError::NonceReuse);
        }
        if self.global.contains(&nonce_arr) {
            return Err(NonceFreshnessError::GlobalCollision);
        }
        self.per_key.insert(pair);
        self.global.insert(nonce_arr);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID_A: [u8; 32] = [0xAA; 32];
    const ID_B: [u8; 32] = [0xBB; 32];

    fn nonce(byte: u8) -> Vec<u8> {
        vec![byte; PSNF_NONCE_LEN]
    }

    /// **PSNF-01** — empty nonce rejected.
    #[test]
    fn psnf_01_empty_rejected() {
        let mut t = NonceTracker::new();
        assert_eq!(t.track(&ID_A, &[]), Err(NonceFreshnessError::EmptyNonce));
    }

    /// **PSNF-02** — wrong length rejected.
    #[test]
    fn psnf_02_wrong_len_rejected() {
        let mut t = NonceTracker::new();
        assert_eq!(t.track(&ID_A, &[1u8; 8]), Err(NonceFreshnessError::WrongLength));
    }

    /// **PSNF-03** — nonce reuse rejected.
    #[test]
    fn psnf_03_reuse_rejected() {
        let mut t = NonceTracker::new();
        t.track(&ID_A, &nonce(1)).unwrap();
        assert_eq!(t.track(&ID_A, &nonce(1)), Err(NonceFreshnessError::NonceReuse));
    }

    /// **PSNF-04** — zero nonce rejected.
    #[test]
    fn psnf_04_zero_rejected() {
        let mut t = NonceTracker::new();
        assert_eq!(t.track(&ID_A, &nonce(0)), Err(NonceFreshnessError::ZeroNonce));
    }

    /// **PSNF-05** — too many tracked rejected.
    #[test]
    fn psnf_05_too_many_rejected() {
        let mut t = NonceTracker::new();
        for i in 0..PSNF_MAX_TRACKED {
            let mut n = [0u8; PSNF_NONCE_LEN];
            let idx = ((i + 1) as u64).to_le_bytes();
            n[..8].copy_from_slice(&idx);
            t.track(&ID_A, &n).unwrap();
        }
        let mut last = [0u8; PSNF_NONCE_LEN];
        last[0] = 0xFE;
        assert_eq!(t.track(&ID_A, &last), Err(NonceFreshnessError::TooManyTracked));
    }

    /// **PSNF-06** — global collision rejected.
    #[test]
    fn psnf_06_global_collision_rejected() {
        let mut t = NonceTracker::new();
        t.track(&ID_A, &nonce(1)).unwrap();
        assert_eq!(t.track(&ID_B, &nonce(1)), Err(NonceFreshnessError::GlobalCollision));
    }

    /// **PSNF-07** — valid nonce accepted.
    #[test]
    fn psnf_07_valid_accepted() {
        let mut t = NonceTracker::new();
        assert_eq!(t.track(&ID_A, &nonce(1)), Ok(()));
    }

    /// **PSNF-08** — different nonce same key accepted.
    #[test]
    fn psnf_08_diff_nonce_accepted() {
        let mut t = NonceTracker::new();
        t.track(&ID_A, &nonce(1)).unwrap();
        assert_eq!(t.track(&ID_A, &nonce(2)), Ok(()));
    }

    /// **PSNF-09** — different key different nonce accepted.
    #[test]
    fn psnf_09_diff_key_accepted() {
        let mut t = NonceTracker::new();
        t.track(&ID_A, &nonce(1)).unwrap();
        assert_eq!(t.track(&ID_B, &nonce(2)), Ok(()));
    }

    /// **PSNF-10** — max tracked accepted.
    #[test]
    fn psnf_10_max_accepted() {
        let mut t = NonceTracker::new();
        for i in 0..PSNF_MAX_TRACKED {
            let mut n = [0u8; PSNF_NONCE_LEN];
            let idx = ((i + 1) as u64).to_le_bytes();
            n[..8].copy_from_slice(&idx);
            t.track(&ID_A, &n).unwrap();
        }
        assert_eq!(t.per_key.len(), PSNF_MAX_TRACKED);
    }
}
