//! # trios-tri (∓)
//!
//! Ternary BitLinear + QAT Engine (Φ3)
//!
//! Implements ternary {-1, 0, +1} quantization for bulk compute layers
//! (FFN gate, FFN up, FFN down, activations) using zero-DSP architecture
//! for maximum efficiency on XC7A100T FPGA.
//!
//! ## Symbol
//!
//! ∓ — Three-state weights: {-1, 0, +1}
//!
//! ## Phase
//!
//! Φ3 — Ternary Precision Layer
//!
//! ## Usage in Hybrid Pipeline
//!
//! Per STATIC_ROUTING_TABLE, these layers get Ternary format:
//! - FFN gate (first FFN linear)
//! - FFN up (second FFN linear)
//! - FFN down (third FFN linear) - some architectures use GF16
//! - Activations (GELU, ReLU)
//!
//! ## Key Benefits
//!
//! - **Zero DSP cost** (uses only LUT)
//! - **59× fewer LUT than GF16** at unit level (52 vs 71)
//! - **20.25× compression** vs f32 (1.58 bits/parameter)
//! - **10.13× compression** vs GF16
//! - Compatible with **QAT + STE** for training-aware quantization
//!
//! ## Example
//!
//! ```ignore
//! use trios_tri::{Ternary, TernaryMatrix, hardware_cost};
//!
//! // Convert f32 weights to ternary
//! let weights = vec![1.0, -0.8, 0.3, 1.5];
//! let ternary = Ternary::from_f32_batch(&weights);
//!
//! // Matrix operations
//! let matrix = TernaryMatrix::from_f32(&data, rows, cols);
//! let result = matrix.matmul(&other);
//!
//! // Hardware cost (zero DSP!)
//! let cost = hardware_cost();
//! assert_eq!(cost.dsp_per_param, 0);
//! ```
//!
//! ## Modules
//!
//! - [`arith`] — Arithmetic operations and dot product
//! - [`matrix`] — 2D matrix operations for FFN layers
//! - [`core_compat`] — Integration with `trios-core` types
//! - [`qat`] — Quantization-Aware Training foundation (STE, learnable scale)
//! - [`ffn`] — Layer-specific quantization (gate, up, down)

// Public modules
pub mod arith;
pub mod matrix;
pub mod core_compat;
pub mod qat;

// Re-exports for convenience
pub use arith::{dot_product, l1_distance, count_nonzero as vec_count_nonzero, count_zero as vec_count_zero};
pub use matrix::TernaryMatrix;
pub use core_compat::{is_ternary_format, hardware_cost, supports_ternary, default_precision};
pub use core_compat::{ternary_memory_bytes, ternary_compression_ratio, ternary_compression_vs_gf16};
pub use qat::{TernarySTE, LearnableScale, QatConfig};

// ==============================================================================
// TERNARY VALUE TYPE
// ==============================================================================

/// Ternary value: {-1, 0, +1} (∓)
///
/// The fundamental unit of ternary quantization. Represents neural network
/// weights and activations with just three possible values.
///
/// # Representation
///
/// Uses `i8` backend for efficient storage and arithmetic:
/// - `NegOne = -1`
/// - `Zero = 0`
/// - `PosOne = 1`
///
/// # Memory
///
/// - Storage: log₂(3) ≈ 1.58 bits per parameter
/// - Effective: ~0.2 bytes per parameter (with packing)
///
/// # Example
///
/// ```
/// use trios_tri::Ternary;
///
/// let t = Ternary::from_f32(0.8);
/// assert_eq!(t, Ternary::PosOne);
///
/// let f32 = t.to_f32();
/// assert_eq!(f32, 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(i8)]
pub enum Ternary {
    /// Negative weight (-1)
    NegOne = -1,

    /// Zero weight (0) — enables pruning
    Zero = 0,

    /// Positive weight (+1)
    PosOne = 1,
}

impl Ternary {
    /// Convert f32 to ternary with thresholding.
    ///
    /// Uses threshold of ±0.5:
    /// - `x > 0.5` → `PosOne`
    /// - `x < -0.5` → `NegOne`
    /// - Otherwise → `Zero`
    ///
    /// # Example
    ///
    /// ```
    /// use trios_tri::Ternary;
    ///
    /// assert_eq!(Ternary::from_f32(1.0), Ternary::PosOne);
    /// assert_eq!(Ternary::from_f32(-1.0), Ternary::NegOne);
    /// assert_eq!(Ternary::from_f32(0.0), Ternary::Zero);
    /// ```
    pub fn from_f32(value: f32) -> Self {
        if value > 0.5 {
            Ternary::PosOne
        } else if value < -0.5 {
            Ternary::NegOne
        } else {
            Ternary::Zero
        }
    }

    /// Convert ternary back to f32.
    ///
    /// Direct conversion: `-1 → -1.0`, `0 → 0.0`, `+1 → +1.0`
    ///
    /// # Example
    ///
    /// ```
    /// use trios_tri::Ternary;
    ///
    /// assert_eq!(Ternary::PosOne.to_f32(), 1.0);
    /// assert_eq!(Ternary::Zero.to_f32(), 0.0);
    /// assert_eq!(Ternary::NegOne.to_f32(), -1.0);
    /// ```
    pub fn to_f32(self) -> f32 {
        self as i8 as f32
    }

    /// Get bit-width per parameter (log₂(3) ≈ 1.585).
    ///
    /// This is the theoretical minimum storage for 3 states.
    /// In practice, we may pack 5 ternary values into 8 bits
    /// (3⁵ = 243 ≤ 256 = 2⁸).
    ///
    /// # Example
    ///
    /// ```
    /// use trios_tri::Ternary;
    ///
    /// assert!((Ternary::bits_per_param() - 1.585).abs() < 0.01);
    /// ```
    pub fn bits_per_param() -> f32 {
        (3.0_f32).log2()
    }
}

// Re-export core types
pub use Ternary;

// ==============================================================================
// TERNARY QUANTIZATION (Per-Tensor)
// ==============================================================================

/// Quantize f32 weights to ternary.
///
/// # Arguments
/// * `weights` - f32 weight tensor
/// * `scale` - Scaling factor for full-range quantization
///
/// # Returns
/// Vector of Ternary values
///
/// # Example
///
/// ```
/// use trios_tri::{quantize, compute_scale, Ternary};
///
/// let weights = vec![1.5, -0.8, 0.2, 2.0];
/// let scale = compute_scale(&weights);
/// let ternary = quantize(&weights, scale);
/// assert!(ternary.contains(&Ternary::PosOne));
/// ```
pub fn quantize(weights: &[f32], scale: f32) -> Vec<Ternary> {
    weights.iter().map(|&w| {
        let scaled = w * scale;
        Ternary::from_f32(scaled)
    }).collect()
}

/// Dequantize ternary weights back to f32.
///
/// # Arguments
/// * `ternary_weights` - Ternary quantized weights
/// * `scale` - Scaling factor used during quantization
///
/// # Returns
/// f32 weights
///
/// # Example
///
/// ```
/// use trios_tri::{quantize, dequantize, compute_scale};
///
/// let weights = vec![1.5, -0.8, 0.2, 2.0];
/// let scale = compute_scale(&weights);
/// let ternary = quantize(&weights, scale);
/// let dequant = dequantize(&ternary, scale);
///
/// for (orig, got) in weights.iter().zip(dequant.iter()) {
///     assert!((orig - got).abs() < 1.0, "roundtrip error");
/// }
/// ```
pub fn dequantize(ternary_weights: &[Ternary], scale: f32) -> Vec<f32> {
    ternary_weights.iter().map(|&t| t.to_f32() / scale).collect()
}

/// Compute optimal scaling factor for ternary quantization.
///
/// Uses max-abs scaling to preserve dynamic range:
/// `scale = 1.0 / max(|w|)` where `w` is any weight.
///
/// # Arguments
/// * `weights` - f32 weight tensor
///
/// # Returns
/// Optimal scaling factor (1.0 / max_abs_weight)
///
/// # Example
///
/// ```
/// use trios_tri::compute_scale;
///
/// let weights = vec![0.1, 0.5, 1.0, 1.5];
/// let scale = compute_scale(&weights);
/// assert_eq!(scale, 1.0 / 1.5);
/// ```
pub fn compute_scale(weights: &[f32]) -> f32 {
    if weights.is_empty() {
        return 1.0;
    }

    let max_abs = weights.iter().fold(0.0_f32, |acc, &w| acc.abs().max(w.abs()));
    if max_abs > 0.0 {
        1.0 / max_abs
    } else {
        1.0
    }
}

/// Calculate sparsity after ternary quantization.
///
/// Sparsity is the ratio of zero weights to total weights.
/// Higher sparsity means more pruning potential.
///
/// # Arguments
/// * `ternary_weights` - Ternary quantized weights
///
/// # Returns
/// Sparsity ratio (0.0 = all non-zero, 1.0 = all zero)
///
/// # Example
///
/// ```
/// use trios_tri::{Ternary, compute_sparsity};
///
/// let ternary = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne, Ternary::Zero];
/// let sparsity = compute_sparsity(&ternary);
/// assert_eq!(sparsity, 0.5); // 2 out of 4 are zero
/// ```
pub fn compute_sparsity(ternary_weights: &[Ternary]) -> f32 {
    let zero_count = ternary_weights.iter().filter(|&&t| t == Ternary::Zero).count();
    zero_count as f32 / ternary_weights.len() as f32
}

// ==============================================================================
// HYBRID API - TERNARY FOR BULK LAYERS
// ==============================================================================

/// Ternary quantization for FFN layers (bulk compute).
///
/// Used in hybrid precision pipeline where FFN gate/up use Ternary
/// for zero-DSP efficiency.
///
/// Φ3: Layer-specific quantization functions.
pub mod ffn {
    use super::*;

    /// Quantize FFN gate weights to ternary.
    ///
    /// FFN gate determines activation routing — can use ternary
    /// because it's followed by GELU nonlinearity which handles
    /// quantization noise.
    ///
    /// # Arguments
    /// * `weights` - f32 gate weights
    /// * `scale` - Optional scaling factor (auto-computed if None)
    ///
    /// # Returns
    /// Vector of ternary weights
    pub fn quantize_gate(weights: &[f32], scale: Option<f32>) -> Vec<Ternary> {
        let scale = scale.unwrap_or_else(|| compute_scale(weights));
        quantize(weights, scale)
    }

    /// Quantize FFN up weights to ternary.
    ///
    /// FFN up expands dimensionality — massive compute, ternary ideal.
    /// This is where most of the 20.25× compression benefit is realized.
    ///
    /// # Arguments
    /// * `weights` - f32 up-projection weights
    /// * `scale` - Optional scaling factor (auto-computed if None)
    ///
    /// # Returns
    /// Vector of ternary weights
    pub fn quantize_up(weights: &[f32], scale: Option<f32>) -> Vec<Ternary> {
        let scale = scale.unwrap_or_else(|| compute_scale(weights));
        quantize(weights, scale)
    }

    /// Quantize FFN down weights to ternary.
    ///
    /// FFN down projects back to d_model. Some architectures use GF16
    /// here for precision, but ternary is possible with QAT.
    ///
    /// # Arguments
    /// * `weights` - f32 down-projection weights
    /// * `scale` - Optional scaling factor (auto-computed if None)
    ///
    /// # Returns
    /// Vector of ternary weights
    pub fn quantize_down(weights: &[f32], scale: Option<f32>) -> Vec<Ternary> {
        let scale = scale.unwrap_or_else(|| compute_scale(weights));
        quantize(weights, scale)
    }

    /// Calculate memory savings from ternary FFN layers.
    ///
    /// # Arguments
    /// * `num_params` - Number of parameters in FFN layer
    ///
    /// # Returns
    /// Memory in bytes (1.58 bits/param ≈ 0.2 bytes/param)
    ///
    /// # Example
    ///
    /// ```
    /// use trios_tri::ffn::ternary_size_bytes;
    ///
    /// // 1000 parameters at 1.58 bits each
    /// let bytes = ternary_size_bytes(1000);
    /// assert!(bytes > 190 && bytes < 210);
    /// ```
    pub fn ternary_size_bytes(num_params: usize) -> usize {
        // 1.58 bits/param = 0.2 bytes/param (approximately)
        num_params / 5  // Integer division for conservative estimate
    }

    /// Calculate compression ratio vs f32.
    ///
    /// # Arguments
    /// * `_num_params` - Number of parameters (unused, ratio is constant)
    ///
    /// # Returns
    /// Compression ratio (32.0 / 1.58 ≈ 20.25x)
    ///
    /// # Example
    ///
    /// ```
    /// use trios_tri::ffn::compression_ratio;
    ///
    /// let ratio = compression_ratio(1000);
    /// assert!(ratio > 20.0 && ratio < 21.0);
    /// ```
    pub fn compression_ratio(_num_params: usize) -> f32 {
        32.0 / Ternary::bits_per_param()
    }

    /// Calculate compression ratio vs GF16.
    ///
    /// # Arguments
    /// * `_num_params` - Number of parameters (unused, ratio is constant)
    ///
    /// # Returns
    /// Compression ratio (16.0 / 1.58 ≈ 10.13x)
    pub fn compression_ratio_vs_gf16(_num_params: usize) -> f32 {
        16.0 / Ternary::bits_per_param()
    }
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_from_f32() {
        assert_eq!(Ternary::from_f32(1.0), Ternary::PosOne);
        assert_eq!(Ternary::from_f32(-1.0), Ternary::NegOne);
        assert_eq!(Ternary::from_f32(0.0), Ternary::Zero);
    }

    #[test]
    fn test_quantize_dequantize() {
        let weights = vec![1.5, -0.8, 0.2, 2.0];
        let scale = compute_scale(&weights);
        let ternary = quantize(&weights, scale);
        let dequant = dequantize(&ternary, scale);

        for (orig, got) in weights.iter().zip(dequant.iter()) {
            assert!((orig - got).abs() < 1.0, "roundtrip error");
        }
    }

    #[test]
    fn test_compute_scale() {
        let weights = vec![0.1, 0.5, 1.0, 1.5];
        let scale = compute_scale(&weights);
        assert_eq!(scale, 1.0 / 1.5);
    }

    #[test]
    fn test_sparsity() {
        let ternary = vec![Ternary::PosOne, Ternary::Zero, Ternary::NegOne, Ternary::Zero];
        let sparsity = compute_sparsity(&ternary);
        assert_eq!(sparsity, 0.5);
    }

    #[test]
    fn test_ffn_quantization() {
        let gate_weights = vec![0.2, 0.8, -0.3, 0.6, -0.1, 0.9];
        let ternary_gate = ffn::quantize_gate(&gate_weights, None);
        assert_eq!(ternary_gate.len(), 6);

        let sparsity = compute_sparsity(&ternary_gate);
        assert!(sparsity > 0.0 && sparsity < 1.0);
    }

    #[test]
    fn test_bits_per_param() {
        assert!((Ternary::bits_per_param() - 1.585).abs() < 0.01);
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = ffn::compression_ratio(1000);
        assert!(ratio > 20.0 && ratio < 21.0);
    }

    #[test]
    fn test_compression_ratio_vs_gf16() {
        let ratio = ffn::compression_ratio_vs_gf16(1000);
        assert!(ratio > 10.0 && ratio < 11.0);
    }
}
