//! SC-01 — sacred ratios
//!
//! phi (golden ratio), sqrt(2), sqrt(3), pi.

use trios_sacred_sc00::Vec2;

pub const PHI: f64 = 1.618_033_988_749_894_8;
pub const SQRT2: f64 = std::f64::consts::SQRT_2;
pub const SQRT3: f64 = 1.732_050_807_568_877_2;
pub const PI: f64 = std::f64::consts::PI;

/// Scale a Vec2 by phi.
pub fn scale_phi(v: Vec2) -> Vec2 {
    Vec2::new(v.x * PHI, v.y * PHI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_anchor() {
        let lhs = PHI * PHI + (1.0 / PHI) * (1.0 / PHI);
        assert!((lhs - 3.0).abs() < 1e-12);
    }

    #[test]
    fn scale_phi_works() {
        let v = scale_phi(Vec2::new(1.0, 0.0));
        assert!((v.x - PHI).abs() < 1e-12);
    }
}
