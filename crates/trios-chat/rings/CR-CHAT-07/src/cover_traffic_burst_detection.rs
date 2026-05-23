//! # CR-CHAT-07 — Cover-traffic burst pattern detection (Wave-41 Lane B)
//!
//! R-CHAT-10 — Wire-timing uniformity enforcement.
//!
//! Even with fixed-size padding (CR-CHAT-04) and canonical gap quantisation,
//! an adversary observing wire timing can detect **bursts** — clusters of
//! emissions closer together than the minimum canonical gap. A burst reveals
//! that the user is actively typing (real messages queued rapidly), while
//! silence reveals idle periods. Together these break the "always online,
//! indistinguishable" cover traffic promise.
//!
//! trios-chat enforces **6 rules** on a sliding window of emissions:
//!
//! 1. No two consecutive emissions are closer than `MIN_GAP_MS`.
//! 2. No burst of ≥ `MAX_BURST_LEN` emissions within `BURST_WINDOW_MS`.
//! 3. Silence gap (no emissions) must not exceed `MAX_SILENCE_MS`.
//! 4. Cover-to-real ratio in any window is within `[MIN_COVER_RATIO, 1.0]`.
//! 5. Total emissions in any `BURST_WINDOW_MS` window ≤ `MAX_EMISSIONS`.
//! 6. Every emission gap is a member of `CANONICAL_GAPS_MS`.
//!
//! Tests **BURST-01..10**. Error enum [`BurstError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · COVER-BURST`

#![forbid(unsafe_code)]

/// Minimum gap between consecutive emissions (ms).
pub const BURST_MIN_GAP_MS: u64 = 1_000;

/// Maximum emissions in a burst window before flagging.
pub const BURST_MAX_EMISSIONS: usize = 10;

/// Burst window duration (ms).
pub const BURST_WINDOW_MS: u64 = 30_000;

/// Maximum silence gap allowed (ms).
pub const BURST_MAX_SILENCE_MS: u64 = 300_000;

/// Minimum cover-to-total ratio (numerator).
pub const BURST_MIN_COVER_NUM: u64 = 1;

/// Minimum cover-to-total ratio (denominator).
pub const BURST_MIN_COVER_DEN: u64 = 3;

/// One emission recorded on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionKind {
    /// Real chat message.
    Real,
    /// Cover traffic decoy.
    Cover,
}

/// One recorded emission with its timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmissionRecord {
    /// Timestamp in milliseconds since session start.
    pub timestamp_ms: u64,
    /// Kind of emission.
    pub kind: EmissionKind,
}

/// All ways a burst pattern can be detected.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BurstError {
    /// Two consecutive emissions are too close.
    GapBelowMinimum,
    /// Too many emissions within the burst window.
    TooManyEmissions,
    /// Silence gap exceeds maximum allowed.
    SilenceTooLong,
    /// Cover-to-real ratio too low in window.
    CoverRatioTooLow,
    /// Emission gap is not a canonical value.
    NonCanonicalGap,
}

/// `[VERIFIED]` Validate a sequence of emission records against burst
/// pattern detection rules. Returns `Ok(())` if all rules pass.
///
/// Rules enforced in fixed order:
///
/// 1. Every consecutive gap ≥ `BURST_MIN_GAP_MS`.
/// 2. Emissions in any `BURST_WINDOW_MS` window ≤ `BURST_MAX_EMISSIONS`.
/// 3. No silence gap > `BURST_MAX_SILENCE_MS`.
/// 4. Cover ratio ≥ `BURST_MIN_COVER_NUM / BURST_MIN_COVER_DEN` in any
///    window of `BURST_WINDOW_MS`.
/// 5. Every gap is in `CANONICAL_GAPS_MS` (after quantisation).
pub fn validate_burst_pattern(records: &[EmissionRecord]) -> Result<(), BurstError> {
    if records.len() < 2 {
        return Ok(());
    }
    let canonical_gaps: [u64; 4] = super::CANONICAL_GAPS_MS;
    for i in 1..records.len() {
        let gap = records[i].timestamp_ms.saturating_sub(records[i - 1].timestamp_ms);
        if gap < BURST_MIN_GAP_MS {
            return Err(BurstError::GapBelowMinimum);
        }
        let is_canonical = canonical_gaps.contains(&gap)
            || (gap > *canonical_gaps.last().unwrap());
        if !is_canonical {
            return Err(BurstError::NonCanonicalGap);
        }
        if gap > BURST_MAX_SILENCE_MS {
            return Err(BurstError::SilenceTooLong);
        }
    }
    for i in 0..records.len() {
        let window_end = records[i].timestamp_ms + BURST_WINDOW_MS;
        let window_records: Vec<&EmissionRecord> = records[i..]
            .iter()
            .take_while(|r| r.timestamp_ms <= window_end)
            .collect();
        if window_records.len() > BURST_MAX_EMISSIONS {
            return Err(BurstError::TooManyEmissions);
        }
        if window_records.len() < 2 {
            continue;
        }
        let covers = window_records.iter().filter(|r| r.kind == EmissionKind::Cover).count();
        let total = window_records.len();
        if total >= 2 {
            let ratio_num = covers as u64 * BURST_MIN_COVER_DEN;
            let ratio_den = total as u64 * BURST_MIN_COVER_NUM;
            if ratio_num < ratio_den {
                return Err(BurstError::CoverRatioTooLow);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(ts: u64, kind: EmissionKind) -> EmissionRecord {
        EmissionRecord { timestamp_ms: ts, kind }
    }

    /// **BURST-01** — gap below minimum (500ms) rejected.
    #[test]
    fn burst_01_gap_below_minimum_rejected() {
        let records = vec![
            rec(0, EmissionKind::Real),
            rec(500, EmissionKind::Cover),
        ];
        assert_eq!(
            validate_burst_pattern(&records),
            Err(BurstError::GapBelowMinimum)
        );
    }

    /// **BURST-02** — silence gap too long (400s > 300s max) rejected.
    #[test]
    fn burst_02_silence_too_long_rejected() {
        let records = vec![
            rec(0, EmissionKind::Real),
            rec(400_000, EmissionKind::Cover),
        ];
        assert_eq!(
            validate_burst_pattern(&records),
            Err(BurstError::SilenceTooLong)
        );
    }

    /// **BURST-03** — too many emissions in burst window rejected.
    #[test]
    fn burst_03_too_many_emissions_rejected() {
        let mut records = Vec::new();
        for i in 0..15u64 {
            records.push(rec(i * 1_000, if i % 2 == 0 { EmissionKind::Real } else { EmissionKind::Cover }));
        }
        assert_eq!(
            validate_burst_pattern(&records),
            Err(BurstError::TooManyEmissions)
        );
    }

    /// **BURST-04** — non-canonical gap (1500ms) rejected.
    #[test]
    fn burst_04_non_canonical_gap_rejected() {
        let records = vec![
            rec(0, EmissionKind::Real),
            rec(1_500, EmissionKind::Cover),
        ];
        assert_eq!(
            validate_burst_pattern(&records),
            Err(BurstError::NonCanonicalGap)
        );
    }

    /// **BURST-05** — cover ratio too low rejected.
    #[test]
    fn burst_05_cover_ratio_too_low_rejected() {
        let records = vec![
            rec(0, EmissionKind::Real),
            rec(5_000, EmissionKind::Real),
            rec(10_000, EmissionKind::Real),
        ];
        assert_eq!(
            validate_burst_pattern(&records),
            Err(BurstError::CoverRatioTooLow)
        );
    }

    /// **BURST-06** — single record always accepted.
    #[test]
    fn burst_06_single_record_accepted() {
        let records = vec![rec(0, EmissionKind::Real)];
        assert_eq!(validate_burst_pattern(&records), Ok(()));
    }

    /// **BURST-07** — empty records accepted.
    #[test]
    fn burst_07_empty_records_accepted() {
        assert_eq!(validate_burst_pattern(&[]), Ok(()));
    }

    /// **BURST-08** — valid alternating pattern accepted.
    #[test]
    fn burst_08_valid_alternating_accepted() {
        let records = vec![
            rec(0, EmissionKind::Cover),
            rec(5_000, EmissionKind::Real),
            rec(10_000, EmissionKind::Cover),
            rec(15_000, EmissionKind::Real),
        ];
        assert_eq!(validate_burst_pattern(&records), Ok(()));
    }

    /// **BURST-09** — largest canonical gap (300s) accepted.
    #[test]
    fn burst_09_largest_canonical_gap_accepted() {
        let records = vec![
            rec(0, EmissionKind::Real),
            rec(300_000, EmissionKind::Cover),
        ];
        assert_eq!(validate_burst_pattern(&records), Ok(()));
    }

    /// **BURST-10** — exact minimum gap (1s) accepted.
    #[test]
    fn burst_10_exact_min_gap_accepted() {
        let records = vec![
            rec(0, EmissionKind::Cover),
            rec(1_000, EmissionKind::Real),
        ];
        assert_eq!(validate_burst_pattern(&records), Ok(()));
    }
}
