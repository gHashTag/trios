//! # CR-CHAT-01 — Epoch advancement guard (Wave-46 Lane A)
//!
//! R-CHAT-2 — Epoch lifecycle monotonicity.
//!
//! MLS groups advance an epoch counter on every membership change, key
//! update, or external commit. An adversary who can inject or reorder
//! Commit messages can:
//!
//! * **Roll back** an epoch to reuse old keys for decryption.
//! * **Skip** epochs to force key material loss.
//! * **Fork** the group by presenting different epoch numbers to different
//!   members.
//!
//! trios-chat enforces **6 rules** per RFC 9420 §12.1:
//!
//! 1. Epoch must advance (new > current).
//! 2. Gap between consecutive epochs ≤ `EPOCH_MAX_GAP`.
//! 3. Epoch never exceeds `EPOCH_MAX_VALUE`.
//! 4. Commit must reference the correct prior epoch.
//! 5. No duplicate epoch numbers within a group.
//! 6. Epoch must be non-zero.
//!
//! Tests **EPAD-01..10**. Error enum [`EpochAdvanceError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · EPOCH-ADVANCE`

#![forbid(unsafe_code)]

/// Maximum allowed gap between consecutive epochs.
pub const EPOCH_MAX_GAP: u64 = 16;

/// Maximum epoch value.
pub const EPOCH_MAX_VALUE: u64 = (1u64 << 32) - 1;

/// Epoch advancement record.
#[derive(Debug, Clone)]
pub struct EpochTransition {
    /// Previous epoch number.
    pub prior: u64,
    /// Proposed new epoch number.
    pub next: u64,
}

/// All ways epoch advancement can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EpochAdvanceError {
    /// Epoch did not advance (next <= prior).
    NotAdvanced,
    /// Gap exceeds maximum.
    GapTooLarge,
    /// Epoch exceeds maximum value.
    ExceedsMaxValue,
    /// Prior epoch mismatch (commit references wrong prior).
    PriorMismatch,
    /// Duplicate epoch number.
    DuplicateEpoch,
    /// Zero epoch not allowed.
    ZeroEpoch,
}

/// `[VERIFIED]` Validate a single epoch transition against prior state.
pub fn validate_epoch_transition(
    current: u64,
    proposed: &EpochTransition,
) -> Result<(), EpochAdvanceError> {
    if proposed.next == 0 {
        return Err(EpochAdvanceError::ZeroEpoch);
    }
    if proposed.next > EPOCH_MAX_VALUE {
        return Err(EpochAdvanceError::ExceedsMaxValue);
    }
    if proposed.prior != current {
        return Err(EpochAdvanceError::PriorMismatch);
    }
    if proposed.next <= current {
        return Err(EpochAdvanceError::NotAdvanced);
    }
    let gap = proposed.next - current;
    if gap > EPOCH_MAX_GAP {
        return Err(EpochAdvanceError::GapTooLarge);
    }
    Ok(())
}

/// `[VERIFIED]` Validate a full epoch history chain. Each transition must
/// be valid and no duplicates allowed.
pub fn validate_epoch_chain(
    chain: &[u64],
) -> Result<(), EpochAdvanceError> {
    if chain.is_empty() {
        return Ok(());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut current = 0u64;
    for (i, &epoch) in chain.iter().enumerate() {
        if epoch == 0 {
            return Err(EpochAdvanceError::ZeroEpoch);
        }
        if epoch > EPOCH_MAX_VALUE {
            return Err(EpochAdvanceError::ExceedsMaxValue);
        }
        if !seen.insert(epoch) {
            return Err(EpochAdvanceError::DuplicateEpoch);
        }
        if i > 0 {
            if epoch <= current {
                return Err(EpochAdvanceError::NotAdvanced);
            }
            if epoch - current > EPOCH_MAX_GAP {
                return Err(EpochAdvanceError::GapTooLarge);
            }
        }
        current = epoch;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **EPAD-01** — zero epoch rejected.
    #[test]
    fn epad_01_zero_epoch_rejected() {
        let t = EpochTransition { prior: 0, next: 0 };
        assert_eq!(
            validate_epoch_transition(0, &t),
            Err(EpochAdvanceError::ZeroEpoch)
        );
    }

    /// **EPAD-02** — epoch not advanced rejected.
    #[test]
    fn epad_02_not_advanced_rejected() {
        let t = EpochTransition { prior: 5, next: 5 };
        assert_eq!(
            validate_epoch_transition(5, &t),
            Err(EpochAdvanceError::NotAdvanced)
        );
    }

    /// **EPAD-03** — epoch rollback rejected.
    #[test]
    fn epad_03_rollback_rejected() {
        let t = EpochTransition { prior: 10, next: 3 };
        assert_eq!(
            validate_epoch_transition(10, &t),
            Err(EpochAdvanceError::NotAdvanced)
        );
    }

    /// **EPAD-04** — gap too large rejected.
    #[test]
    fn epad_04_gap_too_large_rejected() {
        let t = EpochTransition { prior: 1, next: 1 + EPOCH_MAX_GAP + 1 };
        assert_eq!(
            validate_epoch_transition(1, &t),
            Err(EpochAdvanceError::GapTooLarge)
        );
    }

    /// **EPAD-05** — prior mismatch rejected.
    #[test]
    fn epad_05_prior_mismatch_rejected() {
        let t = EpochTransition { prior: 3, next: 4 };
        assert_eq!(
            validate_epoch_transition(5, &t),
            Err(EpochAdvanceError::PriorMismatch)
        );
    }

    /// **EPAD-06** — valid single transition accepted.
    #[test]
    fn epad_06_valid_transition_accepted() {
        let t = EpochTransition { prior: 1, next: 2 };
        assert_eq!(validate_epoch_transition(1, &t), Ok(()));
    }

    /// **EPAD-07** — exact max gap accepted.
    #[test]
    fn epad_07_exact_max_gap_accepted() {
        let t = EpochTransition { prior: 1, next: 1 + EPOCH_MAX_GAP };
        assert_eq!(validate_epoch_transition(1, &t), Ok(()));
    }

    /// **EPAD-08** — duplicate epoch in chain rejected.
    #[test]
    fn epad_08_duplicate_in_chain_rejected() {
        assert_eq!(
            validate_epoch_chain(&[1, 2, 3, 2]),
            Err(EpochAdvanceError::DuplicateEpoch)
        );
    }

    /// **EPAD-09** — valid chain accepted.
    #[test]
    fn epad_09_valid_chain_accepted() {
        assert_eq!(validate_epoch_chain(&[1, 2, 3, 4, 5]), Ok(()));
    }

    /// **EPAD-10** — empty chain accepted.
    #[test]
    fn epad_10_empty_chain_accepted() {
        assert_eq!(validate_epoch_chain(&[]), Ok(()));
    }
}
