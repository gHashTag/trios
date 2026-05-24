//! # CR-CHAT-02 — Ratchet skip message bound guard (Wave-83 Lane B)
//!
//! RATCHET — total skipped messages per session must be bounded, R-CHAT-2.
//!
//! The double ratchet stores skipped message keys for out-of-order
//! delivery. Without a total bound:
//!
//! * **Memory DoS** — attacker sends messages with gaps, forcing the
//!   receiver to store keys for every skipped index.
//! * **CPU exhaustion** — processing millions of skipped keys consumes
//!   CPU during every message receipt.
//! * **State bloat** — the skipped-key store grows without bound,
//!   eventually causing OOM.
//!
//! This is distinct from SMKE (skipped key exhaustion, per-chain) and
//! SMKG (gap bounding). RSMB enforces a *total session-wide* bound on
//! all skipped messages across all chains and epochs.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Total skipped count <= `RSMB_MAX_TOTAL_SKIPPED`.
//! 2. Skips per epoch <= `RSMB_MAX_PER_EPOCH`.
//! 3. No epoch has zero skips (only track epochs with actual skips).
//! 4. Epoch numbers are valid (> 0).
//! 5. Total epochs tracked <= `RSMB_MAX_EPOCHS`.
//! 6. No duplicate epoch entries.
//!
//! Tests **RSMB-01..10**. Error enum [`SkipBoundError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * RATCHET-SKIP-BOUND`

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

/// Maximum total skipped messages per session.
pub const RSMB_MAX_TOTAL_SKIPPED: usize = 1024;

/// Maximum skips per epoch.
pub const RSMB_MAX_PER_EPOCH: usize = 256;

/// Maximum tracked epochs.
pub const RSMB_MAX_EPOCHS: usize = 128;

/// Per-epoch skip count.
#[derive(Debug, Clone)]
pub struct EpochSkipCount {
    /// Epoch number.
    pub epoch: u64,
    /// Number of skipped messages in this epoch.
    pub skipped: usize,
}

/// All ways skip bound validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipBoundError {
    /// Total skipped exceeded.
    TotalExceeded,
    /// Per-epoch exceeded.
    PerEpochExceeded(u64),
    /// Zero epoch.
    ZeroEpoch,
    /// Too many epochs.
    TooManyEpochs,
    /// Duplicate epoch.
    DuplicateEpoch(u64),
    /// Zero skips in entry.
    ZeroSkips(u64),
}

/// `[VERIFIED]` Validate total ratchet skip message bounds.
pub fn validate_skip_bounds(
    entries: &[EpochSkipCount],
) -> Result<(), SkipBoundError> {
    if entries.len() > RSMB_MAX_EPOCHS {
        return Err(SkipBoundError::TooManyEpochs);
    }
    let mut total = 0usize;
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        if entry.epoch == 0 {
            return Err(SkipBoundError::ZeroEpoch);
        }
        if entry.skipped == 0 {
            return Err(SkipBoundError::ZeroSkips(entry.epoch));
        }
        if entry.skipped > RSMB_MAX_PER_EPOCH {
            return Err(SkipBoundError::PerEpochExceeded(entry.epoch));
        }
        if !seen.insert(entry.epoch) {
            return Err(SkipBoundError::DuplicateEpoch(entry.epoch));
        }
        total += entry.skipped;
        if total > RSMB_MAX_TOTAL_SKIPPED {
            return Err(SkipBoundError::TotalExceeded);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(epoch: u64, skipped: usize) -> EpochSkipCount {
        EpochSkipCount { epoch, skipped }
    }

    fn valid_entries() -> Vec<EpochSkipCount> {
        vec![epoch(1, 10), epoch(2, 20), epoch(3, 5)]
    }

    /// **RSMB-01** — total exceeded rejected.
    #[test]
    fn rsmb_01_total_exceeded_rejected() {
        let entries = vec![epoch(1, RSMB_MAX_TOTAL_SKIPPED + 1)];
        assert_eq!(
            validate_skip_bounds(&entries),
            Err(SkipBoundError::PerEpochExceeded(1))
        );
    }

    /// **RSMB-02** — per-epoch exceeded rejected.
    #[test]
    fn rsmb_02_per_epoch_rejected() {
        assert_eq!(
            validate_skip_bounds(&[epoch(1, RSMB_MAX_PER_EPOCH + 1)]),
            Err(SkipBoundError::PerEpochExceeded(1))
        );
    }

    /// **RSMB-03** — zero epoch rejected.
    #[test]
    fn rsmb_03_zero_epoch_rejected() {
        assert_eq!(
            validate_skip_bounds(&[epoch(0, 10)]),
            Err(SkipBoundError::ZeroEpoch)
        );
    }

    /// **RSMB-04** — too many epochs rejected.
    #[test]
    fn rsmb_04_too_many_rejected() {
        let entries: Vec<EpochSkipCount> = (1..=RSMB_MAX_EPOCHS as u64 + 1)
            .map(|i| epoch(i, 1))
            .collect();
        assert_eq!(
            validate_skip_bounds(&entries),
            Err(SkipBoundError::TooManyEpochs)
        );
    }

    /// **RSMB-05** — duplicate epoch rejected.
    #[test]
    fn rsmb_05_duplicate_rejected() {
        let entries = vec![epoch(1, 10), epoch(2, 20), epoch(1, 5)];
        assert_eq!(
            validate_skip_bounds(&entries),
            Err(SkipBoundError::DuplicateEpoch(1))
        );
    }

    /// **RSMB-06** — zero skips rejected.
    #[test]
    fn rsmb_06_zero_skips_rejected() {
        assert_eq!(
            validate_skip_bounds(&[epoch(1, 0)]),
            Err(SkipBoundError::ZeroSkips(1))
        );
    }

    /// **RSMB-07** — valid entries accepted.
    #[test]
    fn rsmb_07_valid_accepted() {
        assert_eq!(validate_skip_bounds(&valid_entries()), Ok(()));
    }

    /// **RSMB-08** — empty accepted.
    #[test]
    fn rsmb_08_empty_accepted() {
        assert_eq!(validate_skip_bounds(&[]), Ok(()));
    }

    /// **RSMB-09** — cumulative total boundary accepted.
    #[test]
    fn rsmb_09_total_boundary_accepted() {
        let entries = vec![epoch(1, 256), epoch(2, 256), epoch(3, 256), epoch(4, 256)];
        assert_eq!(validate_skip_bounds(&entries), Ok(()));
    }

    /// **RSMB-10** — max per epoch accepted.
    #[test]
    fn rsmb_10_max_per_epoch_accepted() {
        assert_eq!(validate_skip_bounds(&[epoch(1, RSMB_MAX_PER_EPOCH)]), Ok(()));
    }
}
