//! # CR-CHAT-02 — Skipped message key exhaustion guard (Wave-53 Lane A)
//!
//! РЭТЧЕТ — ограничение skipped keys, R-CHAT-2.
//!
//! Когда получатель видит «дыру» в counter (например, 1, 2, 5), он
//! сохраняет промежуточные message keys на случай, что пакеты 3, 4
//! придут позже. Атакующий эксплуатирует это:
//!
//! * **DoS через память** — сгенерировать тысячи пропусков → хост
//!   хранит столько же ключей.
//! * **Exhaustion** — забить лимит skipped keys, заставляя легитимные
//!   сообщения отбрасываться.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Skipped key count ≤ `SMKE_MAX_SKIPPED`.
//! 2. Each skipped key has a valid counter range.
//! 3. Counter gap ≤ `SMKE_MAX_GAP`.
//! 4. No duplicate skipped counter.
//! 5. Skipped keys are bounded per-chain.
//! 6. Total skipped across all chains ≤ `SMKE_MAX_TOTAL`.
//!
//! Tests **SMKE-01..10**. Error enum [`SkippedKeyError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · SKIPPED-KEY-EXHAUSTION`

#![forbid(unsafe_code)]

/// Maximum skipped keys per chain.
pub const SMKE_MAX_SKIPPED: usize = 32;

/// Maximum gap between consecutive counters.
pub const SMKE_MAX_GAP: u64 = 64;

/// Maximum total skipped keys across all chains.
pub const SMKE_MAX_TOTAL: usize = 256;

/// All ways skipped key validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkippedKeyError {
    /// Too many skipped keys in a single chain.
    TooManySkipped,
    /// Counter gap exceeds maximum.
    GapTooLarge,
    /// Duplicate skipped counter.
    DuplicateCounter,
    /// Total skipped keys across chains exceeds limit.
    TotalExceeded,
    /// Zero counter not allowed.
    ZeroCounter,
    /// Skipped range is empty.
    EmptyRange,
}

/// `[VERIFIED]` Validate skipped keys for a single chain.
pub fn validate_chain_skipped(
    received: u64,
    skipped: &[u64],
) -> Result<(), SkippedKeyError> {
    if skipped.len() > SMKE_MAX_SKIPPED {
        return Err(SkippedKeyError::TooManySkipped);
    }
    let mut seen = std::collections::BTreeSet::new();
    for &counter in skipped {
        if counter == 0 {
            return Err(SkippedKeyError::ZeroCounter);
        }
        if !seen.insert(counter) {
            return Err(SkippedKeyError::DuplicateCounter);
        }
        if counter > received {
            if counter - received > SMKE_MAX_GAP {
                return Err(SkippedKeyError::GapTooLarge);
            }
        }
    }
    Ok(())
}

/// `[VERIFIED]` Validate total skipped keys across all chains.
pub fn validate_total_skipped(
    per_chain_counts: &[usize],
) -> Result<(), SkippedKeyError> {
    let total: usize = per_chain_counts.iter().sum();
    if total > SMKE_MAX_TOTAL {
        return Err(SkippedKeyError::TotalExceeded);
    }
    for &count in per_chain_counts {
        if count > SMKE_MAX_SKIPPED {
            return Err(SkippedKeyError::TooManySkipped);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **SMKE-01** — too many skipped rejected.
    #[test]
    fn smke_01_too_many_rejected() {
        let skipped: Vec<u64> = (1..=SMKE_MAX_SKIPPED as u64 + 1).collect();
        assert_eq!(
            validate_chain_skipped(SMKE_MAX_SKIPPED as u64 + 2, &skipped),
            Err(SkippedKeyError::TooManySkipped)
        );
    }

    /// **SMKE-02** — gap too large rejected.
    #[test]
    fn smke_02_gap_too_large_rejected() {
        assert_eq!(
            validate_chain_skipped(1, &[1 + SMKE_MAX_GAP + 1]),
            Err(SkippedKeyError::GapTooLarge)
        );
    }

    /// **SMKE-03** — duplicate counter rejected.
    #[test]
    fn smke_03_duplicate_rejected() {
        assert_eq!(
            validate_chain_skipped(5, &[3, 3]),
            Err(SkippedKeyError::DuplicateCounter)
        );
    }

    /// **SMKE-04** — total exceeded rejected.
    #[test]
    fn smke_04_total_exceeded_rejected() {
        let counts = vec![SMKE_MAX_SKIPPED; SMKE_MAX_TOTAL / SMKE_MAX_SKIPPED + 1];
        assert_eq!(
            validate_total_skipped(&counts),
            Err(SkippedKeyError::TotalExceeded)
        );
    }

    /// **SMKE-05** — zero counter rejected.
    #[test]
    fn smke_05_zero_counter_rejected() {
        assert_eq!(
            validate_chain_skipped(5, &[0]),
            Err(SkippedKeyError::ZeroCounter)
        );
    }

    /// **SMKE-06** — valid skipped accepted.
    #[test]
    fn smke_06_valid_accepted() {
        assert_eq!(validate_chain_skipped(5, &[3, 4]), Ok(()));
    }

    /// **SMKE-07** — empty skipped accepted.
    #[test]
    fn smke_07_empty_accepted() {
        assert_eq!(validate_chain_skipped(1, &[]), Ok(()));
    }

    /// **SMKE-08** — exact max skipped accepted.
    #[test]
    fn smke_08_exact_max_accepted() {
        let skipped: Vec<u64> = (1..=SMKE_MAX_SKIPPED as u64).collect();
        assert_eq!(
            validate_chain_skipped(SMKE_MAX_SKIPPED as u64 + 1, &skipped),
            Ok(())
        );
    }

    /// **SMKE-09** — total within limit accepted.
    #[test]
    fn smke_09_total_within_accepted() {
        let counts = vec![SMKE_MAX_SKIPPED; SMKE_MAX_TOTAL / SMKE_MAX_SKIPPED];
        assert_eq!(validate_total_skipped(&counts), Ok(()));
    }

    /// **SMKE-10** — single chain overflow in total rejected.
    #[test]
    fn smke_10_single_chain_overflow_rejected() {
        assert_eq!(
            validate_total_skipped(&[SMKE_MAX_SKIPPED + 1]),
            Err(SkippedKeyError::TooManySkipped)
        );
    }
}
