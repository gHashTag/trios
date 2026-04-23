//! # Precision Router Tests — Validating GF16 vs Ternary Hybrid Policy
//!
//! This test suite validates the static precision router implementation based on the
//! 3-model synthesis consensus (GPT-5.4, Claude Opus 4.7, Gemini 3.1).
//!
//! ## HYBRID PRECISION POLICY
//!
//! | Layer Type | Precision | Reason |
//! |------------|-----------|--------|
//! | Embedding | GF16 | Similarity metrics require full floating-point precision |
//! | Attention (QKV) | GF16 | QKV projection requires gradient precision |
//! | Attention Output | GF16 | Context accumulation needs stable scaling |
//! | FFN Gate/Up | Ternary | Mass quantized, can use ternary with QAT+STE |
//! | FFN Down | GF16 | Projection to residual requires precision |
//! | Conv2D (1-3) | Ternary | Early layers highly quantizable |
//! | Conv2D (4+) | GF16 | Deeper layers need gradient flow |
//! | Output Norm/Act | GF16 | Final layer requires stable scaling |
//!
//! ## HARDWARE COST REFERENCE (from BENCH-006)
//!
//! | Precision | LUT per MAC-16 | DSP per MAC-16 |
//! |-----------|----------------|---------------|
//! | GF16 | 71 LUT | 16 DSP |
//! | Ternary | 52 LUT | 0 DSP |
//! | FP32 | ~94 LUT | ~1 DSP |

#![cfg(test)]

use trios_golden_float::router::{LayerType, Precision, PrecisionRouter};

// ============================================================================
// Layer Type Precision Routing Tests
// ============================================================================

/// Test that embedding layers use GF16 precision.
///
/// Rationale: Similarity metrics require full floating-point precision.
#[test]
fn test_embedding_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::Embedding),
        Precision::GF16,
        "Embedding should use GF16 precision"
    );
}

/// Test that attention QKV layers use GF16 precision.
///
/// Rationale: QKV projection requires gradient precision.
#[test]
fn test_attention_qkv_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::AttentionQKV),
        Precision::GF16,
        "Attention QKV should use GF16 precision"
    );
}

/// Test that attention output layers use GF16 precision.
///
/// Rationale: Context accumulation needs stable scaling.
#[test]
fn test_attention_output_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::AttentionOutput),
        Precision::GF16,
        "Attention output should use GF16 precision"
    );
}

/// Test that FFN gate layers use Ternary precision.
///
/// Rationale: Mass quantized, can use ternary with QAT+STE.
#[test]
fn test_ffn_gate_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::FFNGate),
        Precision::Ternary,
        "FFN gate should use Ternary precision"
    );
}

/// Test that FFN up layers use Ternary precision.
///
/// Rationale: Mass quantized, can use ternary with QAT+STE.
#[test]
fn test_ffn_up_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::FFNUp),
        Precision::Ternary,
        "FFN up should use Ternary precision"
    );
}

/// Test that FFN down layers use GF16 precision.
///
/// Rationale: Projection to residual requires precision.
#[test]
fn test_ffn_down_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::FFNDown),
        Precision::GF16,
        "FFN down should use GF16 precision"
    );
}

/// Test that early Conv2D layers use Ternary precision.
///
/// Rationale: Early layers (1-3) are highly quantizable.
#[test]
fn test_conv2d_early_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::Conv2DEarly),
        Precision::Ternary,
        "Early Conv2D should use Ternary precision"
    );
}

/// Test that deep Conv2D layers use GF16 precision.
///
/// Rationale: Deeper layers (4+) need gradient flow.
#[test]
fn test_conv2d_deep_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::Conv2DDeep),
        Precision::GF16,
        "Deep Conv2D should use GF16 precision"
    );
}

/// Test that layer norm layers use GF16 precision.
///
/// Rationale: Layer normalization requires stable scaling.
#[test]
fn test_layer_norm_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::LayerNorm),
        Precision::GF16,
        "LayerNorm should use GF16 precision"
    );
}

/// Test that output head layers use GF16 precision.
///
/// Rationale: Final layer requires stable scaling.
#[test]
fn test_output_head_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::OutputHead),
        Precision::GF16,
        "Output head should use GF16 precision"
    );
}

/// Test that generic dense layers default to GF16 precision.
///
/// Rationale: Default to GF16 for safety when layer type is unknown.
#[test]
fn test_dense_default_precision() {
    let router = PrecisionRouter::new();
    assert_eq!(
        router.get_precision(LayerType::Dense),
        Precision::GF16,
        "Dense layers should default to GF16 precision"
    );
}

// ============================================================================
// Layer Name Parsing Tests
// ============================================================================

/// Test layer name parsing for embedding.
#[test]
fn test_parse_embedding_name() {
    let router = PrecisionRouter::new();
    let names = ["embedding", "token_embed", "input_embedding", "EMBEDDING"];
    for name in names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::GF16,
            "{} should parse as GF16",
            name
        );
    }
}

/// Test layer name parsing for attention layers.
#[test]
fn test_parse_attention_names() {
    let router = PrecisionRouter::new();
    let attn_names = [
        "attn_qkv", "attention_qkv", "query_key_value", "qkv_projection",
        "attn_out", "attention_output", "attn_output_projection",
    ];
    for name in attn_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::GF16,
            "{} should parse as GF16",
            name
        );
    }
}

/// Test layer name parsing for FFN layers.
#[test]
fn test_parse_ffn_names() {
    let router = PrecisionRouter::new();
    // Gate and Up → Ternary
    let ternary_names = ["ffn_gate", "ffn_up", "gate", "up_projection"];
    for name in ternary_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::Ternary,
            "{} should parse as Ternary",
            name
        );
    }

    // Down → GF16
    assert_eq!(
        router.get_precision_by_name("ffn_down"),
        Precision::GF16,
        "ffn_down should parse as GF16"
    );
}

/// Test layer name parsing for Conv2D layers.
#[test]
fn test_parse_conv2d_names() {
    let router = PrecisionRouter::new();
    let conv_names = ["conv2d", "conv_2d", "CONV2D"];
    for name in conv_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::Ternary,
            "{} should parse as Ternary (early)",
            name
        );
    }
}

/// Test layer name parsing for output layers.
#[test]
fn test_parse_output_names() {
    let router = PrecisionRouter::new();
    let output_names = ["output", "output_head", "classification_head", "OUTPUT"];
    for name in output_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::GF16,
            "{} should parse as GF16",
            name
        );
    }
}

/// Test layer name parsing for layer norm.
#[test]
fn test_parse_norm_names() {
    let router = PrecisionRouter::new();
    let norm_names = ["layer_norm", "layernorm", "norm", "LAYER_NORM"];
    for name in norm_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::GF16,
            "{} should parse as GF16",
            name
        );
    }
}

/// Test that unknown layer names default to GF16.
///
/// Rationale: Default to GF16 for safety when layer type is unknown.
#[test]
fn test_unknown_name_defaults_to_gf16() {
    let router = PrecisionRouter::new();
    let unknown_names = ["mystery_layer", "x_layer", "hidden_block"];
    for name in unknown_names {
        assert_eq!(
            router.get_precision_by_name(name),
            Precision::GF16,
            "{} should default to GF16",
            name
        );
    }
}

// ============================================================================
// Precision Conversion Tests
// ============================================================================

/// Test that conversion detection works correctly.
///
/// Precision conversion is needed when switching between GF16 and Ternary.
#[test]
fn test_conversion_detection() {
    // Same precision → no conversion needed
    assert!(
        !PrecisionRouter::needs_conversion(Precision::GF16, Precision::GF16),
        "GF16 → GF16 should not need conversion"
    );
    assert!(
        !PrecisionRouter::needs_conversion(Precision::Ternary, Precision::Ternary),
        "Ternary → Ternary should not need conversion"
    );
    assert!(
        !PrecisionRouter::needs_conversion(Precision::FP32, Precision::FP32),
        "FP32 → FP32 should not need conversion"
    );

    // Different precision → conversion needed
    assert!(
        PrecisionRouter::needs_conversion(Precision::GF16, Precision::Ternary),
        "GF16 → Ternary should need conversion"
    );
    assert!(
        PrecisionRouter::needs_conversion(Precision::Ternary, Precision::GF16),
        "Ternary → GF16 should need conversion"
    );
    assert!(
        PrecisionRouter::needs_conversion(Precision::GF16, Precision::FP32),
        "GF16 → FP32 should need conversion"
    );
}

/// Test precision conversion is symmetric.
#[test]
fn test_conversion_symmetry() {
    assert_eq!(
        PrecisionRouter::needs_conversion(Precision::GF16, Precision::Ternary),
        PrecisionRouter::needs_conversion(Precision::Ternary, Precision::GF16),
        "Conversion should be symmetric"
    );
}

/// Test all precision pairs for conversion.
#[test]
fn test_all_precision_conversions() {
    let precisions = [Precision::GF16, Precision::Ternary, Precision::FP32];

    for &from in &precisions {
        for &to in &precisions {
            let needs = PrecisionRouter::needs_conversion(from, to);
            let expected = from != to;
            assert_eq!(
                needs, expected,
                "Conversion detection wrong: {} → {} = {}",
                from as i32, to as i32, needs
            );
        }
    }
}

// ============================================================================
// Hardware Cost Tests
// ============================================================================

/// Test LUT cost for GF16 MAC operation.
///
/// From BENCH-006: GF16 MAC-16 = 71 LUT.
#[test]
fn test_gf16_lut_cost() {
    let cost = PrecisionRouter::mac_lut_cost(Precision::GF16);
    assert_eq!(cost, 71, "GF16 MAC should cost 71 LUT");
}

/// Test LUT cost for Ternary MAC operation.
///
/// From BENCH-006: Ternary MAC-16 = 52 LUT.
#[test]
fn test_ternary_lut_cost() {
    let cost = PrecisionRouter::mac_lut_cost(Precision::Ternary);
    assert_eq!(cost, 52, "Ternary MAC should cost 52 LUT");
}

/// Test LUT cost for FP32 MAC operation.
///
/// Estimated: FP32 MAC = GF16 mul cost + overhead.
#[test]
fn test_fp32_lut_cost() {
    let cost = PrecisionRouter::mac_lut_cost(Precision::FP32);
    assert_eq!(cost, 94, "FP32 MAC should cost 94 LUT (estimated)");
}

/// Test DSP cost for GF16 MAC operation.
///
/// From BENCH-006: GF16 MAC-16 = 16 DSP.
#[test]
fn test_gf16_dsp_cost() {
    let cost = PrecisionRouter::mac_dsp_cost(Precision::GF16);
    assert_eq!(cost, 16, "GF16 MAC should use 16 DSP");
}

/// Test DSP cost for Ternary MAC operation.
///
/// From BENCH-006: Ternary MAC-16 = 0 DSP (pure logic).
#[test]
fn test_ternary_dsp_cost() {
    let cost = PrecisionRouter::mac_dsp_cost(Precision::Ternary);
    assert_eq!(cost, 0, "Ternary MAC should use 0 DSP");
}

/// Test DSP cost for FP32 MAC operation.
///
/// Estimated: FP32 MAC = 1 DSP (minimal).
#[test]
fn test_fp32_dsp_cost() {
    let cost = PrecisionRouter::mac_dsp_cost(Precision::FP32);
    assert_eq!(cost, 1, "FP32 MAC should use 1 DSP (estimated)");
}

/// Test GF16 vs Ternary cost ratio.
///
/// GF16 requires 1.37× LUT overhead (71 vs 52 LUT).
#[test]
fn test_cost_ratio_gf16_vs_ternary() {
    let gf16_lut = PrecisionRouter::mac_lut_cost(Precision::GF16);
    let ternary_lut = PrecisionRouter::mac_lut_cost(Precision::Ternary);
    let ratio = gf16_lut as f64 / ternary_lut as f64;

    assert_eq!(ratio, 71.0 / 52.0, "GF16/ternary LUT ratio");
    assert!(
        (ratio - 1.365).abs() < 0.01,
        "GF16 should have 1.37× LUT overhead, got {}",
        ratio
    );
}

/// Test that Ternary has no DSP cost.
///
/// Critical for scalability: Ternary fits ~1,219 parallel units on XC7A100T.
#[test]
fn test_ternary_no_dsp_bottleneck() {
    let dsp_cost = PrecisionRouter::mac_dsp_cost(Precision::Ternary);
    assert_eq!(dsp_cost, 0, "Ternary should not use DSP blocks");

    let lut_cost = PrecisionRouter::mac_lut_cost(Precision::Ternary);
    let total_lut = 63_400usize; // XC7A100T total
    let capacity = total_lut / lut_cost as usize;

    // With 0 DSP, capacity is only LUT-limited
    assert!(
        capacity > 1000,
        "Ternary should fit >1000 parallel units, got {}",
        capacity
    );
}

/// Test GF16 DSP bottleneck calculation.
///
/// With 16 DSP per MAC and 240 total DSP, GF16 fits ~15 parallel units.
#[test]
fn test_gf16_dsp_bottleneck() {
    let dsp_per_mac = PrecisionRouter::mac_dsp_cost(Precision::GF16);
    let total_dsp = 240usize; // XC7A100T total
    let capacity = total_dsp / dsp_per_mac as usize;

    assert_eq!(capacity, 15, "GF16 should fit 15 parallel MAC-16 units");
}

/// Test hardware capacity comparison.
///
/// On XC7A100T: Ternary ~1,219 units, GF16 ~15 units (DSP-limited).
#[test]
fn test_hardware_capacity_comparison() {
    let total_lut = 63_400usize;
    let total_dsp = 240usize;

    // Ternary capacity (LUT-limited)
    let ternary_lut = PrecisionRouter::mac_lut_cost(Precision::Ternary);
    let ternary_capacity = total_lut / ternary_lut;

    // GF16 capacity (DSP-limited)
    let gf16_lut = PrecisionRouter::mac_lut_cost(Precision::GF16);
    let gf16_dsp = PrecisionRouter::mac_dsp_cost(Precision::GF16);
    let gf16_lut_capacity = total_lut / gf16_lut;
    let gf16_dsp_capacity = total_dsp / gf16_dsp;

    // Ternary is LUT-limited with ~1,219 capacity
    assert!(ternary_capacity > 1000, "Ternary capacity should be >1000");

    // GF16 is DSP-limited with ~15 capacity
    assert_eq!(gf16_dsp_capacity, 15, "GF16 DSP capacity should be 15");
    assert_eq!(gf16_lut_capacity, 893, "GF16 LUT capacity should be 893");

    // GF16 DSP-limited capacity << Ternary LUT-limited capacity
    assert!(gf16_dsp_capacity < ternary_capacity,
        "GF16 DSP bottleneck should limit capacity vs Ternary");
}

// ============================================================================
// Bit Width Tests
// ============================================================================

/// Test GF16 bit width.
#[test]
fn test_gf16_bit_width() {
    assert_eq!(Precision::GF16.bit_width(), 16, "GF16 should be 16 bits");
}

/// Test Ternary bit width.
///
/// Ternary uses 2 bits for {-1, 0, +1} representation.
#[test]
fn test_ternary_bit_width() {
    assert_eq!(Precision::Ternary.bit_width(), 2, "Ternary should be 2 bits");
}

/// Test FP32 bit width.
#[test]
fn test_fp32_bit_width() {
    assert_eq!(Precision::FP32.bit_width(), 32, "FP32 should be 32 bits");
}

/// Test bit width relationships.
#[test]
fn test_bit_width_relationships() {
    let gf16_bits = Precision::GF16.bit_width();
    let ternary_bits = Precision::Ternary.bit_width();
    let fp32_bits = Precision::FP32.bit_width();

    // GF16 is 8× Ternary
    assert_eq!(gf16_bits / ternary_bits, 8, "GF16 should be 8× Ternary");

    // FP32 is 2× GF16
    assert_eq!(fp32_bits / gf16_bits, 2, "FP32 should be 2× GF16");

    // Ternary is 1/16 of FP32
    assert_eq!(fp32_bits / ternary_bits, 16, "FP32 should be 16× Ternary");
}

// ============================================================================
// Floating Point Detection Tests
// ============================================================================

/// Test that GF16 is detected as floating-point.
#[test]
fn test_gf16_is_floating_point() {
    assert!(Precision::GF16.is_floating_point(), "GF16 should be floating-point");
}

/// Test that Ternary is NOT floating-point.
///
/// Ternary is a 3-state quantized format.
#[test]
fn test_ternary_is_not_floating_point() {
    assert!(!Precision::Ternary.is_floating_point(), "Ternary should not be floating-point");
}

/// Test that FP32 is detected as floating-point.
#[test]
fn test_fp32_is_floating_point() {
    assert!(Precision::FP32.is_floating_point(), "FP32 should be floating-point");
}

// ============================================================================
// Hybrid Architecture Validation Tests
// ============================================================================

/// Test hybrid architecture precision distribution.
///
/// From 3-model synthesis: 15× GF16 MAC-16 + 3× Ternary MAC-16.
#[test]
fn test_hybrid_precision_distribution() {
    let router = PrecisionRouter::new();

    // Count GF16 layers
    let gf16_layers = vec![
        LayerType::Embedding,
        LayerType::AttentionQKV,
        LayerType::AttentionOutput,
        LayerType::FFNDown,
        LayerType::LayerNorm,
        LayerType::OutputHead,
        LayerType::Dense,
    ];

    // Count Ternary layers
    let ternary_layers = vec![
        LayerType::FFNGate,
        LayerType::FFNUp,
        LayerType::Conv2DEarly,
    ];

    let gf16_count = gf16_layers.len();
    let ternary_count = ternary_layers.len();

    // Hybrid architecture uses both
    assert!(gf16_count > 0, "Should have GF16 layers");
    assert!(ternary_count > 0, "Should have Ternary layers");

    // Verify precision assignment
    for layer in &gf16_layers {
        assert_eq!(
            router.get_precision(layer),
            Precision::GF16,
            "{:?} should be GF16",
            layer
        );
    }

    for layer in &ternary_layers {
        assert_eq!(
            router.get_precision(layer),
            Precision::Ternary,
            "{:?} should be Ternary",
            layer
        );
    }
}

/// Test hybrid resource allocation matches XC7A100T constraints.
///
/// Expected: 15 GF16 MAC-16 = 1,065 LUT + 3,990 FF + 240 DSP (100% DSP).
#[test]
fn test_hybrid_resource_allocation() {
    let num_gf16_mac = 15usize;
    let num_ternary_mac = 3;

    let gf16_lut = PrecisionRouter::mac_lut_cost(Precision::GF16) as usize;
    let ternary_lut = PrecisionRouter::mac_lut_cost(Precision::Ternary) as usize;

    let gf16_dsp = PrecisionRouter::mac_dsp_cost(Precision::GF16) as usize;
    let ternary_dsp = PrecisionRouter::mac_dsp_cost(Precision::Ternary) as usize;

    // Calculate total usage
    let total_lut = num_gf16_mac * gf16_lut + num_ternary_mac * ternary_lut;
    let total_dsp = num_gf16_mac * gf16_dsp + num_ternary_mac * ternary_dsp;

    // Expected values from 3-model synthesis
    assert_eq!(total_lut, 15 * 71 + 3 * 52, "LUT should match hybrid allocation");
    assert_eq!(total_lut, 1_065, "LUT should be 1,065");
    assert_eq!(total_dsp, 15 * 16 + 3 * 0, "DSP should match hybrid allocation");
    assert_eq!(total_dsp, 240, "DSP should use all 240 blocks");

    // Verify LUT percentage of XC7A100T
    let xc7a100t_lut = 63_400usize;
    let lut_percent = total_lut as f64 / xc7a100t_lut as f64 * 100.0;
    assert!(
        (lut_percent - 2.0).abs() < 0.1,
        "Should use ~2% of LUT, got {}%",
        lut_percent
    );

    // Verify DSP percentage of XC7A100T
    let xc7a100t_dsp = 240usize;
    let dsp_percent = total_dsp as f64 / xc7a100t_dsp as f64 * 100.0;
    assert_eq!(dsp_percent, 100.0, "Should use 100% of DSP");
}

/// Test that GF16 is used for all critical layers.
///
/// Critical layers: Embedding, Attention, Output.
#[test]
fn test_critical_layers_use_gf16() {
    let router = PrecisionRouter::new();
    let critical_layers = vec![
        LayerType::Embedding,
        LayerType::AttentionQKV,
        LayerType::AttentionOutput,
        LayerType::OutputHead,
    ];

    for layer in critical_layers {
        assert_eq!(
            router.get_precision(layer),
            Precision::GF16,
            "Critical layer {:?} must use GF16",
            layer
        );
    }
}

/// Test that Ternary is used for all bulk layers.
///
/// Bulk layers: FFN Gate/Up, Conv2D early.
#[test]
fn test_bulk_layers_use_ternary() {
    let router = PrecisionRouter::new();
    let bulk_layers = vec![
        LayerType::FFNGate,
        LayerType::FFNUp,
        LayerType::Conv2DEarly,
    ];

    for layer in bulk_layers {
        assert_eq!(
            router.get_precision(layer),
            Precision::Ternary,
            "Bulk layer {:?} must use Ternary",
            layer
        );
    }
}

// ============================================================================
// Default Implementation Tests
// ============================================================================

/// Test that PrecisionRouter has a Default implementation.
#[test]
fn test_router_default() {
    let router = PrecisionRouter::default();
    // Default should be the same as new()
    let manual = PrecisionRouter::new();

    // Verify behavior is identical
    assert_eq!(
        router.get_precision(LayerType::Embedding),
        manual.get_precision(LayerType::Embedding),
        "Default should match new()"
    );
}

/// Test that PrecisionRouter can be cloned.
#[test]
fn test_router_clone() {
    let router = PrecisionRouter::new();
    let cloned = router.clone();

    assert_eq!(
        router.get_precision(LayerType::Embedding),
        cloned.get_precision(LayerType::Embedding),
        "Cloned router should behave identically"
    );
}

// ============================================================================
// Serialization Tests
// ============================================================================

/// Test LayerType serialization and deserialization.
#[test]
fn test_layer_type_serialization() {
    let layer = LayerType::Embedding;

    // Serialize
    let json = serde_json::to_string(&layer).unwrap();

    // Deserialize
    let deserialized: LayerType = serde_json::from_str(&json).unwrap();

    assert_eq!(layer, deserialized, "LayerType should roundtrip through JSON");
}

/// Test Precision serialization and deserialization.
#[test]
fn test_precision_serialization() {
    let precision = Precision::GF16;

    // Serialize
    let json = serde_json::to_string(&precision).unwrap();

    // Deserialize
    let deserialized: Precision = serde_json::from_str(&json).unwrap();

    assert_eq!(precision, deserialized, "Precision should roundtrip through JSON");
}

/// Test all LayerType values can be serialized.
#[test]
fn test_all_layer_types_serialization() {
    let layers = vec![
        LayerType::Embedding,
        LayerType::AttentionQKV,
        LayerType::AttentionOutput,
        LayerType::FFNGate,
        LayerType::FFNUp,
        LayerType::FFNDown,
        LayerType::Conv2DEarly,
        LayerType::Conv2DDeep,
        LayerType::LayerNorm,
        LayerType::OutputHead,
        LayerType::Dense,
    ];

    for layer in layers {
        let json = serde_json::to_string(&layer).unwrap();
        let deserialized: LayerType = serde_json::from_str(&json).unwrap();
        assert_eq!(layer, deserialized, "{:?} serialization failed", layer);
    }
}

/// Test all Precision values can be serialized.
#[test]
fn test_all_precisions_serialization() {
    let precisions = vec![
        Precision::GF16,
        Precision::Ternary,
        Precision::FP32,
    ];

    for precision in precisions {
        let json = serde_json::to_string(&precision).unwrap();
        let deserialized: Precision = serde_json::from_str(&json).unwrap();
        assert_eq!(precision, deserialized, "{:?} serialization failed", precision);
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

/// Test router behavior with empty layer names.
#[test]
fn test_empty_layer_name() {
    let router = PrecisionRouter::new();
    let precision = router.get_precision_by_name("");

    // Empty name should default to GF16 for safety
    assert_eq!(precision, Precision::GF16, "Empty name should default to GF16");
}

/// Test router behavior with layer names containing multiple keywords.
///
/// Example: "attn_qkv_output" should match AttentionQKV pattern first.
#[test]
fn test_conflicting_layer_names() {
    let router = PrecisionRouter::new();
    let precision = router.get_precision_by_name("attn_qkv_output");

    // Should match "attn" + "qkv" → AttentionQKV → GF16
    assert_eq!(
        precision,
        Precision::GF16,
        "Conflicting name should match first pattern"
    );
}

/// Test router behavior with uppercase layer names.
///
/// Parser should be case-insensitive.
#[test]
fn test_uppercase_layer_names() {
    let router = PrecisionRouter::new();
    let uppercase_names = vec!["EMBEDDING", "ATTN_QKV", "FFN_GATE", "OUTPUT"];
    let expected_precisions = vec![
        Precision::GF16,
        Precision::GF16,
        Precision::Ternary,
        Precision::GF16,
    ];

    for (name, expected) in uppercase_names.iter().zip(expected_precisions.iter()) {
        let actual = router.get_precision_by_name(name);
        assert_eq!(
            actual, *expected,
            "Uppercase {} should parse correctly",
            name
        );
    }
}

/// Test router behavior with hyphenated layer names.
#[test]
fn test_hyphenated_layer_names() {
    let router = PrecisionRouter::new();
    let hyphenated_names = vec![
        ("embedding-layer", Precision::GF16),
        ("attn-qkv-projection", Precision::GF16),
        ("ffn-gate-proj", Precision::Ternary),
        ("conv-2d-early", Precision::Ternary),
    ];

    for (name, expected) in hyphenated_names {
        let actual = router.get_precision_by_name(name);
        assert_eq!(
            actual, expected,
            "Hyphenated {} should parse correctly",
            name
        );
    }
}

/// Test router behavior with underscored layer names.
#[test]
fn test_underscored_layer_names() {
    let router = PrecisionRouter::new();
    let underscored_names = vec![
        ("token_embedding", Precision::GF16),
        ("query_key_value", Precision::GF16),
        ("ffn_gate_proj", Precision::Ternary),
        ("conv_2d_layer_1", Precision::Ternary),
    ];

    for (name, expected) in underscored_names {
        let actual = router.get_precision_by_name(name);
        assert_eq!(
            actual, expected,
            "Underscored {} should parse correctly",
            name
        );
    }
}
