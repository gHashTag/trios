//! Integration with trios-core types (∓)
//!
//! Φ3: Bridges ternary implementation to SSOT schema.
//!
//! Provides conversion functions and helpers to work with
//! `trios-core` types like `PrecisionFormat`, `HardwareCost`, and `LayerType`.

use trios_core::{HardwareCost, LayerType, PrecisionFormat};

/// Check if a precision format is ternary.
///
/// # Arguments
/// * `format` - Precision format to check
///
/// # Returns
/// `true` if the format is `PrecisionFormat::Ternary158`
///
/// # Example
/// ```
/// use trios_core::PrecisionFormat;
/// use trios_tri::is_ternary_format;
///
/// assert!(is_ternary_format(PrecisionFormat::Ternary158));
/// assert!(!is_ternary_format(PrecisionFormat::GF16));
/// ```
pub fn is_ternary_format(format: PrecisionFormat) -> bool {
    matches!(format, PrecisionFormat::Ternary158)
}

/// Get the hardware cost for ternary precision.
///
/// Returns the pre-defined hardware cost from `trios-core`:
/// - LUT: 52 per MAC-16 unit
/// - DSP: 0 per MAC-16 unit (zero DSP!)
/// - FF: 69 per MAC-16 unit
/// - Cells: 71 per MAC-16 unit
///
/// # Example
/// ```
/// use trios_tri::hardware_cost;
///
/// let cost = hardware_cost();
/// assert_eq!(cost.dsp_per_param, 0); // Zero DSP is the key advantage
/// assert_eq!(cost.lut_per_param, 52);
/// ```
pub fn hardware_cost() -> HardwareCost {
    HardwareCost::ternary()
}

/// Check if a layer type supports ternary quantization.
///
/// Based on sensitivity analysis from BENCH-004:
/// - Dense (FFN): LOW sensitivity → ternary safe
/// - Conv2D (early): LOW sensitivity → ternary safe
/// - Activation: MEDIUM sensitivity → ternary with QAT
///
/// # Arguments
/// * `layer_type` - Layer type to check
///
/// # Returns
/// `true` if the layer type is safe for ternary quantization
///
/// # Example
/// ```
/// use trios_core::LayerType;
/// use trios_tri::supports_ternary;
///
/// assert!(supports_ternary(LayerType::Dense));
/// assert!(supports_ternary(LayerType::Conv2D));
/// assert!(!supports_ternary(LayerType::Embedding)); // HIGH sensitivity
/// ```
pub fn supports_ternary(layer_type: LayerType) -> bool {
    matches!(
        layer_type,
        LayerType::Dense | LayerType::Conv2D | LayerType::Activation
    )
}

/// Get the default precision format for a layer type.
///
/// Delegates to `LayerType::default_precision()` from `trios-core`.
///
/// # Arguments
/// * `layer_type` - Layer type
///
/// # Returns
/// The default precision format for this layer type
///
/// # Example
/// ```
/// use trios_core::{LayerType, PrecisionFormat};
/// use trios_tri::default_precision;
///
/// assert_eq!(default_precision(LayerType::Dense), PrecisionFormat::Ternary158);
/// assert_eq!(default_precision(LayerType::Embedding), PrecisionFormat::GF16);
/// ```
pub fn default_precision(layer_type: LayerType) -> PrecisionFormat {
    layer_type.default_precision()
}

/// Calculate memory usage in bytes for ternary precision.
///
/// Uses 1.58 bits per parameter (log₂(3)).
///
/// # Arguments
/// * `param_count` - Number of parameters
///
/// # Returns
/// Memory usage in bytes
///
/// # Example
/// ```
/// use trios_tri::ternary_memory_bytes;
///
/// // 1000 parameters * 1.58 bits / 8 ≈ 198 bytes
/// let bytes = ternary_memory_bytes(1000);
/// assert!(bytes > 190 && bytes < 210);
/// ```
pub fn ternary_memory_bytes(param_count: usize) -> usize {
    // 1.58 bits per parameter = log₂(3)
    // Ceiling to ensure we allocate enough space
    ((param_count as f32 * 1.58_f32) / 8.0).ceil() as usize
}

/// Calculate compression ratio vs f32 for ternary.
///
/// # Arguments
/// * `_param_count` - Number of parameters (unused, ratio is constant)
///
/// # Returns
/// Compression ratio (approximately 20.25×)
///
/// # Example
/// ```
/// use trios_tri::ternary_compression_ratio;
///
/// let ratio = ternary_compression_ratio(1000);
/// assert!(ratio > 20.0 && ratio < 21.0);
/// ```
pub fn ternary_compression_ratio(_param_count: usize) -> f32 {
    32.0 / 1.58_f32 // 32 bits (f32) / 1.58 bits (ternary)
}

/// Calculate compression ratio vs GF16 for ternary.
///
/// # Arguments
/// * `_param_count` - Number of parameters (unused, ratio is constant)
///
/// # Returns
/// Compression ratio (approximately 10.13×)
///
/// # Example
/// ```
/// use trios_tri::ternary_compression_vs_gf16;
///
/// let ratio = ternary_compression_vs_gf16(1000);
/// assert!(ratio > 10.0 && ratio < 11.0);
/// ```
pub fn ternary_compression_vs_gf16(_param_count: usize) -> f32 {
    16.0 / 1.58_f32 // 16 bits (GF16) / 1.58 bits (ternary)
}

/// Check if ternary format is suitable for a given sensitivity.
///
/// Ternary is suitable for LOW and MEDIUM sensitivity layers,
/// but requires QAT for MEDIUM sensitivity layers.
///
/// # Arguments
/// * `sensitivity` - Sensitivity level from `trios-core`
///
/// # Returns
/// `true` if ternary is potentially suitable (may require QAT)
pub fn is_ternary_suitable(sensitivity: trios_core::Sensitivity) -> bool {
    // Ternary is suitable for LOW sensitivity without QAT
    // and MEDIUM sensitivity with QAT
    matches!(
        sensitivity,
        trios_core::Sensitivity::LOW | trios_core::Sensitivity::MEDIUM
    )
}

/// Get a description of the ternary format for documentation.
///
/// # Returns
/// A string describing the ternary format characteristics
pub fn format_description() -> &'static str {
    "Ternary158: {-1, 0, +1} quantization with 1.58 bits/parameter. \
     Zero DSP cost, ideal for bulk compute layers (FFN, early Conv2D). \
     Requires QAT+STE for training to maintain accuracy."
}

#[cfg(test)]
mod tests {
    use super::*;
    use trios_core::{LayerType, PrecisionFormat, Sensitivity};

    #[test]
    fn test_is_ternary_format() {
        assert!(is_ternary_format(PrecisionFormat::Ternary158));
        assert!(!is_ternary_format(PrecisionFormat::GF16));
        assert!(!is_ternary_format(PrecisionFormat::FP32));
    }

    #[test]
    fn test_hardware_cost() {
        let cost = hardware_cost();
        assert_eq!(cost.lut_per_param, 52);
        assert_eq!(cost.dsp_per_param, 0); // Key advantage
        assert_eq!(cost.ff_per_param, 69);
        assert_eq!(cost.cells_per_param, 71);
    }

    #[test]
    fn test_supports_ternary() {
        assert!(supports_ternary(LayerType::Dense));
        assert!(supports_ternary(LayerType::Conv2D));
        assert!(supports_ternary(LayerType::Activation));
        assert!(!supports_ternary(LayerType::Embedding)); // HIGH
        assert!(!supports_ternary(LayerType::Attention)); // HIGH
        assert!(!supports_ternary(LayerType::OutputHead)); // HIGH
    }

    #[test]
    fn test_default_precision() {
        assert_eq!(
            default_precision(LayerType::Dense),
            PrecisionFormat::Ternary158
        );
        assert_eq!(
            default_precision(LayerType::Conv2D),
            PrecisionFormat::Ternary158
        );
        assert_eq!(
            default_precision(LayerType::Activation),
            PrecisionFormat::Ternary158
        );
        assert_eq!(
            default_precision(LayerType::Embedding),
            PrecisionFormat::GF16
        );
        assert_eq!(
            default_precision(LayerType::Attention),
            PrecisionFormat::GF16
        );
        assert_eq!(
            default_precision(LayerType::OutputHead),
            PrecisionFormat::GF16
        );
    }

    #[test]
    fn test_ternary_memory_bytes() {
        let bytes = ternary_memory_bytes(1000);
        // 1000 * 1.58 / 8 ≈ 197.5, ceil = 198
        assert!((197..=199).contains(&bytes));
    }

    #[test]
    fn test_ternary_compression_ratio() {
        let ratio = ternary_compression_ratio(1000);
        // 32 / 1.58 ≈ 20.25
        assert!(ratio > 20.0 && ratio < 21.0);
    }

    #[test]
    fn test_ternary_compression_vs_gf16() {
        let ratio = ternary_compression_vs_gf16(1000);
        // 16 / 1.58 ≈ 10.13
        assert!(ratio > 10.0 && ratio < 11.0);
    }

    #[test]
    fn test_is_ternary_suitable() {
        assert!(is_ternary_suitable(Sensitivity::LOW));
        assert!(is_ternary_suitable(Sensitivity::MEDIUM));
        assert!(!is_ternary_suitable(Sensitivity::HIGH));
    }

    #[test]
    fn test_format_description() {
        let desc = format_description();
        assert!(desc.contains("Ternary158"));
        assert!(desc.contains("1.58 bits"));
        assert!(desc.contains("Zero DSP"));
    }
}
