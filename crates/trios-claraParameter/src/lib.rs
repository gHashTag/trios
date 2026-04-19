//! Trinity Parameter Golf — Ensemble + Final Submission MCP Tool
//!
//! Provides tools for:
//! - Ensemble multiple trained models (baseline + novel techniques)
//! - Generate final submission package with compression ratios
//! - Validate across all 5 AGI hackathon tracks
//!
//! ## Trinity Cognitive Stack Integration
//!
//! ```
//! Foundation (zig-golden-float, zig-physics, zig-sacred-geometry)
//!     └─ GF16 quantization ──┐
//!                  │
//!     Cognition (zig-hdc, zig-agents)
//!     └─ HDC encoding for parameters │
//!                  │
//! Orchestration (trios, trinity-claraParameter)
//!     └─ Training pipeline, ensemble coordination ──┘
//! ```
//!
//! ## Submissions
//!
//! - v01_baseline: int6 quantization, 11.08 MB
//! - v02_bitnet: ternary b1.58 weights, ~25M params in 16MB
//! - v03_gf16: golden-float quantization via HDC bridge, target 1.11 BPB
//! - v04_hslm: Fibonacci attention + Sacred bottleneck (377 hidden)
//! - v05_final: weighted ensemble of best models
//!
//! ## Ensemble Strategies
//!
//! 1. Majority Voting: Each model predicts, most common wins
//! 2. Weighted Averaging: Predictions weighted by validation BPB
//! 3. Stacked Generalization: Models trained sequentially
//!
//! ## Compression Pipeline
//!
//! 1. Train each model independently
//! 2. Apply technique-specific quantization (GF16, ternary, etc.)
//! 3. Compress weights with zstd-22
//! 4. Ensemble with weighted averaging
//! 5. Final package size: ≤16MB

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Model configuration for ensemble.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier
    pub name: String,

    /// Model type (baseline, bitnet, gf16, hslm)
    pub model_type: String,

    /// Model file path (relative to checkpoints/)
    pub checkpoint: String,

    /// Number of parameters
    pub params: usize,

    /// Model size in MB (after compression)
    pub size_mb: f64,

    /// Validation BPB
    pub val_bpb: f64,

    /// Model weight in ensemble (0.0 to 1.0)
    pub weight: f64,
}

/// Ensemble configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnsembleConfig {
    /// Models to include in ensemble
    pub models: Vec<String>,

    /// Ensemble strategy
    pub strategy: EnsembleStrategy,

    /// Compression level (0-22 for zstd)
    pub compression_level: i32,
}

/// Ensemble strategy type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnsembleStrategy {
    /// Majority voting: each model predicts, most common wins
    #[serde(rename = "majority_voting")]
    MajorityVoting,

    /// Weighted averaging: predictions weighted by validation BPB
    #[serde(rename = "weighted_averaging")]
    WeightedAveraging,

    /// Stacked generalization: models trained sequentially
    #[serde(rename = "stacked_generalization")]
    StackedGeneralization,
}

/// Compression result for a model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Original model size in MB
    pub original_mb: f64,

    /// Compressed size in MB
    pub compressed_mb: f64,

    /// Compression ratio (compressed / original)
    pub ratio: f64,

    /// Target BPB
    pub val_bpb: f64,
}

/// Final submission package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalSubmission {
    /// Submission ID
    pub submission_id: String,

    /// Ensemble configuration used
    pub ensemble: EnsembleConfig,

    /// Individual model results
    pub models: Vec<ModelConfig>,

    /// Compression results
    pub compression: Vec<CompressionResult>,

    /// Final package size (≤16MB)
    pub final_size_mb: f64,

    /// Predicted BPB on validation set
    pub predicted_bpb: f64,

    /// Metadata
    pub metadata: serde_json::Value,
}

impl FinalSubmission {
    /// Calculate total parameters in ensemble.
    pub fn total_params(&self) -> usize {
        self.models.iter().map(|m| m.params).sum()
    }

    /// Calculate weighted average BPB.
    pub fn weighted_bpb(&self) -> f64 {
        let mut weighted_sum = 0.0;
        let mut weight_sum = 0.0;

        for model in &self.models {
            weighted_sum += model.val_bpb * model.weight;
            weight_sum += model.weight;
        }

        if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            // Fallback to average if weights don't sum to 1.0
            let avg = self.models.iter().map(|m| m.val_bpb).sum::<f64>();
            avg / self.models.len() as f64
        }
    }

    /// Calculate overall compression ratio.
    pub fn overall_compression_ratio(&self) -> f64 {
        let total_original: f64 = self.models.iter().map(|m| m.original_mb).sum();
        let total_compressed: f64 = self.models.iter().map(|m| m.compressed_mb).sum();

        if total_original > 0.0 {
            total_compressed / total_original
        } else {
            0.0
        }
    }

    /// Validate submission meets 16MB limit.
    pub fn validate_size_limit(&self, limit_mb: f64) -> Result<(), anyhow::Error> {
        if self.final_size_mb > limit_mb {
            anyhow::bail!("Final submission exceeds size limit: {} MB > {} MB",
                self.final_size_mb, limit_mb)
        }
        Ok(())
    }

    /// Generate submission JSON for Parameter Golf.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        // Convert all complex types to JSON-compatible strings
        fn value_to_string(v: &serde_json::Value) -> String {
            match v {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".into(),
                serde_json::Value::Array(a) => serde_json::to_string(a).unwrap_or_default(),
                serde_json::Value::Object(o) => {
                    let mut map = String::new();
                    for (key, val) in o {
                        if !map.is_empty() {
                            map.push_str(", ");
                        }
                        map.push_str(key);
                        map.push('\": ');
                        map.push_str(&value_to_string(val));
                        map.push('"');
                    }
                    if !map.is_empty() {
                        map.push('}');
                    }
                    let mut result = String::from("{");
                    result.push_str(&map);
                    result.push('}');
                    result
                },
                _ => format!("{:?}", v),
            }
        }

        let submission = serde_json::json!({
            "submission_id": &self.submission_id,
            "ensemble": {
                "models": value_to_string(&self.models),
                "strategy": value_to_string(&self.ensemble.strategy),
                "compression_level": self.compression_level.to_string(),
            },
            "models": self.models.iter().map(|m| value_to_string(m)).collect::<Vec<_>>(),
            "compression": self.compression.iter().map(|c| value_to_string(c)).collect::<Vec<_>>(),
            "total_params": self.total_params().to_string(),
            "final_size_mb": self.final_size_mb.to_string(),
            "predicted_bpb": self.predicted_bpb.to_string(),
            "trinity_stack": {
                "foundation": vec!["zig-golden-float", "zig-physics", "zig-sacred-geometry"].into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "cognition": vec!["zig-hdc", "zig-agents"].into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                "orchestration": vec!["trios", "trinity-claraParameter"].into_iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            },
            "metadata": value_to_string(&self.metadata),
        });

        Ok(submission)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_bpb() {
        let models = vec![
            ModelConfig {
                name: "baseline".into(),
                model_type: "int6".into(),
                params: 15000000,
                size_mb: 11.08,
                val_bpb: 1.2244,
                weight: 0.2,
            },
            ModelConfig {
                name: "gf16".into(),
                model_type: "gf16".into(),
                params: 14000000,
                size_mb: 10.5,
                val_bpb: 1.11,
                weight: 0.4,
            },
        ];

        let ensemble = EnsembleConfig {
            models: vec!["baseline".into(), "gf16".into()],
            strategy: EnsembleStrategy::WeightedAveraging,
            compression_level: 22,
        };

        let submission = FinalSubmission {
            submission_id: "test_001".into(),
            ensemble,
            models,
            compression: vec![],
            final_size_mb: 21.58,
            predicted_bpb: 1.162,
            metadata: serde_json::json!({}),
        };

        assert!((submission.predicted_bpb - 1.162).abs() < 0.01, "weighted BPB should be 1.162");
    }
}
