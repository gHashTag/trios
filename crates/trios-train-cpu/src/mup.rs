//! Maximal Update Parametrization (muP) - P2 Lab
//!
//! muP allows hyperparameters found at small scale to transfer verbatim to larger models.
//!
//! Key principles:
//! 1. Width scaling: scale all non-embedding weights by 1/width
//! 2. QK scaling: scale QK projections by 1/sqrt(head_dim)
//! 3. LR scaling: different LR per parameter group (embedding, output, attention)
//!
//! Reference: "μP: Maximal Update Parametrization" (Yang et al., 2022)
//! and Cerebras muP Practitioner Guide.

/// muP scaling configuration
#[derive(Debug, Clone)]
pub struct MupConfig {
    /// Reference model width (where LR was tuned)
    pub ref_width: usize,

    /// Target model width
    pub target_width: usize,

    /// Learning rate multiplier for embeddings
    pub embedding_mult: f64,

    /// Learning rate multiplier for output layer
    pub output_mult: f64,

    /// Learning rate multiplier for attention projections
    pub attn_mult: f64,

    /// Learning rate multiplier for FFN layers
    pub ffn_mult: f64,
}

impl Default for MupConfig {
    fn default() -> Self {
        Self {
            ref_width: 256,  // P1 champion width
            target_width: 256,
            embedding_mult: 1.0,
            output_mult: 1.0,
            attn_mult: 1.0,
            ffn_mult: 1.0,
        }
    }
}

impl MupConfig {
    /// Create muP config for target width
    pub fn for_target(target_width: usize) -> Self {
        let ref_width = 256;
        let width_ratio = (target_width as f64) / (ref_width as f64);

        // Standard muP scaling: LR scales with width^(-1/2) for most layers
        let scale_factor = width_ratio.sqrt().recip();

        Self {
            ref_width,
            target_width,
            // Embeddings: scale with width^(-1) (tied embeddings)
            embedding_mult: width_ratio.recip(),
            // Output: scale with width^(-1)
            output_mult: width_ratio.recip(),
            // Attention: scale with width^(-1/2)
            attn_mult: scale_factor,
            // FFN: scale with width^(-1/2)
            ffn_mult: scale_factor,
        }
    }

    /// Compute LR multiplier for a parameter group
    pub fn lr_multiplier(&self, group: ParamGroup) -> f64 {
        match group {
            ParamGroup::Embedding => self.embedding_mult,
            ParamGroup::Output => self.output_mult,
            ParamGroup::Attention => self.attn_mult,
            ParamGroup::FFN => self.ffn_mult,
            ParamGroup::LayerNorm => 1.0,  // No scaling for layer norm
        }
    }

    /// Validate muP configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.target_width == 0 {
            return Err("target_width must be > 0".to_string());
        }
        if self.ref_width == 0 {
            return Err("ref_width must be > 0".to_string());
        }
        if self.target_width < self.ref_width {
            return Err("muP typically for upscaling, target_width < ref_width".to_string());
        }
        Ok(())
    }
}

/// Parameter groups for per-group LR scaling
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamGroup {
    /// Token embeddings (and tied output)
    Embedding,

    /// Output layer (if untied)
    Output,

    /// Attention QKV/O projections
    Attention,

    /// FFN (W1, W2)
    FFN,

    /// Layer normalization (gamma, beta)
    LayerNorm,
}

/// Apply muP scaling to model dimensions
///
/// Returns the scaled dimensions for the target model.
pub fn scale_dimensions(base_dims: &ModelDims, mup: &MupConfig) -> ModelDims {
    let width_ratio = (mup.target_width as f64) / (mup.ref_width as f64);

    ModelDims {
        d_model: (base_dims.d_model as f64 * width_ratio) as usize,
        n_heads: base_dims.n_heads,  // Heads don't scale
        d_ffn: (base_dims.d_ffn as f64 * width_ratio) as usize,
    }
}

/// QK scaling factor for multi-head attention
///
/// Scales QK projections by 1/sqrt(head_dim) to ensure stable training
/// across widths.
pub fn qk_scaling_factor(head_dim: usize) -> f64 {
    (head_dim as f64).sqrt().recip()
}

/// Model dimensions
#[derive(Debug, Clone)]
pub struct ModelDims {
    pub d_model: usize,
    pub n_heads: usize,
    pub d_ffn: usize,
}

impl ModelDims {
    /// Get head dimension (d_model / n_heads)
    pub fn head_dim(&self) -> usize {
        self.d_model / self.n_heads
    }

    /// Validate dimensions
    pub fn validate(&self) -> Result<(), String> {
        if self.d_model % self.n_heads != 0 {
            return Err(format!(
                "d_model ({}) must be divisible by n_heads ({})",
                self.d_model, self.n_heads
            ));
        }
        if self.d_model == 0 || self.n_heads == 0 || self.d_ffn == 0 {
            return Err("All dimensions must be > 0".to_string());
        }
        Ok(())
    }
}

/// muP-compliant weight initialization
///
/// Scales initial weights to account for width differences.
pub fn mup_weight_scale(
    fan_in: usize,
    fan_out: usize,
    group: ParamGroup,
    mup: &MupConfig,
) -> f64 {
    let base_scale = (2.0 / (fan_in + fan_out) as f64).sqrt();

    // Apply muP scaling based on parameter group
    let lr_mult = mup.lr_multiplier(group);

    // Weights are scaled inversely to LR to maintain update magnitude
    base_scale * lr_mult.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mup_config_default() {
        let cfg = MupConfig::default();
        assert_eq!(cfg.ref_width, 256);
        assert_eq!(cfg.target_width, 256);
    }

    #[test]
    fn test_mup_config_for_target() {
        let cfg = MupConfig::for_target(512);  // 2x width
        assert_eq!(cfg.ref_width, 256);
        assert_eq!(cfg.target_width, 512);
        assert!((cfg.embedding_mult - 0.5).abs() < 1e-6);  // 1/2
        assert!((cfg.attn_mult - 0.7071).abs() < 1e-4);  // 1/sqrt(2)
    }

    #[test]
    fn test_mup_config_validate() {
        let valid = MupConfig::for_target(512);
        assert!(valid.validate().is_ok());

        let invalid = MupConfig {
            ref_width: 256,
            target_width: 128,  // Smaller than ref
            ..Default::default()
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_lr_multiplier() {
        let cfg = MupConfig::for_target(512);
        assert_eq!(cfg.lr_multiplier(ParamGroup::Embedding), 0.5);
        assert_eq!(cfg.lr_multiplier(ParamGroup::LayerNorm), 1.0);
    }

    #[test]
    fn test_scale_dimensions() {
        let base = ModelDims {
            d_model: 256,
            n_heads: 4,
            d_ffn: 1024,
        };
        let mup = MupConfig::for_target(512);
        let scaled = scale_dimensions(&base, &mup);

        assert_eq!(scaled.d_model, 512);
        assert_eq!(scaled.n_heads, 4);  // Heads don't scale
        assert_eq!(scaled.d_ffn, 2048);
    }

    #[test]
    fn test_qk_scaling_factor() {
        let scale_64 = qk_scaling_factor(64);
        let scale_128 = qk_scaling_factor(128);
        assert!((scale_64 - 0.125).abs() < 1e-6);  // 1/8
        assert!((scale_128 - 0.08838).abs() < 1e-4);  // 1/sqrt(128)
    }

    #[test]
    fn test_model_dims_head_dim() {
        let dims = ModelDims {
            d_model: 256,
            n_heads: 4,
            d_ffn: 1024,
        };
        assert_eq!(dims.head_dim(), 64);
    }

    #[test]
    fn test_model_dims_validate() {
        let valid = ModelDims {
            d_model: 256,
            n_heads: 4,
            d_ffn: 1024,
        };
        assert!(valid.validate().is_ok());

        let invalid = ModelDims {
            d_model: 256,
            n_heads: 5,
            d_ffn: 1024,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_mup_weight_scale() {
        let mup = MupConfig::for_target(512);
        let scale = mup_weight_scale(256, 1024, ParamGroup::Attention, &mup);

        // Should be scaled by sqrt(lr_mult)
        let expected = (2.0_f64 / 1280.0_f64).sqrt() * (mup.attn_mult.sqrt());
        assert!((scale - expected).abs() < 1e-6);
    }
}
