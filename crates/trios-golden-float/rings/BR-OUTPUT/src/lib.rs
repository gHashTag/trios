//! BR-OUTPUT — assembly for trios-golden-float
//!
//! Re-exports the public surface of GF-00 and GF-01.

pub use trios_golden_float_gf00::{GF16, INV_PHI, PHI};
pub use trios_golden_float_gf01::{add_bits, scale_phi};

/// Assembled facade — instantiate to confirm both rings linked.
pub struct GoldenFloat;

impl GoldenFloat {
    pub const fn anchor() -> f64 {
        // phi^2 + phi^-2 = 3
        3.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_const() {
        assert_eq!(GoldenFloat::anchor(), 3.0);
    }

    #[test]
    fn rings_link() {
        let g = GF16::from_bits(0xABCD);
        let h = scale_phi(1.0);
        assert_eq!(g.to_bits(), 0xABCD);
        assert!((h - PHI).abs() < 1e-12);
    }
}
