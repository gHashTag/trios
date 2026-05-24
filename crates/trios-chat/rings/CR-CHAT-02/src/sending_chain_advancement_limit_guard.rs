//! # CR-CHAT-02 — Sending chain advancement limit guard (Wave-100 Lane B)
//!
//! RATCHET — sending chain must not advance beyond a maximum number
//! of steps without a new DH ratchet step.
//!
//! In the Double Ratchet, each DH output seeds a new sending chain.
//! The chain key is advanced with each message sent. If the chain
//! advances too far without a DH ratchet step:
//!
//! * **Forward secrecy weakened** — if the chain key is compromised,
//!   all future messages on that chain are exposed. More steps = more
//!   exposure.
//! * **Key wear-out** — deriving too many keys from a single chain
//!   root increases the probability of key collision.
//! * **State recovery** — an adversary who recovers state at step N
//!   can compute all steps > N until the next DH ratchet.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Steps since last DH <= `SCAL_MAX_STEPS`.
//! 2. Steps must be monotonically increasing.
//! 3. Total chain advances <= `SCAL_MAX_ADVANCES`.
//! 4. Chain ID must not be zero.
//! 5. No duplicate chain records.
//! 6. Step count > 0 (zero-step records are invalid).
//!
//! Tests **SCAL-01..10**. Error enum [`ChainAdvanceError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * CHAIN-LIMIT`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum steps since last DH ratchet.
pub const SCAL_MAX_STEPS: u64 = 1000;

/// Maximum total advances across all chains.
pub const SCAL_MAX_ADVANCES: usize = 10_000;

/// Chain ID length.
pub const SCAL_CHAIN_ID_LEN: usize = 16;

/// A sending chain advancement record.
#[derive(Debug, Clone)]
pub struct ChainAdvance {
    /// Chain identifier.
    pub chain_id: [u8; SCAL_CHAIN_ID_LEN],
    /// Number of steps since last DH ratchet.
    pub steps_since_dh: u64,
}

/// All ways chain advancement validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainAdvanceError {
    /// Steps exceed maximum.
    TooManySteps { idx: usize, steps: u64, max: u64 },
    /// Not monotonically increasing.
    NotMonotonic { idx: usize, prev: u64, current: u64 },
    /// Too many advances.
    TooManyAdvances { got: usize, max: usize },
    /// Zero chain ID.
    ZeroChainId(usize),
    /// Duplicate chain record.
    DuplicateChain(usize),
    /// Zero steps.
    ZeroSteps(usize),
}

/// `[VERIFIED]` Validate sending chain advancement limits.
pub fn validate_chain_advances(
    advances: &[ChainAdvance],
) -> Result<(), ChainAdvanceError> {
    if advances.len() > SCAL_MAX_ADVANCES {
        return Err(ChainAdvanceError::TooManyAdvances {
            got: advances.len(),
            max: SCAL_MAX_ADVANCES,
        });
    }
    let mut seen: BTreeSet<[u8; SCAL_CHAIN_ID_LEN]> = BTreeSet::new();
    let mut prev_steps: u64 = 0;
    for (i, a) in advances.iter().enumerate() {
        if a.chain_id == [0u8; SCAL_CHAIN_ID_LEN] {
            return Err(ChainAdvanceError::ZeroChainId(i));
        }
        if a.steps_since_dh == 0 {
            return Err(ChainAdvanceError::ZeroSteps(i));
        }
        if a.steps_since_dh > SCAL_MAX_STEPS {
            return Err(ChainAdvanceError::TooManySteps {
                idx: i,
                steps: a.steps_since_dh,
                max: SCAL_MAX_STEPS,
            });
        }
        if i > 0 && a.steps_since_dh <= prev_steps {
            return Err(ChainAdvanceError::NotMonotonic {
                idx: i,
                prev: prev_steps,
                current: a.steps_since_dh,
            });
        }
        if !seen.insert(a.chain_id) {
            return Err(ChainAdvanceError::DuplicateChain(i));
        }
        prev_steps = a.steps_since_dh;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(byte: u8) -> [u8; SCAL_CHAIN_ID_LEN] {
        [byte; SCAL_CHAIN_ID_LEN]
    }

    fn advance(chain_byte: u8, steps: u64) -> ChainAdvance {
        ChainAdvance { chain_id: cid(chain_byte), steps_since_dh: steps }
    }

    fn valid_advances() -> Vec<ChainAdvance> {
        vec![
            advance(0x01, 10),
            advance(0x02, 50),
            advance(0x03, 100),
        ]
    }

    /// **SCAL-01** — too many steps rejected.
    #[test]
    fn scal_01_too_many_steps_rejected() {
        let ads = vec![advance(0x01, SCAL_MAX_STEPS + 1)];
        assert_eq!(
            validate_chain_advances(&ads),
            Err(ChainAdvanceError::TooManySteps {
                idx: 0,
                steps: SCAL_MAX_STEPS + 1,
                max: SCAL_MAX_STEPS,
            })
        );
    }

    /// **SCAL-02** — not monotonic rejected.
    #[test]
    fn scal_02_not_monotonic_rejected() {
        let ads = vec![advance(0x01, 50), advance(0x02, 30)];
        assert_eq!(
            validate_chain_advances(&ads),
            Err(ChainAdvanceError::NotMonotonic {
                idx: 1,
                prev: 50,
                current: 30,
            })
        );
    }

    /// **SCAL-03** — too many advances rejected.
    #[test]
    fn scal_03_too_many_rejected() {
        let ads: Vec<ChainAdvance> = (0..=SCAL_MAX_ADVANCES)
            .map(|i| {
                let b = (i % 254 + 1) as u8;
                ChainAdvance { chain_id: cid(b), steps_since_dh: (i as u64) + 1 }
            })
            .collect();
        assert!(matches!(
            validate_chain_advances(&ads),
            Err(ChainAdvanceError::TooManyAdvances { .. })
        ));
    }

    /// **SCAL-04** — zero chain ID rejected.
    #[test]
    fn scal_04_zero_chain_rejected() {
        let a = ChainAdvance { chain_id: [0u8; SCAL_CHAIN_ID_LEN], steps_since_dh: 5 };
        assert_eq!(
            validate_chain_advances(&[a]),
            Err(ChainAdvanceError::ZeroChainId(0))
        );
    }

    /// **SCAL-05** — duplicate chain rejected.
    #[test]
    fn scal_05_duplicate_rejected() {
        let ads = vec![advance(0x01, 10), advance(0x01, 20)];
        assert_eq!(
            validate_chain_advances(&ads),
            Err(ChainAdvanceError::DuplicateChain(1))
        );
    }

    /// **SCAL-06** — zero steps rejected.
    #[test]
    fn scal_06_zero_steps_rejected() {
        let a = ChainAdvance { chain_id: cid(0x01), steps_since_dh: 0 };
        assert_eq!(
            validate_chain_advances(&[a]),
            Err(ChainAdvanceError::ZeroSteps(0))
        );
    }

    /// **SCAL-07** — valid accepted.
    #[test]
    fn scal_07_valid_accepted() {
        assert_eq!(validate_chain_advances(&valid_advances()), Ok(()));
    }

    /// **SCAL-08** — empty accepted.
    #[test]
    fn scal_08_empty_accepted() {
        assert_eq!(validate_chain_advances(&[]), Ok(()));
    }

    /// **SCAL-09** — single at max steps accepted.
    #[test]
    fn scal_09_max_steps_accepted() {
        let ads = vec![advance(0x01, SCAL_MAX_STEPS)];
        assert_eq!(validate_chain_advances(&ads), Ok(()));
    }

    /// **SCAL-10** — monotonic equal rejected (must be strictly increasing).
    #[test]
    fn scal_10_equal_steps_rejected() {
        let ads = vec![advance(0x01, 50), advance(0x02, 50)];
        assert_eq!(
            validate_chain_advances(&ads),
            Err(ChainAdvanceError::NotMonotonic {
                idx: 1,
                prev: 50,
                current: 50,
            })
        );
    }
}
