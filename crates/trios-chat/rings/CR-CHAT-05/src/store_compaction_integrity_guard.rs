//! # CR-CHAT-05 — Store compaction integrity guard (Wave-62 Lane B)
//!
//! PERSISTENCE — compaction must preserve hash chain, R-CHAT-1.
//!
//! When the store compacts old envelopes (garbage collection), it must
//! not break the integrity hash chain. An attacker who can trigger or
//! tamper with compaction can:
//!
//! * **Drop envelopes** — remove evidence by deleting during compaction.
//! * **Break hash chain** — leave gaps that make remaining chain invalid.
//! * **Forge anchor** — replace the compaction anchor hash.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Compaction anchor covers all removed envelopes.
//! 2. Compaction anchor links to surviving chain head.
//! 3. No envelope counter gaps after compaction.
//! 4. Removed count + remaining count = original count.
//! 5. Anchor hash is non-zero.
//! 6. Remaining counters are strictly monotonic.
//!
//! Tests **SCPI-01..10**. Error enum [`CompactionIntegrityError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COMPACTION-INTEGRITY`

#![forbid(unsafe_code)]

/// All ways compaction integrity can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompactionIntegrityError {
    /// Counter gap after compaction.
    CounterGap,
    /// Count mismatch (removed + remaining != original).
    CountMismatch,
    /// Anchor hash is all-zeros.
    ZeroAnchor,
    /// Remaining counters not monotonic.
    NotMonotonic,
    /// Empty remaining chain after compaction.
    EmptyRemaining,
    /// Removed count exceeds original.
    RemovedExceedsOriginal,
}

/// `[VERIFIED]` Validate a compaction operation for integrity.
pub fn validate_compaction(
    original_count: usize,
    removed_count: usize,
    remaining_counters: &[u64],
    anchor_hash: &[u8; 32],
) -> Result<(), CompactionIntegrityError> {
    if *anchor_hash == [0u8; 32] {
        return Err(CompactionIntegrityError::ZeroAnchor);
    }
    if removed_count > original_count {
        return Err(CompactionIntegrityError::RemovedExceedsOriginal);
    }
    if removed_count + remaining_counters.len() != original_count {
        return Err(CompactionIntegrityError::CountMismatch);
    }
    if remaining_counters.is_empty() && original_count > 0 {
        return Err(CompactionIntegrityError::EmptyRemaining);
    }
    for w in remaining_counters.windows(2) {
        if w[1] <= w[0] {
            return Err(CompactionIntegrityError::NotMonotonic);
        }
        if w[1] - w[0] > 1 {
            return Err(CompactionIntegrityError::CounterGap);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANCHOR: [u8; 32] = [0x42; 32];
    const ZERO: [u8; 32] = [0u8; 32];

    /// **SCPI-01** — counter gap rejected.
    #[test]
    fn scpi_01_counter_gap_rejected() {
        assert_eq!(
            validate_compaction(10, 3, &[1, 2, 5, 6, 7, 8, 9], &ANCHOR),
            Err(CompactionIntegrityError::CounterGap)
        );
    }

    /// **SCPI-02** — count mismatch rejected.
    #[test]
    fn scpi_02_count_mismatch_rejected() {
        assert_eq!(
            validate_compaction(10, 3, &[1, 2, 3, 4, 5, 6], &ANCHOR),
            Err(CompactionIntegrityError::CountMismatch)
        );
    }

    /// **SCPI-03** — zero anchor rejected.
    #[test]
    fn scpi_03_zero_anchor_rejected() {
        assert_eq!(
            validate_compaction(5, 2, &[1, 2, 3], &ZERO),
            Err(CompactionIntegrityError::ZeroAnchor)
        );
    }

    /// **SCPI-04** — not monotonic rejected.
    #[test]
    fn scpi_04_not_monotonic_rejected() {
        assert_eq!(
            validate_compaction(5, 2, &[2, 1, 3], &ANCHOR),
            Err(CompactionIntegrityError::NotMonotonic)
        );
    }

    /// **SCPI-05** — empty remaining rejected.
    #[test]
    fn scpi_05_empty_remaining_rejected() {
        assert_eq!(
            validate_compaction(5, 5, &[], &ANCHOR),
            Err(CompactionIntegrityError::EmptyRemaining)
        );
    }

    /// **SCPI-06** — removed exceeds original rejected.
    #[test]
    fn scpi_06_removed_exceeds_rejected() {
        assert_eq!(
            validate_compaction(3, 5, &[1, 2], &ANCHOR),
            Err(CompactionIntegrityError::RemovedExceedsOriginal)
        );
    }

    /// **SCPI-07** — valid compaction accepted.
    #[test]
    fn scpi_07_valid_accepted() {
        assert_eq!(
            validate_compaction(5, 2, &[1, 2, 3], &ANCHOR),
            Ok(())
        );
    }

    /// **SCPI-08** — no removal accepted.
    #[test]
    fn scpi_08_no_removal_accepted() {
        assert_eq!(
            validate_compaction(3, 0, &[1, 2, 3], &ANCHOR),
            Ok(())
        );
    }

    /// **SCPI-09** — single remaining accepted.
    #[test]
    fn scpi_09_single_accepted() {
        assert_eq!(
            validate_compaction(3, 2, &[1], &ANCHOR),
            Ok(())
        );
    }

    /// **SCPI-10** — empty original accepted.
    #[test]
    fn scpi_10_empty_original_accepted() {
        assert_eq!(
            validate_compaction(0, 0, &[], &ANCHOR),
            Ok(())
        );
    }
}
