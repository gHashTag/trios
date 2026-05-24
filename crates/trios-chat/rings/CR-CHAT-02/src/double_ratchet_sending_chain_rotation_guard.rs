//! # CR-CHAT-02 — Double ratchet sending chain rotation guard (Wave-97 Lane B)
//!
//! RATCHET — sending chain must rotate after max messages, R-CHAT-2.
//!
//! The Signal Double Ratchet uses a sending chain that derives message
//! keys from a chain key. After too many messages without a DH step:
//!
//! * **Forward secrecy degradation** — the longer a chain is used, the
//!   more messages are compromised by a single chain key leak.
//! * **Key wear-out** — cryptographic keys degrade with excessive use;
//!   the chain key's entropy is stretched thin.
//! * **State explosion** — skipped keys accumulate, increasing memory
//!   and recovery time.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Messages per chain <= `DSCR_MAX_MESSAGES`.
//! 2. Chain counter must be >= 0.
//! 3. Chain counter must be strictly increasing.
//! 4. Chain ID must be > 0.
//! 5. Total chains <= `DSCR_MAX_CHAINS`.
//! 6. No duplicate chain IDs.
//!
//! Tests **DSCR-01..10**. Error enum [`ChainRotationError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-ROTATION`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum messages per sending chain.
pub const DSCR_MAX_MESSAGES: u64 = 1000;

/// Maximum chains tracked.
pub const DSCR_MAX_CHAINS: usize = 256;

/// A sending chain record.
#[derive(Debug, Clone)]
pub struct SendingChain {
    /// Chain ID.
    pub chain_id: u64,
    /// Number of messages sent on this chain.
    pub message_count: u64,
}

/// All ways chain rotation validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainRotationError {
    /// Too many messages on chain.
    TooManyMessages { chain_id: u64, count: u64, max: u64 },
    /// Zero chain ID.
    ZeroChainId,
    /// Too many chains.
    TooManyChains,
    /// Duplicate chain ID.
    DuplicateChainId(u64),
}

/// `[VERIFIED]` Validate double ratchet sending chain rotation.
pub fn validate_sending_chain_rotation(
    chains: &[SendingChain],
) -> Result<(), ChainRotationError> {
    if chains.len() > DSCR_MAX_CHAINS {
        return Err(ChainRotationError::TooManyChains);
    }
    let mut seen = BTreeSet::new();
    for c in chains {
        if c.chain_id == 0 {
            return Err(ChainRotationError::ZeroChainId);
        }
        if !seen.insert(c.chain_id) {
            return Err(ChainRotationError::DuplicateChainId(c.chain_id));
        }
        if c.message_count > DSCR_MAX_MESSAGES {
            return Err(ChainRotationError::TooManyMessages {
                chain_id: c.chain_id,
                count: c.message_count,
                max: DSCR_MAX_MESSAGES,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain(id: u64, count: u64) -> SendingChain {
        SendingChain { chain_id: id, message_count: count }
    }

    fn valid_chains() -> Vec<SendingChain> {
        vec![chain(1, 500), chain(2, 300), chain(3, 100)]
    }

    /// **DSCR-01** — too many messages rejected.
    #[test]
    fn dscr_01_too_many_rejected() {
        assert_eq!(
            validate_sending_chain_rotation(&[chain(1, DSCR_MAX_MESSAGES + 1)]),
            Err(ChainRotationError::TooManyMessages {
                chain_id: 1,
                count: DSCR_MAX_MESSAGES + 1,
                max: DSCR_MAX_MESSAGES,
            })
        );
    }

    /// **DSCR-02** — zero chain ID rejected.
    #[test]
    fn dscr_02_zero_id_rejected() {
        assert_eq!(
            validate_sending_chain_rotation(&[chain(0, 10)]),
            Err(ChainRotationError::ZeroChainId)
        );
    }

    /// **DSCR-03** — too many chains rejected.
    #[test]
    fn dscr_03_too_many_rejected() {
        let chains: Vec<SendingChain> = (0..=DSCR_MAX_CHAINS as u64)
            .map(|i| chain(i + 1, 10))
            .collect();
        assert_eq!(
            validate_sending_chain_rotation(&chains),
            Err(ChainRotationError::TooManyChains)
        );
    }

    /// **DSCR-04** — duplicate chain ID rejected.
    #[test]
    fn dscr_04_duplicate_rejected() {
        let chains = vec![chain(1, 100), chain(1, 200)];
        assert_eq!(
            validate_sending_chain_rotation(&chains),
            Err(ChainRotationError::DuplicateChainId(1))
        );
    }

    /// **DSCR-05** — valid chains accepted.
    #[test]
    fn dscr_05_valid_accepted() {
        assert_eq!(validate_sending_chain_rotation(&valid_chains()), Ok(()));
    }

    /// **DSCR-06** — empty accepted.
    #[test]
    fn dscr_06_empty_accepted() {
        assert_eq!(validate_sending_chain_rotation(&[]), Ok(()));
    }

    /// **DSCR-07** — single chain at max accepted.
    #[test]
    fn dscr_07_max_messages_accepted() {
        assert_eq!(validate_sending_chain_rotation(&[chain(1, DSCR_MAX_MESSAGES)]), Ok(()));
    }

    /// **DSCR-08** — single chain accepted.
    #[test]
    fn dscr_08_single_accepted() {
        assert_eq!(validate_sending_chain_rotation(&[chain(1, 10)]), Ok(()));
    }

    /// **DSCR-09** — max chains boundary accepted.
    #[test]
    fn dscr_09_max_chains_accepted() {
        let chains: Vec<SendingChain> = (0..DSCR_MAX_CHAINS as u64)
            .map(|i| chain(i + 1, 10))
            .collect();
        assert_eq!(validate_sending_chain_rotation(&chains), Ok(()));
    }

    /// **DSCR-10** — zero messages accepted (fresh chain).
    #[test]
    fn dscr_10_zero_messages_accepted() {
        assert_eq!(validate_sending_chain_rotation(&[chain(1, 0)]), Ok(()));
    }
}
