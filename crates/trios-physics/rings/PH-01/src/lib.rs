//! PH-01 — physics equations
//!
//! Equations on top of PH-00 constants.

use trios_physics_ph00::{C, H};

/// E = m c^2.
pub fn rest_energy(mass_kg: f64) -> f64 {
    mass_kg * C * C
}

/// de Broglie wavelength: lambda = h / p.
pub fn de_broglie_wavelength(momentum: f64) -> f64 {
    H / momentum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rest_energy_one_kg() {
        let e = rest_energy(1.0);
        assert!((e - C * C).abs() < 1e-6);
    }

    #[test]
    fn de_broglie_unit_momentum() {
        let lambda = de_broglie_wavelength(1.0);
        assert!((lambda - H).abs() < 1e-40);
    }
}
