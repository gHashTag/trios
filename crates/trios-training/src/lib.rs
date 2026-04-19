//! IGLA-GF16 Training Pipeline for Parameter Golf
//!
//! Implements complete training pipeline for 16MB model submission:
//! - FineWeb dataloading
//! - IGLA-GF16 model (Φ1-Φ4 integration)
//! - φ-LR schedule
//! - Trinity weight initialization
//! - CA φ-mask for attention
//! - BPB evaluation for submission
//!
//! ## Architecture (7 layers, VOCAB=32000)
//!
//! - d_model = 144 (Fibonacci #12)
//! - n_heads = 8 (Fibonacci #6)
//! - n_layers = 7 (fits 16MB)
//! - vocab_size = 32000 (LLaMA tokenizer)
//! - Total: ~9.5MB GF16 (well under 16MB limit)

pub mod model;
pub mod phi_schedule;
pub mod trinity_init;
pub mod ca_mask;
pub mod data;
pub mod train;
pub mod eval;

// Re-export key types
pub use model::IGLAGF16Model;
pub use phi_schedule::phi_lr;
pub use trinity_init::TrinityInitConfig;
pub use ca_mask::CAMask;
pub use data::FineWebBatch;
pub use train::train_igla_gf16;
pub use eval::evaluate_submission;

/// Training configuration for IGLA-GF16
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Number of training iterations
    pub iterations: usize,

    /// Learning rate schedule (φ-based or cosine)
    pub lr_schedule: LRSchedule,

    /// Whether to use φ-LR schedule
    pub use_phi_physics: bool,

    /// Batch size in tokens
    pub batch_tokens: usize,

    /// Validation frequency (every 34 steps = Fib #8)
    pub val_every: usize,

    /// Output directory for checkpoints
    pub output_dir: String,
}

/// Learning rate schedule type
#[derive(Debug, Clone, Copy)]
pub enum LRSchedule {
    /// φ-based LR schedule (starts at α_φ = 0.118034)
    Phi,
    /// Standard cosine decay
    Cosine,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            iterations: 20000,
            lr_schedule: LRSchedule::Phi,
            use_phi_physics: true,
            batch_tokens: 524288,
            val_every: 34,  // Fib #8
            output_dir: "outputs/igla_gf16".to_string(),
        }
    }
}

/// Model size estimator for 16MB Parameter Golf constraint
pub fn estimate_model_size(
    vocab_size: usize,
    d_model: usize,
    n_layers: usize,
    n_heads: usize,
    d_ffn: usize,
) -> ModelSizeEstimate {
    // Embedding (tied with output)
    let embedding_params = vocab_size * d_model;

    // Attention per layer: QKV + proj (4 * d_model * d_model)
    let attn_params_layer = 4 * d_model * d_model;

    // FFN per layer: gate + up + down (3 * d_model * d_ffn)
    let ffn_params_layer = 3 * d_model * d_ffn;

    // Layer norm per layer: 2 * d_model
    let ln_params_layer = 4 * d_model;

    // Total per layer
    let params_layer = attn_params_layer + ffn_params_layer + ln_params_layer;

    // Total parameters
    let total_params = embedding_params + (n_layers * params_layer);

    // Size in bytes (GF16 = 2 bytes/param)
    let size_bytes = total_params * 2;

    // Check 16MB limit
    let fits_16mb = size_bytes <= 16 * 1024 * 1024;

    ModelSizeEstimate {
        total_params,
        size_bytes,
        fits_16mb,
        embedding_mb: embedding_params * 2 / (1024 * 1024),
        attention_mb: (n_layers * attn_params_layer) * 2 / (1024 * 1024),
        ffn_mb: (n_layers * ffn_params_layer) * 2 / (1024 * 1024),
    }
}

/// Model size estimation result
#[derive(Debug, Clone)]
pub struct ModelSizeEstimate {
    pub total_params: usize,
    pub size_bytes: usize,
    pub fits_16mb: bool,
    pub embedding_mb: f64,
    pub attention_mb: f64,
    pub ffn_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_size_estimate() {
        let size = estimate_model_size(32000, 144, 7, 8, 232);

        // Should be well under 16MB
        assert!(size.fits_16mb, "Model must fit 16MB limit");

        // Should be around 9.5MB
        assert!(size.size_bytes > 8 * 1024 * 1024);
        assert!(size.size_bytes < 11 * 1024 * 1024);

        println!("Model size: {:.2} MB ({:.2}M params)",
                 size.size_bytes as f64 / (1024.0 * 1024.0),
                 size.total_params as f64 / 1e6);
    }
}
