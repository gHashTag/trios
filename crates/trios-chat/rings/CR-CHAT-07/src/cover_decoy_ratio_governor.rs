//! # CR-CHAT-07 — Cover decoy ratio governor (Wave-70 Lane A)
//!
//! ANTI-CORRELATION — cover/real ratio must stay in [r_min, r_max], R-CHAT-10.
//!
//! The cover-traffic scheduler emits both real and decoy (cover) envelopes.
//! If the ratio of cover to real falls outside a safe window:
//!
//! * **Ratio too low** — too few cover messages; an observer can
//!   statistically identify real traffic bursts.
//! * **Ratio too high** — too much cover; the channel is easily
//!   distinguished from natural traffic by its high volume.
//! * **No real traffic** — all cover means the user is idle, which is
//!   itself a signal.
//!
//! trios-chat enforces **6 rules**:
//!
//! 1. Cover/real ratio >= `CDRG_MIN_RATIO_NUM / CDRG_MIN_RATIO_DEN`.
//! 2. Cover/real ratio <= `CDRG_MAX_RATIO_NUM / CDRG_MAX_RATIO_DEN`.
//! 3. Window size >= `CDRG_MIN_WINDOW`.
//! 4. Window size <= `CDRG_MAX_WINDOW`.
//! 5. At least one real message in window.
//! 6. At least one cover message in window.
//!
//! Tests **CDRG-01..10**. Error enum [`DecoyRatioError`].
//!
//! Anchor: `phi^2 + phi^-2 = 3 * TRINITY * CHAT * COVER-DECOY-RATIO`

#![forbid(unsafe_code)]

/// Minimum ratio numerator (cover/real).
pub const CDRG_MIN_RATIO_NUM: u64 = 1;

/// Minimum ratio denominator.
pub const CDRG_MIN_RATIO_DEN: u64 = 4;

/// Maximum ratio numerator.
pub const CDRG_MAX_RATIO_NUM: u64 = 3;

/// Maximum ratio denominator.
pub const CDRG_MAX_RATIO_DEN: u64 = 1;

/// Minimum window size (emissions).
pub const CDRG_MIN_WINDOW: usize = 8;

/// Maximum window size (emissions).
pub const CDRG_MAX_WINDOW: usize = 256;

/// All ways decoy ratio validation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecoyRatioError {
    /// Cover/real ratio too low.
    RatioTooLow,
    /// Cover/real ratio too high.
    RatioTooHigh,
    /// Window too small.
    WindowTooSmall,
    /// Window too large.
    WindowTooLarge,
    /// No real traffic in window.
    NoRealTraffic,
    /// No cover traffic in window.
    NoCoverTraffic,
}

/// `[VERIFIED]` Validate cover/real ratio within a window.
pub fn validate_decoy_ratio(
    cover_count: usize,
    real_count: usize,
) -> Result<(), DecoyRatioError> {
    let total = cover_count + real_count;
    if total < CDRG_MIN_WINDOW {
        return Err(DecoyRatioError::WindowTooSmall);
    }
    if total > CDRG_MAX_WINDOW {
        return Err(DecoyRatioError::WindowTooLarge);
    }
    if real_count == 0 {
        return Err(DecoyRatioError::NoRealTraffic);
    }
    if cover_count == 0 {
        return Err(DecoyRatioError::NoCoverTraffic);
    }
    let ratio_num = cover_count as u64 * CDRG_MIN_RATIO_DEN;
    let ratio_den = real_count as u64 * CDRG_MIN_RATIO_NUM;
    if ratio_num < ratio_den {
        return Err(DecoyRatioError::RatioTooLow);
    }
    let high_num = cover_count as u64 * CDRG_MAX_RATIO_DEN;
    let high_den = real_count as u64 * CDRG_MAX_RATIO_NUM;
    if high_num > high_den {
        return Err(DecoyRatioError::RatioTooHigh);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **CDRG-01** — ratio too low rejected.
    #[test]
    fn cdrg_01_ratio_low_rejected() {
        assert_eq!(
            validate_decoy_ratio(1, 10),
            Err(DecoyRatioError::RatioTooLow)
        );
    }

    /// **CDRG-02** — ratio too high rejected.
    #[test]
    fn cdrg_02_ratio_high_rejected() {
        assert_eq!(
            validate_decoy_ratio(50, 1),
            Err(DecoyRatioError::RatioTooHigh)
        );
    }

    /// **CDRG-03** — window too small rejected.
    #[test]
    fn cdrg_03_window_small_rejected() {
        assert_eq!(
            validate_decoy_ratio(2, 2),
            Err(DecoyRatioError::WindowTooSmall)
        );
    }

    /// **CDRG-04** — window too large rejected.
    #[test]
    fn cdrg_04_window_large_rejected() {
        assert_eq!(
            validate_decoy_ratio(200, 200),
            Err(DecoyRatioError::WindowTooLarge)
        );
    }

    /// **CDRG-05** — no real traffic rejected.
    #[test]
    fn cdrg_05_no_real_rejected() {
        assert_eq!(
            validate_decoy_ratio(20, 0),
            Err(DecoyRatioError::NoRealTraffic)
        );
    }

    /// **CDRG-06** — no cover traffic rejected.
    #[test]
    fn cdrg_06_no_cover_rejected() {
        assert_eq!(
            validate_decoy_ratio(0, 20),
            Err(DecoyRatioError::NoCoverTraffic)
        );
    }

    /// **CDRG-07** — balanced ratio accepted.
    #[test]
    fn cdrg_07_balanced_accepted() {
        assert_eq!(validate_decoy_ratio(10, 10), Ok(()));
    }

    /// **CDRG-08** — minimum cover ratio accepted.
    #[test]
    fn cdrg_08_min_ratio_accepted() {
        assert_eq!(validate_decoy_ratio(2, 8), Ok(()));
    }

    /// **CDRG-09** — maximum cover ratio accepted.
    #[test]
    fn cdrg_09_max_ratio_accepted() {
        assert_eq!(validate_decoy_ratio(15, 5), Ok(()));
    }

    /// **CDRG-10** — exact min window accepted.
    #[test]
    fn cdrg_10_min_window_accepted() {
        assert_eq!(validate_decoy_ratio(2, 6), Ok(()));
    }
}
