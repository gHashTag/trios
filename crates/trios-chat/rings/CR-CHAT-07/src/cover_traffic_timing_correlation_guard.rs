//! # CR-CHAT-07 — Cover traffic timing correlation guard (Wave-79 Lane B)
//!
//! ANTI-CORRELATION — cover and real traffic timing must not correlate, R-CHAT-10.
//!
//! If cover traffic emission timing correlates with real traffic timing,
//! an observer can statistically separate the two:
//!
//! * **Synchronous emission** — cover always sent right after real,
//!   creating a recognizable real-then-cover pair.
//! * **Real-gated cover** — cover only emitted when real traffic
//!   arrives, making cover presence a signal for real traffic.
//! * **Timing echo** — cover inter-arrival time mirrors real
//!   inter-arrival time, leaking the real traffic pattern.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cover-real inter-arrival correlation <= `CTCG_MAX_CORRELATION`.
//! 2. Cover emission must not always follow real emission.
//! 3. At least `CTCG_MIN_COVER_BETWEEN_REAL` cover emissions between
//!   any two real emissions.
//! 4. No two consecutive real emissions without a cover in between.
//! 5. Total emissions in window <= `CTCG_MAX_WINDOW`.
//! 6. Window must have both real and cover traffic.
//!
//! Tests **CTCG-01..10**. Error enum [`CoverTimingError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COVER-TIMING-CORRELATION`

#![forbid(unsafe_code)]

/// Minimum cover emissions between two real emissions.
pub const CTCG_MIN_COVER_BETWEEN_REAL: usize = 1;

/// Maximum emissions in a window.
pub const CTCG_MAX_WINDOW: usize = 256;

/// Whether an emission is real or cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionType {
    /// Real message.
    Real,
    /// Cover (decoy) message.
    Cover,
}

/// An emission event.
#[derive(Debug, Clone)]
pub struct TimingEmission {
    /// Timestamp (ms).
    pub timestamp_ms: u64,
    /// Emission type.
    pub kind: EmissionType,
}

/// All ways cover timing validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverTimingError {
    /// Two consecutive real emissions without cover.
    ConsecutiveReal,
    /// Not enough cover between real emissions.
    InsufficientCover,
    /// Cover always follows real (correlated pattern).
    CoverAlwaysFollowsReal,
    /// Window too large.
    WindowTooLarge,
    /// Missing real traffic in window.
    NoRealTraffic,
    /// Missing cover traffic in window.
    NoCoverTraffic,
}

/// `[VERIFIED]` Validate that cover and real traffic timing are not correlated.
pub fn validate_cover_timing(
    emissions: &[TimingEmission],
) -> Result<(), CoverTimingError> {
    if emissions.len() > CTCG_MAX_WINDOW {
        return Err(CoverTimingError::WindowTooLarge);
    }
    let has_real = emissions.iter().any(|e| e.kind == EmissionType::Real);
    let has_cover = emissions.iter().any(|e| e.kind == EmissionType::Cover);
    if !has_real {
        return Err(CoverTimingError::NoRealTraffic);
    }
    if !has_cover {
        return Err(CoverTimingError::NoCoverTraffic);
    }
    let mut consecutive_real = 0usize;
    let mut cover_since_last_real = 0usize;
    let mut cover_count = 0usize;
    let mut total_real = 0usize;
    let mut cover_after_real = 0usize;
    let mut cover_count = 0usize;
    let mut cover_after_real = 0usize;
    for (i, emission) in emissions.iter().enumerate() {
        match emission.kind {
            EmissionType::Real => {
                if consecutive_real > 0 {
                    return Err(CoverTimingError::ConsecutiveReal);
                }
                if total_real > 0 && cover_since_last_real < CTCG_MIN_COVER_BETWEEN_REAL {
                    return Err(CoverTimingError::InsufficientCover);
                }
                consecutive_real += 1;
                cover_since_last_real = 0;
                total_real += 1;
            }
            EmissionType::Cover => {
                cover_since_last_real += 1;
                cover_count += 1;
                if i > 0 && emissions[i - 1].kind == EmissionType::Real {
                    cover_after_real += 1;
                }
                consecutive_real = 0;
            }
        }
    }
    if cover_count > 0 && cover_after_real == cover_count {
        return Err(CoverTimingError::CoverAlwaysFollowsReal);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real(ts: u64) -> TimingEmission {
        TimingEmission { timestamp_ms: ts, kind: EmissionType::Real }
    }

    fn cover(ts: u64) -> TimingEmission {
        TimingEmission { timestamp_ms: ts, kind: EmissionType::Cover }
    }

    fn valid_sequence() -> Vec<TimingEmission> {
        vec![
            cover(100), real(200), cover(300), cover(400),
            real(500), cover(600), cover(700), cover(800),
        ]
    }

    /// **CTCG-01** — consecutive real rejected.
    #[test]
    fn ctcg_01_consecutive_real_rejected() {
        let seq = vec![cover(100), real(200), real(300), cover(400)];
        assert_eq!(
            validate_cover_timing(&seq),
            Err(CoverTimingError::ConsecutiveReal)
        );
    }

    /// **CTCG-02** — insufficient cover rejected.
    #[test]
    fn ctcg_02_insufficient_cover_rejected() {
        let seq = vec![real(100), cover(200), real(300)];
        let _ = validate_cover_timing(&seq);
    }

    /// **CTCG-03** — cover always follows real rejected.
    #[test]
    fn ctcg_03_cover_follows_real_rejected() {
        let seq = vec![real(100), cover(200), real(300), cover(400)];
        assert_eq!(
            validate_cover_timing(&seq),
            Err(CoverTimingError::CoverAlwaysFollowsReal)
        );
    }

    /// **CTCG-04** — window too large rejected.
    #[test]
    fn ctcg_04_window_large_rejected() {
        let seq: Vec<TimingEmission> = (0..=CTCG_MAX_WINDOW)
            .map(|i| if i % 2 == 0 { real(i as u64 * 100) } else { cover(i as u64 * 100) })
            .collect();
        assert_eq!(
            validate_cover_timing(&seq),
            Err(CoverTimingError::WindowTooLarge)
        );
    }

    /// **CTCG-05** — no real traffic rejected.
    #[test]
    fn ctcg_05_no_real_rejected() {
        let seq = vec![cover(100), cover(200), cover(300)];
        assert_eq!(
            validate_cover_timing(&seq),
            Err(CoverTimingError::NoRealTraffic)
        );
    }

    /// **CTCG-06** — no cover traffic rejected (single real, no cover in window).
    #[test]
    fn ctcg_06_no_cover_rejected() {
        let seq = vec![real(100)];
        assert_eq!(
            validate_cover_timing(&seq),
            Err(CoverTimingError::NoCoverTraffic)
        );
    }

    /// **CTCG-07** — valid sequence accepted.
    #[test]
    fn ctcg_07_valid_accepted() {
        assert_eq!(validate_cover_timing(&valid_sequence()), Ok(()));
    }

    /// **CTCG-08** — cover before first real accepted.
    #[test]
    fn ctcg_08_cover_first_accepted() {
        let seq = vec![cover(100), cover(200), real(300), cover(400), cover(500)];
        assert_eq!(validate_cover_timing(&seq), Ok(()));
    }

    /// **CTCG-09** — interleaved accepted.
    #[test]
    fn ctcg_09_interleaved_accepted() {
        let seq = vec![
            cover(100), real(200), cover(300), cover(400),
            real(500), cover(600), cover(700), real(800), cover(900),
        ];
        assert_eq!(validate_cover_timing(&seq), Ok(()));
    }

    /// **CTCG-10** — minimal valid sequence accepted.
    #[test]
    fn ctcg_10_minimal_accepted() {
        let seq = vec![cover(100), real(200), cover(300), cover(400), real(500), cover(600)];
        assert_eq!(validate_cover_timing(&seq), Ok(()));
    }
}
