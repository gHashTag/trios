//! Integration with trios-core types

/// Check if a format is ternary
pub fn is_ternary_format(_format: &str) -> bool {
    // Placeholder: check if format string indicates ternary
    true
}

/// Hardware cost metrics for ternary operations
#[derive(Debug, Clone, Copy)]
pub struct HardwareCost {
    pub dsp_per_param: u32,
    pub lut_per_param: u32,
    pub bram_per_param: u32,
}

impl HardwareCost {
    /// Zero DSP cost for ternary
    pub const fn zero_dsp() -> Self {
        Self {
            dsp_per_param: 0,
            lut_per_param: 52,
            bram_per_param: 0,
        }
    }
}

impl Default for HardwareCost {
    fn default() -> Self {
        Self::zero_dsp()
    }
}

/// Get hardware cost for ternary operations
pub fn hardware_cost() -> HardwareCost {
    HardwareCost::zero_dsp()
}

/// Check if ternary is supported
pub fn supports_ternary() -> bool {
    true
}

/// Get default precision for hybrid pipeline
pub fn default_precision() -> &'static str {
    "ternary"
}

/// Calculate memory bytes for ternary parameters
pub fn ternary_memory_bytes(num_params: usize) -> usize {
    // 1.58 bits/param ≈ 0.2 bytes/param
    num_params / 5
}

/// Calculate compression ratio vs f32
pub fn ternary_compression_ratio() -> f32 {
    32.0 / 1.585
}

/// Calculate compression ratio vs GF16
pub fn ternary_compression_vs_gf16() -> f32 {
    16.0 / 1.585
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_cost_zero_dsp() {
        let cost = hardware_cost();
        assert_eq!(cost.dsp_per_param, 0);
    }

    #[test]
    fn test_supports_ternary() {
        assert!(supports_ternary());
    }

    #[test]
    fn test_ternary_memory_bytes() {
        let bytes = ternary_memory_bytes(1000);
        assert!(bytes > 190 && bytes < 210);
    }

    #[test]
    fn test_compression_ratios() {
        let ratio = ternary_compression_ratio();
        assert!(ratio > 20.0 && ratio < 21.0);

        let ratio_gf16 = ternary_compression_vs_gf16();
        assert!(ratio_gf16 > 10.0 && ratio_gf16 < 11.0);
    }
}
