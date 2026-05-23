//! # CR-CHAT-07 — Traffic volume correlation guard (Wave-58 Lane A)
//!
//! ANTI-CORRELATION — объём трафика не выдаёт active/idle, R-CHAT-10.
//!
//! Атакующий наблюдает объём wire traffic за единицу времени. Если
//! active user шлёт больше, чем idle (cover-only), объём — oracle.
//! Защита: объём real+cover за окно ≈ константа.
//!
//! 1. Window содержит ≥ `TVCG_MIN_EMISSIONS`.
//! 2. Объём за окно ∈ [min, max] (узкий диапазон).
//! 3. Std dev объёма ≤ `TVCG_MAX_STDDEV_BYTES`.
//! 4. Нет emission объёма = 0.
//! 5. Нет emission > `TVCG_MAX_SINGLE`.
//! 6. Window size ≤ `TVCG_MAX_WINDOW`.
//!
//! Tests **TVCG-01..10**. Error enum [`VolumeCorrelationError`].
//!
//! Anchor: `φ² + φ⁻² = 3 · TRINITY · CHAT · VOLUME-CORRELATION`

#![forbid(unsafe_code)]

/// Minimum emissions per window.
pub const TVCG_MIN_EMISSIONS: usize = 4;

/// Maximum window size.
pub const TVCG_MAX_WINDOW: usize = 128;

/// Maximum single emission size (bytes).
pub const TVCG_MAX_SINGLE: u64 = 16384;

/// Expected volume per emission (bytes).
pub const TVCG_EXPECTED_PER_EMISSION: u64 = 1024;

/// Allowed deviation from expected (bytes).
pub const TVCG_DEVIATION: u64 = 256;

/// All ways volume correlation can fail.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum VolumeCorrelationError {
    /// Too few emissions.
    TooFewEmissions,
    /// Window too large.
    WindowTooLarge,
    /// Zero-volume emission.
    ZeroVolume,
    /// Single emission too large.
    SingleTooLarge,
    /// Volume outside expected range.
    VolumeOutOfRange,
    /// Variance too high.
    VarianceTooHigh,
}

/// `[VERIFIED]` Validate traffic volume over a window for correlation
/// resistance.
pub fn validate_volume_correlation(
    volumes: &[u64],
) -> Result<(), VolumeCorrelationError> {
    if volumes.len() > TVCG_MAX_WINDOW {
        return Err(VolumeCorrelationError::WindowTooLarge);
    }
    if volumes.len() < TVCG_MIN_EMISSIONS {
        return Err(VolumeCorrelationError::TooFewEmissions);
    }
    for &v in volumes {
        if v == 0 {
            return Err(VolumeCorrelationError::ZeroVolume);
        }
        if v > TVCG_MAX_SINGLE {
            return Err(VolumeCorrelationError::SingleTooLarge);
        }
        let lo = TVCG_EXPECTED_PER_EMISSION.saturating_sub(TVCG_DEVIATION);
        let hi = TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION;
        if v < lo || v > hi {
            return Err(VolumeCorrelationError::VolumeOutOfRange);
        }
    }
    let mean = volumes.iter().sum::<u64>() as f64 / volumes.len() as f64;
    let variance = volumes.iter()
        .map(|&v| { let d = v as f64 - mean; d * d })
        .sum::<f64>() / volumes.len() as f64;
    if variance.sqrt() > (TVCG_DEVIATION as f64) * 0.9 {
        return Err(VolumeCorrelationError::VarianceTooHigh);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_volumes(n: usize) -> Vec<u64> {
        vec![TVCG_EXPECTED_PER_EMISSION; n]
    }

    /// **TVCG-01** — too few rejected.
    #[test]
    fn tvcg_01_too_few_rejected() {
        assert_eq!(
            validate_volume_correlation(&[TVCG_EXPECTED_PER_EMISSION; 3]),
            Err(VolumeCorrelationError::TooFewEmissions)
        );
    }

    /// **TVCG-02** — window too large rejected.
    #[test]
    fn tvcg_02_window_large_rejected() {
        let v = vec![TVCG_EXPECTED_PER_EMISSION; TVCG_MAX_WINDOW + 1];
        assert_eq!(
            validate_volume_correlation(&v),
            Err(VolumeCorrelationError::WindowTooLarge)
        );
    }

    /// **TVCG-03** — zero volume rejected.
    #[test]
    fn tvcg_03_zero_rejected() {
        let mut v = uniform_volumes(4);
        v[0] = 0;
        assert_eq!(
            validate_volume_correlation(&v),
            Err(VolumeCorrelationError::ZeroVolume)
        );
    }

    /// **TVCG-04** — single too large rejected.
    #[test]
    fn tvcg_04_single_large_rejected() {
        let mut v = uniform_volumes(4);
        v[0] = TVCG_MAX_SINGLE + 1;
        assert_eq!(
            validate_volume_correlation(&v),
            Err(VolumeCorrelationError::SingleTooLarge)
        );
    }

    /// **TVCG-05** — volume out of range rejected.
    #[test]
    fn tvcg_05_out_of_range_rejected() {
        let mut v = uniform_volumes(4);
        v[0] = 100;
        assert_eq!(
            validate_volume_correlation(&v),
            Err(VolumeCorrelationError::VolumeOutOfRange)
        );
    }

    /// **TVCG-06** — variance too high rejected.
    #[test]
    fn tvcg_06_variance_high_rejected() {
        let v = vec![
            TVCG_EXPECTED_PER_EMISSION - TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION - TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION - TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION - TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION,
        ];
        assert_eq!(
            validate_volume_correlation(&v),
            Err(VolumeCorrelationError::VarianceTooHigh)
        );
    }

    /// **TVCG-07** — uniform accepted.
    #[test]
    fn tvcg_07_uniform_accepted() {
        assert_eq!(validate_volume_correlation(&uniform_volumes(8)), Ok(()));
    }

    /// **TVCG-08** — slight variation accepted.
    #[test]
    fn tvcg_08_slight_var_accepted() {
        let v = vec![
            TVCG_EXPECTED_PER_EMISSION - 10,
            TVCG_EXPECTED_PER_EMISSION + 10,
            TVCG_EXPECTED_PER_EMISSION - 10,
            TVCG_EXPECTED_PER_EMISSION + 10,
        ];
        assert_eq!(validate_volume_correlation(&v), Ok(()));
    }

    /// **TVCG-09** — exact boundary accepted.
    #[test]
    fn tvcg_09_boundary_accepted() {
        let v = vec![
            TVCG_EXPECTED_PER_EMISSION - TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION,
            TVCG_EXPECTED_PER_EMISSION + TVCG_DEVIATION,
            TVCG_EXPECTED_PER_EMISSION,
        ];
        assert_eq!(validate_volume_correlation(&v), Ok(()));
    }

    /// **TVCG-10** — large window accepted.
    #[test]
    fn tvcg_10_large_accepted() {
        assert_eq!(
            validate_volume_correlation(&uniform_volumes(TVCG_MAX_WINDOW)),
            Ok(())
        );
    }
}
