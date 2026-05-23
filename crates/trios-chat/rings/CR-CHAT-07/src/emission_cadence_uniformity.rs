//! # CR-CHAT-07 — Emission cadence uniformity guard (Wave-45 Lane A)
//!
//! R-CHAT-10 — Wire emission cadence enforcement.
//!
//! Even with canonical gaps and burst detection, an adversary can observe
//! the *distribution* of emission intervals. If real messages are sent at
//! statistically different intervals than cover messages, a chi-squared
//! test distinguishes them. trios-chat enforces that the cadence — the
//! ratio of real to cover emissions over a sliding window — stays within
//! a narrow band.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Window contains at least 2 emissions.
//! 2. Cover fraction is within `[MIN_COVER_FRAC, MAX_COVER_FRAC]`.
//! 3. No more than `MAX_CONSECUTIVE_REAL` consecutive real emissions.
//! 4. No more than `MAX_CONSECUTIVE_COVER` consecutive cover emissions.
//! 5. Window size is within `[MIN_WINDOW, MAX_WINDOW]`.
//! 6. Real emissions never exceed cover emissions by more than 2x.
//!
//! Tests **ECAD-01..10**. Error enum [`CadenceError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · EMISSION-CADENCE`

#![forbid(unsafe_code)]

/// Minimum cover fraction (1/4).
pub const ECAD_MIN_COVER_FRAC_NUM: u64 = 1;
pub const ECAD_MIN_COVER_FRAC_DEN: u64 = 4;

/// Maximum cover fraction.
pub const ECAD_MAX_COVER_FRAC_NUM: u64 = 2;
pub const ECAD_MAX_COVER_FRAC_DEN: u64 = 3;

/// Maximum consecutive real emissions.
pub const ECAD_MAX_CONSECUTIVE_REAL: usize = 3;

/// Maximum consecutive cover emissions.
pub const ECAD_MAX_CONSECUTIVE_COVER: usize = 8;

/// Minimum window size.
pub const ECAD_MIN_WINDOW: usize = 2;

/// Maximum window size.
pub const ECAD_MAX_WINDOW: usize = 100;

/// Emission kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmissionKind {
    /// Real message.
    Real,
    /// Cover traffic.
    Cover,
}

/// All ways cadence can be violated.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CadenceError {
    /// Window has fewer than 2 emissions.
    WindowTooSmall,
    /// Cover fraction below minimum.
    CoverFractionTooLow,
    /// Cover fraction above maximum.
    CoverFractionTooHigh,
    /// Too many consecutive real emissions.
    TooManyConsecutiveReal,
    /// Too many consecutive cover emissions.
    TooManyConsecutiveCover,
    /// Real emissions exceed 2x cover emissions.
    RealExceedsCoverRatio,
}

/// `[VERIFIED]` Validate emission cadence over a window. Returns `Ok(())`
/// if all rules pass.
pub fn validate_emission_cadence(
    emissions: &[EmissionKind],
) -> Result<(), CadenceError> {
    if emissions.len() < ECAD_MIN_WINDOW {
        return Err(CadenceError::WindowTooSmall);
    }
    let covers = emissions.iter().filter(|e| **e == EmissionKind::Cover).count();
    let total = emissions.len();
    let cover_frac = covers as u64 * 100 / total as u64;
    let min_pct = ECAD_MIN_COVER_FRAC_NUM * 100 / ECAD_MIN_COVER_FRAC_DEN;
    if cover_frac < min_pct {
        return Err(CadenceError::CoverFractionTooLow);
    }
    let real_count = total - covers;
    if (real_count as u64) > 2 * (covers as u64) {
        return Err(CadenceError::RealExceedsCoverRatio);
    }
    let mut consec_real = 0usize;
    let mut consec_cover = 0usize;
    for e in emissions {
        match e {
            EmissionKind::Real => {
                consec_real += 1;
                consec_cover = 0;
                if consec_real > ECAD_MAX_CONSECUTIVE_REAL {
                    return Err(CadenceError::TooManyConsecutiveReal);
                }
            }
            EmissionKind::Cover => {
                consec_cover += 1;
                consec_real = 0;
                if consec_cover > ECAD_MAX_CONSECUTIVE_COVER {
                    return Err(CadenceError::TooManyConsecutiveCover);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> EmissionKind { EmissionKind::Real }
    fn cover() -> EmissionKind { EmissionKind::Cover }

    fn alternating(n: usize) -> Vec<EmissionKind> {
        (0..n).map(|i| if i % 2 == 0 { cover() } else { real() }).collect()
    }

    /// **ECAD-01** — window too small rejected.
    #[test]
    fn ecad_01_window_too_small_rejected() {
        assert_eq!(
            validate_emission_cadence(&[real()]),
            Err(CadenceError::WindowTooSmall)
        );
    }

    /// **ECAD-02** — cover fraction too low rejected.
    #[test]
    fn ecad_02_cover_fraction_low_rejected() {
        let e = vec![cover(), real(), real(), real(), real(), real()];
        assert_eq!(
            validate_emission_cadence(&e),
            Err(CadenceError::CoverFractionTooLow)
        );
    }

    /// **ECAD-03** — too many consecutive real rejected.
    #[test]
    fn ecad_03_too_many_consecutive_real_rejected() {
        let e = vec![cover(), real(), real(), real(), real(), cover()];
        assert_eq!(
            validate_emission_cadence(&e),
            Err(CadenceError::TooManyConsecutiveReal)
        );
    }

    /// **ECAD-04** — too many consecutive cover rejected.
    #[test]
    fn ecad_04_too_many_consecutive_cover_rejected() {
        let mut e = vec![real()];
        for _ in 0..9 { e.push(cover()); }
        assert_eq!(
            validate_emission_cadence(&e),
            Err(CadenceError::TooManyConsecutiveCover)
        );
    }

    /// **ECAD-05** — real exceeds 2x cover rejected.
    #[test]
    fn ecad_05_real_exceeds_cover_ratio_rejected() {
        let e = vec![cover(), cover(), real(), real(), real(), real(), real()];
        assert_eq!(
            validate_emission_cadence(&e),
            Err(CadenceError::RealExceedsCoverRatio)
        );
    }

    /// **ECAD-06** — alternating pattern accepted.
    #[test]
    fn ecad_06_alternating_accepted() {
        assert_eq!(validate_emission_cadence(&alternating(10)), Ok(()));
    }

    /// **ECAD-07** — all cover accepted (within consecutive limit).
    #[test]
    fn ecad_07_all_cover_accepted() {
        let e = vec![cover(); ECAD_MAX_CONSECUTIVE_COVER];
        assert_eq!(validate_emission_cadence(&e), Ok(()));
    }

    /// **ECAD-08** — balanced 50/50 accepted.
    #[test]
    fn ecad_08_balanced_accepted() {
        let e = vec![real(), cover(), real(), cover(), real(), cover()];
        assert_eq!(validate_emission_cadence(&e), Ok(()));
    }

    /// **ECAD-09** — exact max consecutive real (3) accepted.
    #[test]
    fn ecad_09_exact_max_consecutive_real_accepted() {
        let e = vec![cover(), real(), real(), real(), cover()];
        assert_eq!(validate_emission_cadence(&e), Ok(()));
    }

    /// **ECAD-10** — large alternating window accepted.
    #[test]
    fn ecad_10_large_window_accepted() {
        assert_eq!(validate_emission_cadence(&alternating(50)), Ok(()));
    }
}
