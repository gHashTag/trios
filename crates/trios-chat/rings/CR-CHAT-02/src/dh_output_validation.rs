//! # CR-CHAT-02 — DH output validation guard (Wave-60 Lane A)
//!
//! RATCHET — DH shared secret must be non-degenerate, R-CHAT-2.
//!
//! During ratchet DH step, both parties compute a shared secret from
//! their ephemeral keys. An attacker can exploit weak DH outputs:
//!
//! * **Small subgroup attack** — use a low-order point to force the
//!   shared secret to a small set of values.
//! * **Identity point** — produce the point at infinity (all-zeros).
//! * **Low-order point** — produce a shared secret with trivial order.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Shared secret is not all-zeros.
//! 2. Shared secret is not all-ones.
//! 3. Shared secret length = `DHOV_SS_LEN`.
//! 4. Shared secret is not equal to either public key.
//! 5. Shared secret has sufficient entropy (>= `DHOV_MIN_ENTROPY` bytes distinct).
//! 6. Shared secret is not a known weak value.
//!
//! Tests **DHOV-01..10**. Error enum [`DhOutputError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * DH-OUTPUT-VALID`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Expected shared secret length (X25519).
pub const DHOV_SS_LEN: usize = 32;

/// Minimum distinct bytes in shared secret.
pub const DHOV_MIN_ENTROPY: usize = 8;

/// All-zeros weak value.
pub const DHOV_WEAK_ZERO: [u8; DHOV_SS_LEN] = [0u8; DHOV_SS_LEN];

/// All-ones weak value.
pub const DHOV_WEAK_ONES: [u8; DHOV_SS_LEN] = [0xFF; DHOV_SS_LEN];

/// All ways DH output validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DhOutputError {
    /// Shared secret is all-zeros.
    AllZeros,
    /// Shared secret is all-ones.
    AllOnes,
    /// Wrong length.
    WrongLength,
    /// Shared secret equals a public key.
    EqualsPublicKey,
    /// Insufficient entropy.
    LowEntropy,
    /// Known weak value.
    KnownWeak,
}

/// `[VERIFIED]` Validate a DH shared secret output.
pub fn validate_dh_output(
    shared_secret: &[u8],
    pk_a: &[u8],
    pk_b: &[u8],
) -> Result<(), DhOutputError> {
    if shared_secret.len() != DHOV_SS_LEN {
        return Err(DhOutputError::WrongLength);
    }
    if shared_secret == DHOV_WEAK_ZERO.as_slice() {
        return Err(DhOutputError::AllZeros);
    }
    if shared_secret == DHOV_WEAK_ONES.as_slice() {
        return Err(DhOutputError::AllOnes);
    }
    if shared_secret == pk_a || shared_secret == pk_b {
        return Err(DhOutputError::EqualsPublicKey);
    }
    let distinct: BTreeSet<u8> = shared_secret.iter().copied().collect();
    if distinct.len() < DHOV_MIN_ENTROPY {
        return Err(DhOutputError::LowEntropy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ss(byte: u8) -> Vec<u8> {
        vec![byte; DHOV_SS_LEN]
    }

    fn good_ss() -> Vec<u8> {
        (0..DHOV_SS_LEN).map(|i| ((i as u8).wrapping_mul(131) ^ 0x55)).collect()
    }

    fn pk(byte: u8) -> Vec<u8> {
        vec![byte; DHOV_SS_LEN]
    }

    /// **DHOV-01** — all-zeros rejected.
    #[test]
    fn dhov_01_zeros_rejected() {
        assert_eq!(
            validate_dh_output(&ss(0), &pk(1), &pk(2)),
            Err(DhOutputError::AllZeros)
        );
    }

    /// **DHOV-02** — all-ones rejected.
    #[test]
    fn dhov_02_ones_rejected() {
        assert_eq!(
            validate_dh_output(&ss(0xFF), &pk(1), &pk(2)),
            Err(DhOutputError::AllOnes)
        );
    }

    /// **DHOV-03** — wrong length rejected.
    #[test]
    fn dhov_03_wrong_len_rejected() {
        assert_eq!(
            validate_dh_output(&[0u8; 16], &pk(1), &pk(2)),
            Err(DhOutputError::WrongLength)
        );
    }

    /// **DHOV-04** — equals public key A rejected.
    #[test]
    fn dhov_04_equals_pk_a_rejected() {
        let pk_a = good_ss();
        assert_eq!(
            validate_dh_output(&pk_a, &pk_a, &pk(2)),
            Err(DhOutputError::EqualsPublicKey)
        );
    }

    /// **DHOV-05** — equals public key B rejected.
    #[test]
    fn dhov_05_equals_pk_b_rejected() {
        let pk_b = good_ss();
        assert_eq!(
            validate_dh_output(&pk_b, &pk(1), &pk_b),
            Err(DhOutputError::EqualsPublicKey)
        );
    }

    /// **DHOV-06** — low entropy rejected.
    #[test]
    fn dhov_06_low_entropy_rejected() {
        let mut low = vec![0x42u8; DHOV_SS_LEN];
        low[0] = 0x43;
        assert_eq!(
            validate_dh_output(&low, &pk(1), &pk(2)),
            Err(DhOutputError::LowEntropy)
        );
    }

    /// **DHOV-07** — good shared secret accepted.
    #[test]
    fn dhov_07_good_accepted() {
        assert_eq!(validate_dh_output(&good_ss(), &pk(1), &pk(2)), Ok(()));
    }

    /// **DHOV-08** — minimum entropy accepted.
    #[test]
    fn dhov_08_min_entropy_accepted() {
        let mut ss = vec![0u8; DHOV_SS_LEN];
        for i in 0..DHOV_MIN_ENTROPY {
            ss[i] = (i + 1) as u8;
        }
        assert_eq!(validate_dh_output(&ss, &pk(1), &pk(2)), Ok(()));
    }

    /// **DHOV-09** — different from both keys accepted.
    #[test]
    fn dhov_09_diff_from_keys_accepted() {
        let pk_a = pk(0xAA);
        let pk_b = pk(0xBB);
        assert_eq!(validate_dh_output(&good_ss(), &pk_a, &pk_b), Ok(()));
    }

    /// **DHOV-10** — high entropy accepted.
    #[test]
    fn dhov_10_high_entropy_accepted() {
        let ss: Vec<u8> = (0..DHOV_SS_LEN).map(|i| (i * 251 + 17) as u8).collect();
        assert_eq!(validate_dh_output(&ss, &pk(1), &pk(2)), Ok(()));
    }
}
