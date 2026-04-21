//! Precision Router - Mixed-Precision Policy Engine
//!
//! Assigns GF16 or Ternary format to layers based on:
//! - Sensitivity (HIGH for critical layers)
//! - Hardware budget (DSP/LUT constraints)
//! - Transition overhead (minimize GF16↔ternary boundaries)

use serde::{Deserialize, Serialize};

// ==============================================================================
// LAYER CLASSIFICATION (GF16 vs Ternary)
// ==============================================================================

/// Layer type determines quantization strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    /// Critical precision layers (embedding, attention, output)
    /// Require GF16 for f32 parity (0.00% gap)
    GF16,

    /// Bulk computation layers (FFN/linear activations)
    /// Use ternary {-1, 0, +1} for zero-DSP efficiency
    Ternary,
}

impl LayerType {
    /// Get computational cost multiplier (relative to f32)
    pub fn cost_multiplier(&self) -> f32 {
        match self {
            LayerType::GF16 => 1.0,        // Same as f32 (uses DSP)
            LayerType::Ternary => 0.0,    // Zero DSP cost (ternary only)
        }
    }

    /// Get DSP units per parameter (XC7A100T)
    pub fn dsp_per_param(&self) -> u32 {
        match self {
            LayerType::GF16 => 16,         // 16 DSP per MAC-16 operation
            LayerType::Ternary => 0,        // Ternary uses only LUT
        }
    }

    /// Get LUT units per parameter (XC7A100T)
    pub fn lut_per_param(&self) -> u32 {
        match self {
            LayerType::GF16 => 118,        // From whitepaper BENCH-005
            LayerType::Ternary => 2,        // From whitepaper BENCH-005
        }
    }
}

// ==============================================================================
// LAYER SPECIFICATION
// ==============================================================================

/// Layer specification for routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSpec {
    /// Layer name (e.g., "embedding", "attn_qkv", "ffn_gate")
    pub name: String,

    /// Layer type determines quantization format
    pub layer_type: LayerType,

    /// Number of parameters in this layer
    pub param_count: usize,

    /// Sensitivity to quantization (HIGH/MEDIUM/LOW)
    pub sensitivity: Sensitivity,

    /// Whether this layer can be converted to ternary without QAT
    pub ternary_without_qat: bool,
}

impl LayerSpec {
    /// Create a new layer specification
    pub fn new(
        name: impl Into<String>,
        layer_type: LayerType,
        param_count: usize,
        sensitivity: Sensitivity,
        ternary_without_qat: bool,
    ) -> Self {
        Self {
            name: name.into(),
            layer_type,
            param_count,
            sensitivity,
            ternary_without_qat,
        }
    }

    /// Estimate total resource cost for XC7A100T
    pub fn estimate_cost(&self, dsp_budget: u32, lut_budget: u32) -> bool {
        let dsp_cost = self.param_count as u32 * self.layer_type.dsp_per_param();
        let lut_cost = self.param_count as u32 * self.layer_type.lut_per_param();

        dsp_cost <= dsp_budget && lut_cost <= lut_budget
    }
}

// ==============================================================================
// SENSITIVITY LEVELS
// ==============================================================================

/// Quantization sensitivity determines precision requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sensitivity {
    /// Critical: requires GF16 for f32 parity (embedding, output)
    HIGH,

    /// Medium: can use ternary with QAT, but may lose some accuracy
    MEDIUM,

    /// Low: safe for ternary even without QAT (activation functions)
    LOW,
}

// ==============================================================================
// HARDWARE BUDGET CONSTRAINTS
// ==============================================================================

/// Memory budget for 16MB Parameter Golf constraint
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MemoryBudget {
    /// Target size in bytes (16MB = 16,777,216 bytes)
    pub target_bytes: u32,

    /// Maximum compression ratio allowed
    pub max_compression_ratio: f32,
}

impl MemoryBudget {
    /// Create 16MB memory budget (Parameter Golf constraint)
    pub fn mb_16() -> Self {
        Self {
            target_bytes: 16 * 1024 * 1024,
            max_compression_ratio: 4.0,  // GF16 + zstd achieves ~4x
        }
    }

    /// Check if model size fits within budget
    pub fn fits(&self, model_size_bytes: u32) -> bool {
        model_size_bytes <= self.target_bytes
    }
}

/// DSP budget for XC7A100T (240 DSP total)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DspBudget {
    /// Total DSP units available on XC7A100T
    pub total_dsp: u32,

    /// Maximum DSP utilization percentage (0.0-1.0)
    pub max_utilization: f32,
}

impl DspBudget {
    /// Create XC7A100T DSP budget
    pub fn xc7a100t() -> Self {
        Self {
            total_dsp: 240,
            max_utilization: 0.95,
        }
    }

    /// Get available DSP units
    pub fn available_dsp(&self) -> u32 {
        (self.total_dsp as f32 * self.max_utilization) as u32
    }
}

// ==============================================================================
// STATIC ROUTING POLICY (Claude/Gemini Approach)
// ==============================================================================

/// Static routing table from AI consensus analysis
///
/// Policy:
/// - Embedding: GF16 (HIGH sensitivity, critical for representation)
/// - Attention (QKV, proj): GF16 (HIGH sensitivity, attention is fragile)
/// - Output: GF16 (HIGH sensitivity, final logits must be precise)
/// - FFN (gate, up): Ternary (LOW sensitivity, bulk compute, zero DSP)
/// - Layer norm: GF16 (MEDIUM sensitivity, normalization stability)
pub static STATIC_ROUTING_TABLE: &[(&str, LayerType)] = &[
    // Critical precision layers → GF16
    ("embedding", LayerType::GF16),
    ("tok_emb", LayerType::GF16),
    ("attn_qkv", LayerType::GF16),
    ("attn_proj", LayerType::GF16),
    ("attn_out", LayerType::GF16),
    ("final_norm", LayerType::GF16),
    ("output", LayerType::GF16),

    // Bulk computation layers → Ternary
    ("mlp_gate", LayerType::Ternary),
    ("mlp_up", LayerType::Ternary),
    ("mlp_down", LayerType::Ternary),
    ("ffn_gate", LayerType::Ternary),
    ("ffn_up", LayerType::Ternary),
    ("ffn_down", LayerType::Ternary),
    ("activation", LayerType::Ternary),
];

/// Get static routing decision for a layer name
pub fn get_static_routing(layer_name: &str) -> Option<LayerType> {
    STATIC_ROUTING_TABLE
        .iter()
        .find(|(name, _)| layer_name.contains(name))
        .map(|(_, layer_type)| *layer_type)
}

// ==============================================================================
// PRECISION FORMAT (Union Type)
// ==============================================================================

/// Precision format for quantization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrecisionFormat {
    /// GoldenFloat16 (1:6:9 format)
    GF16,

    /// Ternary {-1, 0, +1} (1.58 bits/weight)
    Ternary,
}

impl PrecisionFormat {
    /// Get bit-width per parameter
    pub fn bits_per_param(&self) -> f32 {
        match self {
            PrecisionFormat::GF16 => 16.0,
            PrecisionFormat::Ternary => 1.58,  // log2(3) ≈ 1.585
        }
    }

    /// Get compression ratio vs f32 (32 bits/param)
    pub fn compression_ratio(&self) -> f32 {
        32.0 / self.bits_per_param()
    }
}

// ==============================================================================
// ROUTING PLAN
// ==============================================================================

/// Routing plan for all layers in a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPlan {
    /// Layer specifications with assigned formats
    pub layers: Vec<LayerSpec>,

    /// Total estimated DSP usage
    pub estimated_dsp: u32,

    /// Total estimated LUT usage
    pub estimated_lut: u32,

    /// Total estimated model size (bytes)
    pub estimated_size_bytes: u32,

    /// Whether plan fits within all budgets
    pub fits_budget: bool,
}

impl RoutingPlan {
    /// Calculate estimated resource usage
    pub fn estimate_resources(&mut self, budget: &MemoryBudget, dsp_budget: &DspBudget) {
        let mut total_dsp = 0u32;
        let mut total_lut = 0u32;
        let mut total_bits = 0u64;

        for layer in &self.layers {
            total_dsp += layer.param_count as u32 * layer.layer_type.dsp_per_param();
            total_lut += layer.param_count as u32 * layer.layer_type.lut_per_param();
            // Convert layer_type to precision format for bit calculation
            let precision = match layer.layer_type {
                LayerType::GF16 => PrecisionFormat::GF16,
                LayerType::Ternary => PrecisionFormat::Ternary,
            };
            total_bits += (layer.param_count as f64 * precision.bits_per_param() as f64) as u64;
        }

        self.estimated_dsp = total_dsp;
        self.estimated_lut = total_lut;
        self.estimated_size_bytes = (total_bits / 8) as u32;
        self.fits_budget =
            budget.fits(self.estimated_size_bytes) &&
            total_dsp <= dsp_budget.available_dsp();
    }
}

// ==============================================================================
// ROUTER (Public API)
// ==============================================================================

/// Precision router - assigns GF16 or Ternary to layers
pub struct PrecisionRouter {
    /// Whether to use static policy (Claude/Gemini) or dynamic search (GPT-5.4)
    use_static_policy: bool,
}

impl PrecisionRouter {
    /// Create new precision router
    pub fn new(use_static_policy: bool) -> Self {
        Self {
            use_static_policy,
        }
    }

    /// Plan routing for all layers
    pub fn plan(
        &self,
        layers: &[LayerSpec],
        budget: &MemoryBudget,
        dsp_budget: &DspBudget,
    ) -> anyhow::Result<RoutingPlan> {
        let mut routed_layers = Vec::with_capacity(layers.len());

        for layer in layers {
            let layer_type = if self.use_static_policy {
                // Static routing from AI consensus
                get_static_routing(&layer.name)
                    .ok_or_else(|| anyhow::anyhow!("No static routing for layer: {}", layer.name))?
            } else {
                // Dynamic routing based on sensitivity and budget
                self.plan_dynamic(layer, budget, dsp_budget)?
            };

            routed_layers.push(LayerSpec {
                name: layer.name.clone(),
                layer_type,
                param_count: layer.param_count,
                sensitivity: layer.sensitivity,
                ternary_without_qat: layer.ternary_without_qat,
            });
        }

        let mut plan = RoutingPlan {
            layers: routed_layers,
            estimated_dsp: 0,
            estimated_lut: 0,
            estimated_size_bytes: 0,
            fits_budget: false,
        };

        plan.estimate_resources(budget, dsp_budget);

        Ok(plan)
    }

    /// Dynamic routing based on sensitivity and budget
    fn plan_dynamic(
        &self,
        layer: &LayerSpec,
        _budget: &MemoryBudget,
        dsp_budget: &DspBudget,
    ) -> anyhow::Result<LayerType> {
        // HIGH sensitivity → GF16 (non-negotiable)
        if layer.sensitivity == Sensitivity::HIGH {
            return Ok(LayerType::GF16);
        }

        // MEDIUM sensitivity → GF16 if QAT available, else Ternary
        if layer.sensitivity == Sensitivity::MEDIUM {
            // Check if we can afford GF16
            let dsp_cost = layer.param_count as u32 * LayerType::GF16.dsp_per_param();
            if dsp_cost <= dsp_budget.available_dsp() {
                return Ok(LayerType::GF16);
            }
            // If QAT available, can try ternary
            if !layer.ternary_without_qat {
                return Ok(LayerType::Ternary);
            }
            // Fall back to GF16 (QAT available)
            return Ok(LayerType::GF16);
        }

        // LOW sensitivity → Ternary (safe)
        Ok(LayerType::Ternary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_routing_table() {
        // Test critical layers route to GF16
        assert_eq!(get_static_routing("embedding"), Some(LayerType::GF16));
        assert_eq!(get_static_routing("attn_qkv"), Some(LayerType::GF16));
        assert_eq!(get_static_routing("output"), Some(LayerType::GF16));

        // Test bulk layers route to Ternary
        assert_eq!(get_static_routing("mlp_gate"), Some(LayerType::Ternary));
        assert_eq!(get_static_routing("ffn_up"), Some(LayerType::Ternary));
    }

    #[test]
    fn test_cost_multipliers() {
        // GF16 uses DSP
        assert_eq!(LayerType::GF16.cost_multiplier(), 1.0);
        assert_eq!(LayerType::GF16.dsp_per_param(), 16);
        assert_eq!(LayerType::GF16.lut_per_param(), 118);

        // Ternary uses only LUT
        assert_eq!(LayerType::Ternary.cost_multiplier(), 0.0);
        assert_eq!(LayerType::Ternary.dsp_per_param(), 0);
        assert_eq!(LayerType::Ternary.lut_per_param(), 2);
    }

    #[test]
    fn test_memory_budget() {
        let budget = MemoryBudget::mb_16();
        assert_eq!(budget.target_bytes, 16 * 1024 * 1024);
        assert!(budget.fits(16 * 1024 * 1024));
        assert!(!budget.fits(16 * 1024 * 1024 + 1));
    }

    #[test]
    fn test_dsp_budget() {
        let budget = DspBudget::xc7a100t();
        assert_eq!(budget.total_dsp, 240);
        assert_eq!(budget.max_utilization, 0.95);
        assert_eq!(budget.available_dsp(), 228);  // 240 * 0.95
    }
}
