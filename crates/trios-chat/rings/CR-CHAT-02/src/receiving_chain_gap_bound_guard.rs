//! # CR-CHAT-02 — Receiving chain gap bound guard (Wave-104 Lane A)
//!
//! RATCHET — receiving chain counter gaps must be bounded.
//!
//! In the Double Ratchet, out-of-order delivery causes gaps in the
//! receiving counter sequence. Each gap requires buffering a skipped
//! message key. Without bounds:
//!
//! * **Memory exhaustion** — an adversary sends message N followed by
//!   N+1000000, forcing the receiver to buffer 999999 skipped keys.
//! * **CPU amplification** — each skipped key derivation is a full
//!   HKDF operation; millions of skips burn CPU without delivering
//!   any usable message.
//! * **DoS via gap** — the receiver cannot process real messages
//!   until all gaps are resolved or the keys expire from the cache.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Gap size <= `RCGB_MAX_GAP`.
//! 2. Total gaps <= `RCGB_MAX_TOTAL_GAPS`.
//! 3. Counter must be > 0.
//! 4. Gaps must be reported in order.
//! 5. No duplicate counter ranges.
//! 6. Batch size <= `RCGB_MAX_BATCH`.
//!
//! Tests **RCGB-01..10**. Error enum [`GapBoundError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * GAP-BOUNDED`

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

/// Maximum single gap size.
pub const RCGB_MAX_GAP: u64 = 1024;

/// Maximum total gaps across all ranges.
pub const RCGB_MAX_TOTAL_GAPS: u64 = 8192;

/// Maximum gap records per batch.
pub const RCGB_MAX_BATCH: usize = 256;

/// A receiving chain gap record.
#[derive(Debug, Clone)]
pub struct ChainGap {
    /// Start of gap (exclusive — last received counter before gap).
    pub after: u64,
    /// End of gap (inclusive — counter of message that fills the gap).
    pub before: u64,
}

/// All ways gap bound validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GapBoundError {
    /// Single gap too large.
    GapTooLarge { idx: usize, gap: u64, max: u64 },
    /// Total gaps exceeded.
    TotalExceeded { total: u64, max: u64 },
    /// Zero counter.
    ZeroCounter(usize),
    /// Not ordered.
    NotOrdered { idx: usize, prev: u64, current: u64 },
    /// Duplicate range.
    DuplicateRange(usize),
    /// Batch too large.
    BatchTooLarge { got: usize, max: usize },
}

/// `[VERIFIED]` Validate receiving chain gap bounds.
pub fn validate_chain_gaps(gaps: &[ChainGap]) -> Result<(), GapBoundError> {
    if gaps.len() > RCGB_MAX_BATCH {
        return Err(GapBoundError::BatchTooLarge {
            got: gaps.len(),
            max: RCGB_MAX_BATCH,
        });
    }
    let mut seen: BTreeSet<(u64, u64)> = BTreeSet::new();
    let mut prev_end: u64 = 0;
    let mut total_gaps: u64 = 0;
    for (i, g) in gaps.iter().enumerate() {
        if g.after == 0 || g.before == 0 {
            return Err(GapBoundError::ZeroCounter(i));
        }
        let gap_size = g.before.saturating_sub(g.after);
        if gap_size > RCGB_MAX_GAP {
            return Err(GapBoundError::GapTooLarge {
                idx: i,
                gap: gap_size,
                max: RCGB_MAX_GAP,
            });
        }
        if !seen.insert((g.after, g.before)) {
            return Err(GapBoundError::DuplicateRange(i));
        }
        if i > 0 && g.after < prev_end {
            return Err(GapBoundError::NotOrdered {
                idx: i,
                prev: prev_end,
                current: g.after,
            });
        }
        total_gaps += gap_size;
        if total_gaps > RCGB_MAX_TOTAL_GAPS {
            return Err(GapBoundError::TotalExceeded {
                total: total_gaps,
                max: RCGB_MAX_TOTAL_GAPS,
            });
        }
        prev_end = g.before;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(after: u64, before: u64) -> ChainGap {
        ChainGap { after, before }
    }

    fn valid_gaps() -> Vec<ChainGap> {
        vec![
            gap(10, 15),
            gap(20, 25),
            gap(30, 35),
        ]
    }

    /// **RCGB-01** — gap too large rejected.
    #[test]
    fn rcgb_01_gap_too_large_rejected() {
        let gs = vec![gap(1, RCGB_MAX_GAP + 2)];
        assert_eq!(
            validate_chain_gaps(&gs),
            Err(GapBoundError::GapTooLarge {
                idx: 0,
                gap: RCGB_MAX_GAP + 1,
                max: RCGB_MAX_GAP,
            })
        );
    }

    /// **RCGB-02** — total exceeded rejected.
    #[test]
    fn rcgb_02_total_exceeded_rejected() {
        let gs: Vec<ChainGap> = (0..=RCGB_MAX_TOTAL_GAPS / 100)
            .map(|i| ChainGap { after: i * 200 + 1, before: i * 200 + 101 })
            .collect();
        assert!(matches!(
            validate_chain_gaps(&gs),
            Err(GapBoundError::TotalExceeded { .. })
        ));
    }

    /// **RCGB-03** — zero counter rejected.
    #[test]
    fn rcgb_03_zero_counter_rejected() {
        let g = ChainGap { after: 0, before: 10 };
        assert_eq!(
            validate_chain_gaps(&[g]),
            Err(GapBoundError::ZeroCounter(0))
        );
    }

    /// **RCGB-04** — not ordered rejected.
    #[test]
    fn rcgb_04_not_ordered_rejected() {
        let gs = vec![gap(50, 60), gap(30, 40)];
        assert_eq!(
            validate_chain_gaps(&gs),
            Err(GapBoundError::NotOrdered {
                idx: 1,
                prev: 60,
                current: 30,
            })
        );
    }

    /// **RCGB-05** — duplicate range rejected.
    #[test]
    fn rcgb_05_duplicate_rejected() {
        let gs = vec![gap(10, 20), gap(10, 20)];
        assert_eq!(
            validate_chain_gaps(&gs),
            Err(GapBoundError::DuplicateRange(1))
        );
    }

    /// **RCGB-06** — batch too large rejected.
    #[test]
    fn rcgb_06_batch_too_large_rejected() {
        let gs: Vec<ChainGap> = (0..=RCGB_MAX_BATCH)
            .map(|i| ChainGap { after: (i as u64) * 2000 + 1, before: (i as u64) * 2000 + 10 })
            .collect();
        assert_eq!(
            validate_chain_gaps(&gs),
            Err(GapBoundError::BatchTooLarge {
                got: RCGB_MAX_BATCH + 1,
                max: RCGB_MAX_BATCH,
            })
        );
    }

    /// **RCGB-07** — valid accepted.
    #[test]
    fn rcgb_07_valid_accepted() {
        assert_eq!(validate_chain_gaps(&valid_gaps()), Ok(()));
    }

    /// **RCGB-08** — empty accepted.
    #[test]
    fn rcgb_08_empty_accepted() {
        assert_eq!(validate_chain_gaps(&[]), Ok(()));
    }

    /// **RCGB-09** — single gap at max accepted.
    #[test]
    fn rcgb_09_max_gap_accepted() {
        let gs = vec![gap(1, RCGB_MAX_GAP + 1)];
        assert_eq!(validate_chain_gaps(&gs), Ok(()));
    }

    /// **RCGB-10** — zero before counter rejected.
    #[test]
    fn rcgb_10_zero_before_rejected() {
        let g = ChainGap { after: 10, before: 0 };
        assert_eq!(
            validate_chain_gaps(&[g]),
            Err(GapBoundError::ZeroCounter(0))
        );
    }
}
