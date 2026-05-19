//! GF-01 — arithmetic operations on GF16
//!
//! Pure-Rust phi-anchored arithmetic. FFI-backed implementations live
//! in the legacy `crates/trios-golden-float/src/` and will migrate here.

use trios_golden_float_gf00::{GF16, PHI};

/// Add two GF16 values by their bit-level sum (placeholder, FFI-free).
///
/// The semantically-correct phi-anchored addition will replace this
/// once the FFI layer migrates from the legacy `src/`.
pub fn add_bits(a: GF16, b: GF16) -> GF16 {
    GF16::from_bits(a.to_bits().wrapping_add(b.to_bits()))
}

/// Multiply by phi as f64 (host arithmetic).
pub fn scale_phi(x: f64) -> f64 {
    x * PHI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_bits_roundtrip() {
        let a = GF16::from_bits(1);
        let b = GF16::from_bits(2);
        assert_eq!(add_bits(a, b).to_bits(), 3);
    }

    #[test]
    fn scale_phi_unity() {
        let s = scale_phi(1.0);
        assert!((s - PHI).abs() < 1e-12);
    }
}
