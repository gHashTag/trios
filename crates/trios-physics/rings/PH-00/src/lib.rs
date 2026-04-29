//! PH-00 — physical constants
//!
//! Bottom of the ring graph for trios-physics.
//! SI units throughout.

/// Speed of light in vacuum (m/s).
pub const C: f64 = 299_792_458.0;

/// Planck constant (J·s).
pub const H: f64 = 6.626_070_15e-34;

/// Reduced Planck constant (J·s).
pub const HBAR: f64 = 1.054_571_817e-34;

/// Newtonian gravitational constant (m^3 / (kg·s^2)).
pub const G: f64 = 6.674_30e-11;

/// Fine-structure constant (dimensionless).
pub const ALPHA: f64 = 7.297_352_5693e-3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_is_si_value() {
        assert!((C - 299_792_458.0).abs() < 1.0);
    }

    #[test]
    fn alpha_inverse_close_to_137() {
        let inv = 1.0 / ALPHA;
        assert!((inv - 137.035_999_084).abs() < 1e-6);
    }
}
