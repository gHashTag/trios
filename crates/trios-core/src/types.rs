//! # TRIOS Core Types (Φ0 Foundation)
//!
//! This module defines the core data structures used across the TRIOS project.
//! These types serve as the Single Source of Truth (SSOT) for the hybrid
//! precision quantization pipeline.
//!
//! ## Schema Version
//!
//! This file corresponds to `experience/Φ0/schema.trinity`.
//!
//! ## Key Types
//!
//! - `PrecisionFormat`: GF16, Ternary158, FP32
//! - `LayerSpec`: Layer specification with sensitivity and dimensions
//! - `MemoryBudget`: 16MB Parameter Golf constraint
//! - `DspBudget`: XC7A100T FPGA constraints (240 DSP)
//! - `FpgaTarget`: Hardware platform specifications

use serde::{Deserialize, Serialize};
use std::{fmt, path::PathBuf};

// ==============================================================================
// GIT TYPES
// ==============================================================================

/// File status in git repository.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Staged,
    Conflicted,
}

/// File change information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub status: FileStatus,
    pub staged: bool,
}

/// Result of a commit operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub oid: String,
    pub message: String,
    pub files_committed: usize,
}

/// Branch information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub upstream: Option<String>,
    pub commit_count: Option<usize>,
}

/// Git log entry.
/// GitButler virtual branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GbBranch {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub files_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

/// Diff result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub files: Vec<String>,
    pub patch: String,
}

// ==============================================================================
// PRECISION FORMATS
// ==============================================================================

/// Precision format for neural network weights and activations.
///
/// From BENCH-004b: GF16 achieves 97.67% accuracy (0.00% gap vs f32),
/// while BF16 and naive ternary degrade to 9.80%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrecisionFormat {
    /// GoldenFloat16 — φ-optimized 16-bit floating point
    ///
    /// Format: [sign:1][exp:6][mantissa:9]
    /// - Parity with f32: 97.67% on MNIST MLP, 0.00% gap
    /// - DSP cost: 16 per MAC-16 unit
    /// - LUT cost: 71 per MAC-16 unit
    GF16,

    /// Ternary158 — 1.58-bit quantized ({-1, 0, +1} with learned scaling)
    ///
    /// Format: log2(3) ≈ 1.58 bits per weight
    /// - Requires QAT (Quantization-Aware Training) with STE
    /// - DSP cost: 0 per MAC-16 unit (pure LUT)
    /// - LUT cost: 52 per MAC-16 unit
    Ternary158,

    /// IEEE 754 FP32 — 32-bit floating point (baseline)
    ///
    /// Used for accuracy reference and training.
    FP32,
}

impl PrecisionFormat {
    /// Returns the bit-width of this precision.
    pub const fn bit_width(&self) -> u32 {
        match self {
            Self::GF16 => 16,
            Self::Ternary158 => 2,  // Effective 1.58 bits
            Self::FP32 => 32,
        }
    }

    /// Returns whether this format uses floating-point representation.
    pub const fn is_floating_point(&self) -> bool {
        matches!(self, Self::GF16 | Self::FP32)
    }

    /// Returns the compression ratio vs f32 (32 bits/param).
    pub fn compression_ratio(&self) -> f32 {
        32.0 / self.bit_width() as f32
    }

    /// Returns the memory size in bytes for a given parameter count.
    pub fn memory_bytes(&self, param_count: usize) -> usize {
        (param_count * self.bit_width() as usize).div_ceil(8)
    }
}

impl fmt::Display for PrecisionFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GF16 => write!(f, "GF16"),
            Self::Ternary158 => write!(f, "Ternary158"),
            Self::FP32 => write!(f, "FP32"),
        }
    }
}

// ==============================================================================
// SENSITIVITY LEVELS
// ==============================================================================

/// Quantization sensitivity determines precision requirements.
///
/// - HIGH: Requires GF16 for f32 parity (embedding, output)
/// - MEDIUM: Can use ternary with QAT, but may lose some accuracy
/// - LOW: Safe for ternary even without QAT (activation functions)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Sensitivity {
    /// Critical: requires GF16 for f32 parity
    HIGH = 2,

    /// Medium: can use ternary with QAT
    MEDIUM = 1,

    /// Low: safe for ternary even without QAT
    LOW = 0,
}

impl Sensitivity {
    /// Returns a numeric value for comparison.
    pub const fn value(&self) -> u8 {
        *self as u8
    }
}

// ==============================================================================
// LAYER TYPES
// ==============================================================================

/// Neural network layer types with associated sensitivity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerType {
    /// Input embedding layer (HIGH sensitivity)
    Embedding,

    /// Multi-head attention (HIGH sensitivity)
    Attention,

    /// Feed-forward / linear layer (sensitivity varies by depth)
    Dense,

    /// 2D convolutional layer (early = LOW, deep = MEDIUM)
    Conv2D,

    /// Layer normalization (MEDIUM sensitivity)
    LayerNorm,

    /// Output classification head (HIGH sensitivity)
    OutputHead,

    /// Activation function (MEDIUM sensitivity)
    Activation,
}

impl LayerType {
    /// Returns the base sensitivity level for this layer type.
    pub const fn base_sensitivity(&self) -> Sensitivity {
        match self {
            Self::Embedding => Sensitivity::HIGH,
            Self::Attention => Sensitivity::HIGH,
            Self::OutputHead => Sensitivity::HIGH,
            Self::Dense => Sensitivity::LOW,
            Self::Conv2D => Sensitivity::LOW,
            Self::LayerNorm => Sensitivity::MEDIUM,
            Self::Activation => Sensitivity::MEDIUM,
        }
    }

    /// Returns the default precision format for this layer type.
    pub const fn default_precision(&self) -> PrecisionFormat {
        match self {
            Self::Embedding => PrecisionFormat::GF16,
            Self::Attention => PrecisionFormat::GF16,
            Self::OutputHead => PrecisionFormat::GF16,
            Self::LayerNorm => PrecisionFormat::GF16,
            Self::Dense => PrecisionFormat::Ternary158,
            Self::Conv2D => PrecisionFormat::Ternary158,
            Self::Activation => PrecisionFormat::Ternary158,
        }
    }
}

impl fmt::Display for LayerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Embedding => write!(f, "Embedding"),
            Self::Attention => write!(f, "Attention"),
            Self::Dense => write!(f, "Dense"),
            Self::Conv2D => write!(f, "Conv2D"),
            Self::LayerNorm => write!(f, "LayerNorm"),
            Self::OutputHead => write!(f, "OutputHead"),
            Self::Activation => write!(f, "Activation"),
        }
    }
}

// ==============================================================================
// LAYER SPECIFICATION
// ==============================================================================

/// Layer specification for precision planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerSpec {
    /// Unique identifier for the layer
    pub name: String,

    /// Type of layer (affects sensitivity)
    pub layer_type: LayerType,

    /// Number of input features
    pub in_features: usize,

    /// Number of output features
    pub out_features: usize,

    /// Sensitivity to quantization
    pub sensitivity: Sensitivity,

    /// Total parameter count (in_features * out_features + bias if any)
    pub param_count: usize,

    /// Optional: layer depth (for deep networks)
    pub depth: Option<usize>,
}

impl LayerSpec {
    /// Create a new layer specification.
    pub fn new(
        name: impl Into<String>,
        layer_type: LayerType,
        in_features: usize,
        out_features: usize,
    ) -> Self {
        let name = name.into();
        let sensitivity = layer_type.base_sensitivity();
        let param_count = in_features * out_features;

        Self {
            name,
            layer_type,
            in_features,
            out_features,
            sensitivity,
            param_count,
            depth: None,
        }
    }

    /// Create a layer specification with custom sensitivity.
    pub fn with_sensitivity(
        name: impl Into<String>,
        layer_type: LayerType,
        in_features: usize,
        out_features: usize,
        sensitivity: Sensitivity,
    ) -> Self {
        let name = name.into();
        let param_count = in_features * out_features;

        Self {
            name,
            layer_type,
            in_features,
            out_features,
            sensitivity,
            param_count,
            depth: None,
        }
    }

    /// Set the layer depth.
    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = Some(depth);
        self
    }

    /// Estimate memory usage in MB for a given precision.
    pub fn memory_mb(&self, precision: PrecisionFormat) -> f64 {
        let bytes = precision.memory_bytes(self.param_count);
        bytes as f64 / (1024.0 * 1024.0)
    }
}

// ==============================================================================
// HARDWARE COST MODELS
// ==============================================================================

/// Hardware cost per parameter for a given precision.
///
/// From BENCH-005 and BENCH-006:
/// - GF16: 71 LUT, 266 FF, 16 DSP, 549 cells per MAC-16
/// - Ternary: 52 LUT, 69 FF, 0 DSP, 71 cells per MAC-16
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HardwareCost {
    /// LUT units per parameter
    pub lut_per_param: u32,

    /// DSP units per parameter
    pub dsp_per_param: u32,

    /// Flip-flops per parameter
    pub ff_per_param: u32,

    /// Total cells per parameter
    pub cells_per_param: u32,
}

impl HardwareCost {
    /// Get cost for GF16 precision.
    pub const fn gf16() -> Self {
        Self {
            lut_per_param: 71,
            dsp_per_param: 16,
            ff_per_param: 266,
            cells_per_param: 549,
        }
    }

    /// Get cost for Ternary precision.
    pub const fn ternary() -> Self {
        Self {
            lut_per_param: 52,
            dsp_per_param: 0,
            ff_per_param: 69,
            cells_per_param: 71,
        }
    }

    /// Get cost by precision format.
    pub const fn for_precision(precision: PrecisionFormat) -> Self {
        match precision {
            PrecisionFormat::GF16 => Self::gf16(),
            PrecisionFormat::Ternary158 => Self::ternary(),
            PrecisionFormat::FP32 => Self {
                lut_per_param: 94,  // GF16 mul cost (estimate)
                dsp_per_param: 1,
                ff_per_param: 47,
                cells_per_param: 142,
            },
        }
    }
}

// ==============================================================================
// MEMORY BUDGET
// ==============================================================================

/// Memory budget for 16MB Parameter Golf constraint.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryBudget {
    /// Target size in bytes (16MB = 16,777,216 bytes)
    pub target_bytes: u32,

    /// Maximum compression ratio allowed
    pub max_compression_ratio: f32,
}

impl MemoryBudget {
    /// Create 16MB memory budget (Parameter Golf constraint).
    pub const fn mb_16() -> Self {
        Self {
            target_bytes: 16 * 1024 * 1024,
            max_compression_ratio: 4.0,
        }
    }

    /// Create a custom memory budget.
    pub const fn new(target_bytes: u32, max_compression_ratio: f32) -> Self {
        Self {
            target_bytes,
            max_compression_ratio,
        }
    }

    /// Check if model size fits within budget.
    pub fn fits(&self, model_size_bytes: u32) -> bool {
        model_size_bytes <= self.target_bytes
    }

    /// Calculate remaining memory in bytes.
    pub fn remaining(&self, used_bytes: u32) -> u32 {
        self.target_bytes.saturating_sub(used_bytes)
    }

    /// Calculate remaining memory in MB.
    pub fn remaining_mb(&self, used_bytes: u32) -> f64 {
        self.remaining(used_bytes) as f64 / (1024.0 * 1024.0)
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::mb_16()
    }
}

// ==============================================================================
// DSP BUDGET
// ==============================================================================

/// DSP budget for FPGA deployment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DspBudget {
    /// Total DSP units available
    pub total_dsp: u32,

    /// Maximum DSP utilization percentage (0.0-1.0)
    pub max_utilization: f32,

    /// FPGA target identifier
    pub target: &'static str,
}

impl DspBudget {
    /// Create XC7A100T DSP budget.
    pub const fn xc7a100t() -> Self {
        Self {
            total_dsp: 240,
            max_utilization: 0.95,
            target: "XC7A100T-FGG676",
        }
    }

    /// Create a custom DSP budget.
    pub const fn new(total_dsp: u32, max_utilization: f32, target: &'static str) -> Self {
        Self {
            total_dsp,
            max_utilization,
            target,
        }
    }

    /// Get available DSP units.
    pub const fn available_dsp(&self) -> u32 {
        (self.total_dsp as f32 * self.max_utilization) as u32
    }

    /// Calculate utilization percentage.
    pub fn utilization(&self, used_dsp: u32) -> f32 {
        (used_dsp as f32 / self.total_dsp as f32) * 100.0
    }

    /// Check if DSP allocation fits within budget.
    pub fn fits(&self, dsp_cost: u32) -> bool {
        dsp_cost <= self.available_dsp()
    }

    /// Calculate remaining DSP units.
    pub const fn remaining(&self, used_dsp: u32) -> u32 {
        self.available_dsp().saturating_sub(used_dsp)
    }
}

impl Default for DspBudget {
    fn default() -> Self {
        Self::xc7a100t()
    }
}

// ==============================================================================
// LUT BUDGET
// ==============================================================================

/// LUT budget for FPGA deployment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LutBudget {
    /// Total LUT available
    pub total_lut: u32,

    /// Maximum LUT utilization percentage (0.0-1.0)
    pub max_utilization: f32,
}

impl LutBudget {
    /// Create XC7A100T LUT budget.
    pub const fn xc7a100t() -> Self {
        Self {
            total_lut: 63_400,
            max_utilization: 0.80,
        }
    }

    /// Create a custom LUT budget.
    pub const fn new(total_lut: u32, max_utilization: f32) -> Self {
        Self {
            total_lut,
            max_utilization,
        }
    }

    /// Get available LUT units.
    pub const fn available_lut(&self) -> u32 {
        (self.total_lut as f32 * self.max_utilization) as u32
    }

    /// Calculate utilization percentage.
    pub fn utilization(&self, used_lut: u32) -> f32 {
        (used_lut as f32 / self.total_lut as f32) * 100.0
    }

    /// Check if LUT allocation fits within budget.
    pub fn fits(&self, lut_cost: u32) -> bool {
        lut_cost <= self.available_lut()
    }
}

impl Default for LutBudget {
    fn default() -> Self {
        Self::xc7a100t()
    }
}

// ==============================================================================
// FPGA TARGET
// ==============================================================================

/// FPGA platform specifications.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FpgaTarget {
    /// Target name
    pub name: &'static str,

    /// Total LUT available
    pub total_lut: u32,

    /// Total DSP available
    pub total_dsp: u32,

    /// Total flip-flops
    pub total_ff: u32,

    /// Total block RAM
    pub total_bram: u32,

    /// Maximum operating frequency in MHz
    pub max_freq_mhz: f32,
}

impl FpgaTarget {
    /// XC7A100T-FGG676 specifications.
    pub const fn xc7a100t() -> Self {
        Self {
            name: "XC7A100T-FGG676",
            total_lut: 63_400,
            total_dsp: 240,
            total_ff: 126_800,
            total_bram: 135,
            max_freq_mhz: 100.0,
        }
    }
}

impl Default for FpgaTarget {
    fn default() -> Self {
        Self::xc7a100t()
    }
}

// ==============================================================================
// SUCCESS METRICS
// ==============================================================================

/// Success metrics from Master Plan §2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessMetrics {
    /// Accuracy on MNIST MLP (>= 97.67%)
    pub accuracy_mnist_mlp: f32,

    /// Accuracy gap vs f32 (<= 0.10%)
    pub accuracy_gap_vs_f32: f32,

    /// Total memory in MB (<= 16.00)
    pub memory_total_mb: f32,

    /// DSP utilization (>= 0.95)
    pub dsp_utilization: f32,

    /// Throughput in GOPS (>= 1000)
    pub throughput_gops: u32,

    /// JEPA collapse detection (should be false)
    pub jepa_collapse_detected: bool,

    /// Coq proofs with Admitted (should be 0)
    pub coq_proofs_admitted: u32,

    /// Merkle root hash
    pub merkle_root: String,

    /// Reproducibility flag
    pub reproducibility: bool,
}

impl SuccessMetrics {
    /// Create default target metrics.
    pub fn targets() -> Self {
        Self {
            accuracy_mnist_mlp: 97.67,
            accuracy_gap_vs_f32: 0.10,
            memory_total_mb: 16.00,
            dsp_utilization: 0.95,
            throughput_gops: 1000,
            jepa_collapse_detected: false,
            coq_proofs_admitted: 0,
            merkle_root: String::new(),
            reproducibility: true,
        }
    }

    /// Check if all metrics meet targets.
    pub fn all_pass(&self) -> bool {
        self.accuracy_mnist_mlp >= 97.67
            && self.accuracy_gap_vs_f32 <= 0.10
            && self.memory_total_mb <= 16.00
            && self.dsp_utilization >= 0.95
            && self.throughput_gops >= 1000
            && !self.jepa_collapse_detected
            && self.coq_proofs_admitted == 0
            && self.reproducibility
    }
}

impl Default for SuccessMetrics {
    fn default() -> Self {
        Self::targets()
    }
}

// ==============================================================================
// MODEL SPECIFICATION
// ==============================================================================

/// Complete model specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Model identifier
    pub name: String,

    /// Model version
    pub version: String,

    /// All layers in the model
    pub layers: Vec<LayerSpec>,

    /// Total parameter count
    pub total_params: usize,

    /// Total memory in MB (f32 baseline)
    pub total_memory_mb: f32,

    /// Architecture type
    pub architecture: String,
}

impl ModelSpec {
    /// Create a new model specification.
    pub fn new(name: impl Into<String>, architecture: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "0.1.0".to_string(),
            layers: Vec::new(),
            total_params: 0,
            total_memory_mb: 0.0,
            architecture: architecture.into(),
        }
    }

    /// Add a layer to the model.
    pub fn add_layer(mut self, layer: LayerSpec) -> Self {
        self.total_params += layer.param_count;
        self.layers.push(layer);
        self
    }

    /// Calculate total memory in MB for f32 baseline.
    pub fn calculate_memory(&mut self) -> &mut Self {
        self.total_memory_mb = (self.total_params * 4) as f32 / (1024.0 * 1024.0);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precision_format_bit_width() {
        assert_eq!(PrecisionFormat::GF16.bit_width(), 16);
        assert_eq!(PrecisionFormat::Ternary158.bit_width(), 2);
        assert_eq!(PrecisionFormat::FP32.bit_width(), 32);
    }

    #[test]
    fn test_precision_format_compression() {
        assert!((PrecisionFormat::GF16.compression_ratio() - 2.0).abs() < 0.01);
        assert!((PrecisionFormat::Ternary158.compression_ratio() - 16.0).abs() < 1.0);
        assert_eq!(PrecisionFormat::FP32.compression_ratio(), 1.0);
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
        assert_eq!(budget.available_dsp(), 228); // 240 * 0.95
    }

    #[test]
    fn test_lut_budget() {
        let budget = LutBudget::xc7a100t();
        assert_eq!(budget.total_lut, 63_400);
        assert_eq!(budget.max_utilization, 0.80);
    }

    #[test]
    fn test_hardware_cost() {
        let gf16_cost = HardwareCost::gf16();
        assert_eq!(gf16_cost.lut_per_param, 71);
        assert_eq!(gf16_cost.dsp_per_param, 16);

        let ternary_cost = HardwareCost::ternary();
        assert_eq!(ternary_cost.lut_per_param, 52);
        assert_eq!(ternary_cost.dsp_per_param, 0);
    }

    #[test]
    fn test_layer_spec() {
        let layer = LayerSpec::new("embedding", LayerType::Embedding, 784, 144);
        assert_eq!(layer.sensitivity, Sensitivity::HIGH);
        assert_eq!(layer.param_count, 784 * 144);
        assert_eq!(layer.memory_mb(PrecisionFormat::GF16), (784 * 144 * 2) as f64 / (1024.0 * 1024.0));
    }

    #[test]
    fn test_success_metrics() {
        let metrics = SuccessMetrics::targets();
        assert!(metrics.all_pass());

        let mut failed = metrics.clone();
        failed.accuracy_mnist_mlp = 90.0;
        assert!(!failed.all_pass());
    }

    #[test]
    fn test_model_spec() {
        let mut spec = ModelSpec::new("test_mlp", "MLP")
            .add_layer(LayerSpec::new("dense1", LayerType::Dense, 784, 144))
            .add_layer(LayerSpec::new("dense2", LayerType::Dense, 144, 10));
        spec.calculate_memory();

        assert_eq!(spec.layers.len(), 2);
        assert_eq!(spec.total_params, 784 * 144 + 144 * 10);
        assert!(spec.total_memory_mb > 0.0);
    }
}
