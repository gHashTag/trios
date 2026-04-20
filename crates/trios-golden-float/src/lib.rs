//! # trios-golden-float
//!
//! Safe Rust wrapper around zig-golden-float, providing GF16 numeric operations.
//! When Zig vendor is not available, all FFI-dependent functions return stubs.

mod ffi;
pub mod router;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct GF16(u16);

#[cfg(has_zig_lib)]
impl GF16 {
    pub fn from_f32(x: f32) -> Self {
        unsafe { GF16(ffi::gf16_from_f32(x)) }
    }
    pub fn to_f32(self) -> f32 {
        unsafe { ffi::gf16_to_f32(self.0) }
    }
    pub fn to_bits(self) -> u16 {
        self.0
    }
    pub fn from_bits(bits: u16) -> Self {
        GF16(bits)
    }
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> Self {
        unsafe { GF16(ffi::gf16_add(self.0, other.0)) }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Self) -> Self {
        unsafe { GF16(ffi::gf16_sub(self.0, other.0)) }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> Self {
        unsafe { GF16(ffi::gf16_mul(self.0, other.0)) }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, other: Self) -> Self {
        unsafe { GF16(ffi::gf16_div(self.0, other.0)) }
    }
}

#[cfg(not(has_zig_lib))]
impl GF16 {
    pub fn from_f32(_x: f32) -> Self {
        GF16(0)
    }
    pub fn to_f32(self) -> f32 {
        0.0
    }
    pub fn to_bits(self) -> u16 {
        self.0
    }
    pub fn from_bits(bits: u16) -> Self {
        GF16(bits)
    }
    #[allow(clippy::should_implement_trait)]
    pub fn add(self, other: Self) -> Self {
        GF16(self.0.wrapping_add(other.0))
    }
    #[allow(clippy::should_implement_trait)]
    pub fn sub(self, other: Self) -> Self {
        GF16(self.0.wrapping_sub(other.0))
    }
    #[allow(clippy::should_implement_trait)]
    pub fn mul(self, other: Self) -> Self {
        GF16(self.0.wrapping_mul(other.0))
    }
    #[allow(clippy::should_implement_trait)]
    pub fn div(self, _other: Self) -> Self {
        GF16(0)
    }
}

#[cfg(has_zig_lib)]
impl std::ops::Add for GF16 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.add(rhs)
    }
}
#[cfg(has_zig_lib)]
impl std::ops::Sub for GF16 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.sub(rhs)
    }
}
#[cfg(has_zig_lib)]
impl std::ops::Mul for GF16 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.mul(rhs)
    }
}

#[cfg(has_zig_lib)]
pub fn compress_weights(weights: &[f32]) -> Vec<u16> {
    let mut out = vec![0u16; weights.len()];
    unsafe {
        ffi::gf16_compress_weights(weights.as_ptr(), weights.len(), out.as_mut_ptr());
    }
    out
}
#[cfg(not(has_zig_lib))]
pub fn compress_weights(weights: &[f32]) -> Vec<u16> {
    vec![0; weights.len()]
}

#[cfg(has_zig_lib)]
pub fn decompress_weights(compressed: &[u16]) -> Vec<f32> {
    let mut out = vec![0.0f32; compressed.len()];
    unsafe {
        ffi::gf16_decompress_weights(compressed.as_ptr(), compressed.len(), out.as_mut_ptr());
    }
    out
}
#[cfg(not(has_zig_lib))]
pub fn decompress_weights(compressed: &[u16]) -> Vec<f32> {
    vec![0.0; compressed.len()]
}

#[cfg(has_zig_lib)]
pub fn dot_product(a: &[u16], b: &[u16]) -> GF16 {
    assert_eq!(a.len(), b.len());
    unsafe { GF16(ffi::gf16_dot_product(a.as_ptr(), b.as_ptr(), a.len())) }
}
#[cfg(not(has_zig_lib))]
pub fn dot_product(a: &[u16], b: &[u16]) -> GF16 {
    assert_eq!(a.len(), b.len());
    GF16(0)
}

#[cfg(has_zig_lib)]
pub fn quantize_matrix(data: &[f32], rows: usize, cols: usize, scale: f32) -> Vec<u16> {
    let mut out = vec![0u16; rows * cols];
    let rc =
        unsafe { ffi::gf16_quantize_matrix(data.as_ptr(), rows, cols, scale, out.as_mut_ptr()) };
    assert_eq!(rc, 0, "gf16_quantize_matrix failed");
    out
}
#[cfg(not(has_zig_lib))]
pub fn quantize_matrix(data: &[f32], rows: usize, cols: usize, _scale: f32) -> Vec<u16> {
    vec![0; rows * cols]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_constant() {
        const PHI: f64 = 1.6180339887498948482;
        assert!((PHI - 1.618033988749895).abs() < 1e-9);
    }

    #[test]
    fn test_gf16_zero() {
        let z = GF16::from_f32(0.0);
        assert_eq!(z.to_bits(), 0u16);
    }

    #[test]
    fn test_gf16_roundtrip() {
        let x = 1.0f32;
        let encoded = GF16::from_f32(x);
        if cfg!(has_zig_lib) {
            assert!((encoded.to_f32() - x).abs() < 0.05);
        }
    }

    #[test]
    #[ignore = "requires zig-golden-float vendor submodule"]
    fn roundtrip_phi() {
        let phi: f32 = 1.618;
        let g = GF16::from_f32(phi);
        let recovered = g.to_f32();
        assert!(
            (recovered - phi).abs() < 0.05,
            "roundtrip failed: {phi} -> GF16 -> {recovered}"
        );
    }
}

pub mod hybrid {
    use super::GF16;

    pub fn quantize_embedding(weights: &[f32], scale: Option<f32>) -> Vec<GF16> {
        let phi_scale = scale.unwrap_or(1.0);
        weights
            .iter()
            .map(|&w| GF16::from_f32(w * phi_scale))
            .collect()
    }
    pub fn quantize_attention(weights: &[f32], scale: Option<f32>) -> Vec<GF16> {
        let phi_scale = scale.unwrap_or(1.0);
        weights
            .iter()
            .map(|&w| GF16::from_f32(w * phi_scale))
            .collect()
    }
    pub fn quantize_output(weights: &[f32], scale: Option<f32>) -> Vec<GF16> {
        let phi_scale = scale.unwrap_or(1.0);
        weights
            .iter()
            .map(|&w| GF16::from_f32(w * phi_scale))
            .collect()
    }
    pub fn compute_phi_scale(weights: &[f32]) -> f32 {
        if weights.is_empty() {
            return 1.0;
        }
        let mean = weights.iter().sum::<f32>() / weights.len() as f32;
        let variance =
            weights.iter().map(|&w| (w - mean).powi(2)).sum::<f32>() / weights.len() as f32;
        let std = variance.sqrt();
        let phi = 1.618_034_f32;
        std.powf(-0.5) / phi
    }
    pub fn dequantize(gf16_weights: &[GF16], scale: f32) -> Vec<f32> {
        gf16_weights.iter().map(|&g| g.to_f32() / scale).collect()
    }
    pub fn gf16_size_bytes(num_params: usize) -> usize {
        num_params * 2
    }
    pub fn compression_ratio(_num_params: usize) -> f32 {
        32.0 / 16.0
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_phi_scale() {
            let weights = vec![0.1, 0.5, 1.0, 1.5, 2.0];
            let scale = compute_phi_scale(&weights);
            assert!(scale > 0.0);
        }
        #[test]
        fn test_size_bytes() {
            assert_eq!(gf16_size_bytes(8_590_032), 17_180_064);
        }
        #[test]
        fn test_compression_ratio() {
            assert_eq!(compression_ratio(1000), 2.0);
        }
    }
}
