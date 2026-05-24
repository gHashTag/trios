//! # CR-CHAT-01 — Signature algorithm pinning guard (Wave-71 Lane B)
//!
//! IDENTITY — signature algorithm is pinned in credential, R-CHAT-1.
//!
//! An identity credential pins a specific signature algorithm (e.g.
//! Ed25519). If the algorithm can be downgraded:
//!
//! * **Algorithm downgrade** — attacker forces a weaker algorithm
//!   (e.g. from Ed25519 to a broken RSA variant).
//! * **Cross-algorithm confusion** — same key material interpreted
//!   under two algorithms, enabling forgery.
//! * **Protocol mismatch** — credential says Ed25519 but the wire
//!   message uses ECDSA, bypassing the pin.
//!
//! This is distinct from PSAD (prekey signature algo downgrade) which
//! operates at the prekey bundle level. SAPN operates at the identity
//! credential level.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Credential algorithm matches pinned algorithm.
//! 2. Wire algorithm matches pinned algorithm.
//! 3. Algorithm ID is in the allowed set.
//! 4. No algorithm ID is zero (reserved).
//! 5. Pinning list is non-empty.
//! 6. Pinning list size <= `SAPN_MAX_PINS`.
//!
//! Tests **SAPN-01..10**. Error enum [`SigAlgoPinError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * SIG-ALGO-PINNING`

#![forbid(unsafe_code)]

/// Maximum pinning list size.
pub const SAPN_MAX_PINS: usize = 8;

/// Allowed algorithm IDs (Ed25519 = 1, Ed448 = 2, ECDSA-P256 = 3).
pub const SAPN_ALLOWED_ALGOS: &[u16] = &[1, 2, 3];

/// All ways signature algorithm pinning can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SigAlgoPinError {
    /// Algorithm mismatch with pin.
    AlgoMismatch,
    /// Wire algorithm mismatch.
    WireAlgoMismatch,
    /// Algorithm not in allowed set.
    AlgoNotAllowed,
    /// Zero algorithm ID.
    ZeroAlgoId,
    /// Empty pinning list.
    EmptyPins,
    /// Too many pins.
    TooManyPins,
}

/// `[VERIFIED]` Validate that credential and wire algorithms match the pinned set.
pub fn validate_sig_algo_pinning(
    pins: &[u16],
    credential_algo: u16,
    wire_algo: u16,
) -> Result<(), SigAlgoPinError> {
    if pins.is_empty() {
        return Err(SigAlgoPinError::EmptyPins);
    }
    if pins.len() > SAPN_MAX_PINS {
        return Err(SigAlgoPinError::TooManyPins);
    }
    for &algo in pins {
        if algo == 0 {
            return Err(SigAlgoPinError::ZeroAlgoId);
        }
    }
    if !SAPN_ALLOWED_ALGOS.contains(&credential_algo) {
        return Err(SigAlgoPinError::AlgoNotAllowed);
    }
    if !pins.contains(&credential_algo) {
        return Err(SigAlgoPinError::AlgoMismatch);
    }
    if wire_algo != credential_algo {
        return Err(SigAlgoPinError::WireAlgoMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_pins() -> Vec<u16> {
        vec![1, 2]
    }

    /// **SAPN-01** — algorithm mismatch rejected.
    #[test]
    fn sapn_01_mismatch_rejected() {
        assert_eq!(
            validate_sig_algo_pinning(&valid_pins(), 3, 3),
            Err(SigAlgoPinError::AlgoMismatch)
        );
    }

    /// **SAPN-02** — wire algorithm mismatch rejected.
    #[test]
    fn sapn_02_wire_mismatch_rejected() {
        assert_eq!(
            validate_sig_algo_pinning(&valid_pins(), 1, 2),
            Err(SigAlgoPinError::WireAlgoMismatch)
        );
    }

    /// **SAPN-03** — algorithm not allowed rejected.
    #[test]
    fn sapn_03_not_allowed_rejected() {
        assert_eq!(
            validate_sig_algo_pinning(&[1], 99, 99),
            Err(SigAlgoPinError::AlgoNotAllowed)
        );
    }

    /// **SAPN-04** — zero algorithm ID rejected.
    #[test]
    fn sapn_04_zero_rejected() {
        assert_eq!(
            validate_sig_algo_pinning(&[0, 1], 1, 1),
            Err(SigAlgoPinError::ZeroAlgoId)
        );
    }

    /// **SAPN-05** — empty pins rejected.
    #[test]
    fn sapn_05_empty_rejected() {
        assert_eq!(
            validate_sig_algo_pinning(&[], 1, 1),
            Err(SigAlgoPinError::EmptyPins)
        );
    }

    /// **SAPN-06** — too many pins rejected.
    #[test]
    fn sapn_06_too_many_rejected() {
        let pins: Vec<u16> = (0..=SAPN_MAX_PINS).map(|i| (i % 3 + 1) as u16).collect();
        assert_eq!(
            validate_sig_algo_pinning(&pins, 1, 1),
            Err(SigAlgoPinError::TooManyPins)
        );
    }

    /// **SAPN-07** — valid pinning accepted.
    #[test]
    fn sapn_07_valid_accepted() {
        assert_eq!(validate_sig_algo_pinning(&valid_pins(), 1, 1), Ok(()));
    }

    /// **SAPN-08** — second pin accepted.
    #[test]
    fn sapn_08_second_pin_accepted() {
        assert_eq!(validate_sig_algo_pinning(&valid_pins(), 2, 2), Ok(()));
    }

    /// **SAPN-09** — single pin accepted.
    #[test]
    fn sapn_09_single_pin_accepted() {
        assert_eq!(validate_sig_algo_pinning(&[1], 1, 1), Ok(()));
    }

    /// **SAPN-10** — all allowed algos pinned accepted.
    #[test]
    fn sapn_10_all_allowed_accepted() {
        assert_eq!(
            validate_sig_algo_pinning(SAPN_ALLOWED_ALGOS, 3, 3),
            Ok(())
        );
    }
}
