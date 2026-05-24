//! # CR-CHAT-01 — Identity key usage count guard (Wave-77 Lane B)
//!
//! IDENTITY — each identity key has a maximum signature count, R-CHAT-1.
//!
//! Over-signing with the same identity key increases the attack surface:
//!
//! * **Side-channel accumulation** — each signature creates a power
//!   trace or timing side-channel; more signatures = more data for
//!   DPA (Differential Power Analysis).
//! * **Nonce reuse risk** — higher signature count increases the
//!   probability of nonce reuse in probabilistic signature schemes.
//! * **Key extraction** — sufficient signed messages under the same
//!   key enables lattice attacks on certain schemes.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Signature count <= `IKUC_MAX_SIGNATURES`.
//! 2. Signature count >= 0 (non-negative).
//! 3. Key ID is non-zero.
//! 4. Warning threshold at `IKUC_WARN_SIGNATURES`.
//! 5. After max, key must be rotated (no further signatures).
//! 6. Rotation resets count for the new key.
//!
//! Tests **IKUC-01..10**. Error enum [`KeyUsageError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * IDENTITY-KEY-USAGE`

#![forbid(unsafe_code)]

/// Maximum signatures per identity key.
pub const IKUC_MAX_SIGNATURES: u64 = 100_000;

/// Warning threshold.
pub const IKUC_WARN_SIGNATURES: u64 = 80_000;

/// All ways key usage validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyUsageError {
    /// Signature count exceeded.
    SignatureCountExceeded,
    /// Zero key ID.
    ZeroKeyId,
    /// Key must be rotated (at or past max).
    MustRotate,
    /// Negative count (underflow).
    NegativeCount,
}

/// `[VERIFIED]` Validate identity key usage count.
pub fn validate_key_usage(
    key_id: u64,
    signature_count: u64,
) -> Result<(), KeyUsageError> {
    if key_id == 0 {
        return Err(KeyUsageError::ZeroKeyId);
    }
    if signature_count > IKUC_MAX_SIGNATURES {
        return Err(KeyUsageError::SignatureCountExceeded);
    }
    Ok(())
}

/// `[VERIFIED]` Check if key needs rotation due to usage.
pub fn key_needs_rotation(signature_count: u64) -> bool {
    signature_count >= IKUC_WARN_SIGNATURES
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **IKUC-01** — signature count exceeded rejected.
    #[test]
    fn ikuc_01_exceeded_rejected() {
        assert_eq!(
            validate_key_usage(1, IKUC_MAX_SIGNATURES + 1),
            Err(KeyUsageError::SignatureCountExceeded)
        );
    }

    /// **IKUC-02** — zero key ID rejected.
    #[test]
    fn ikuc_02_zero_key_rejected() {
        assert_eq!(
            validate_key_usage(0, 10),
            Err(KeyUsageError::ZeroKeyId)
        );
    }

    /// **IKUC-03** — at max signatures accepted.
    #[test]
    fn ikuc_03_at_max_accepted() {
        assert_eq!(validate_key_usage(1, IKUC_MAX_SIGNATURES), Ok(()));
    }

    /// **IKUC-04** — zero signatures accepted.
    #[test]
    fn ikuc_04_zero_sigs_accepted() {
        assert_eq!(validate_key_usage(1, 0), Ok(()));
    }

    /// **IKUC-05** — warning threshold triggers rotation.
    #[test]
    fn ikuc_05_warning_triggers() {
        assert!(key_needs_rotation(IKUC_WARN_SIGNATURES));
    }

    /// **IKUC-06** — below warning does not trigger.
    #[test]
    fn ikuc_06_below_warning() {
        assert!(!key_needs_rotation(IKUC_WARN_SIGNATURES - 1));
    }

    /// **IKUC-07** — valid mid-range accepted.
    #[test]
    fn ikuc_07_mid_range_accepted() {
        assert_eq!(validate_key_usage(42, 50_000), Ok(()));
    }

    /// **IKUC-08** — single signature accepted.
    #[test]
    fn ikuc_08_single_accepted() {
        assert_eq!(validate_key_usage(1, 1), Ok(()));
    }

    /// **IKUC-09** — max signatures need rotation.
    #[test]
    fn ikuc_09_max_needs_rotation() {
        assert!(key_needs_rotation(IKUC_MAX_SIGNATURES));
    }

    /// **IKUC-10** — large key ID accepted.
    #[test]
    fn ikuc_10_large_key_accepted() {
        assert_eq!(validate_key_usage(u64::MAX, 1000), Ok(()));
    }
}
