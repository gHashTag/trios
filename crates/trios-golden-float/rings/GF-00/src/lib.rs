//! GF-00 — phi constants and GF16 newtype
//!
//! Bottom of the dependency graph for trios-golden-float.
//! Provides the phi constant and the GF16 wrapping type.
//! Anchor: `phi^2 + phi^-2 = 3`.

/// The golden ratio: (1 + sqrt(5)) / 2.
pub const PHI: f64 = 1.618_033_988_749_894_8;

/// Reciprocal of phi: phi - 1 = 1/phi.
pub const INV_PHI: f64 = 0.618_033_988_749_894_8;

/// GF16 — a 16-bit phi-anchored numeric value.
///
/// Newtype around `u16` bits for FFI compatibility with the legacy
/// `src/lib.rs` implementation. Migration of arithmetic lives in GF-01.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct GF16(pub u16);

impl GF16 {
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    pub const fn to_bits(self) -> u16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_anchor_holds() {
        // phi^2 + phi^-2 = 3 (within fp tolerance)
        let lhs = PHI * PHI + INV_PHI * INV_PHI;
        assert!((lhs - 3.0).abs() < 1e-12, "phi anchor broken: {lhs}");
    }

    #[test]
    fn gf16_roundtrip() {
        let g = GF16::from_bits(0x4248);
        assert_eq!(g.to_bits(), 0x4248);
    }
}
