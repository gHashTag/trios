//! # trios-model (M)
//!
//! IGLA-GF16 Model Architecture (Φ5.1)
//!
//! Implements the hybrid precision model architecture combining GF16 for
//! critical layers and Ternary for bulk compute layers.
//!
//! ## Symbol
//!
//! M — Model Architecture
//!
//! ## Phase
//!
//! Φ5.1 — IGLA-GF16 Architecture
//!
//! ## Precision Routing
//!
//! Per STATIC_ROUTING_TABLE:
//! - Embedding → GF16 (critical, representation learning)
//! - Attention QKV → GF16 (critical, attention fragility)
//! - FFN gate → Ternary (bulk, zero-DSP)
//! - FFN up → Ternary (bulk, massive expansion)
//! - FFN down → GF16 (critical, projection precision)
//!
//! ## Example
//!
//! ```ignore
//! use trios_model::IglaModel;
//!
//! let model = IglaModel::new();
//! let output = model.forward(&[1, 2, 3, 4]);
//! ```

use trios_golden_float::GF16;
use trios_tri::Ternary;

/// IGLA-GF16 Model Architecture
///
/// Hybrid precision model combining GF16 and Ternary for optimal
/// accuracy vs efficiency tradeoff.
///
/// # Fields
///
/// - `embedding`: GF16 quantized embedding weights
/// - `attention_qkv`: GF16 quantized QKV projection weights
/// - `ffn_gate`: Ternary quantized FFN gate weights
/// - `ffn_up`: Ternary quantized FFN up-projection weights
/// - `ffn_down`: GF16 quantized FFN down-projection weights
#[derive(Debug, Clone)]
pub struct IglaModel {
    /// GF16 quantized embedding layer weights
    pub embedding: Vec<GF16>,
    /// GF16 quantized attention QKV weights
    pub attention_qkv: Vec<GF16>,
    /// Ternary quantized FFN gate weights
    pub ffn_gate: Vec<Ternary>,
    /// Ternary quantized FFN up weights
    pub ffn_up: Vec<Ternary>,
    /// GF16 quantized FFN down weights
    pub ffn_down: Vec<GF16>,
}

impl IglaModel {
    /// Create a new empty IGLA-GF16 model.
    ///
    /// All weight vectors are initialized empty. Use `load_weights()`
    /// to populate the model with quantized weights.
    ///
    /// # Example
    ///
    /// ```
    /// use trios_model::IglaModel;
    ///
    /// let model = IglaModel::new();
    /// assert!(model.embedding.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            embedding: vec![],
            attention_qkv: vec![],
            ffn_gate: vec![],
            ffn_up: vec![],
            ffn_down: vec![],
        }
    }

    /// Run forward pass through the model.
    ///
    /// # Arguments
    ///
    /// * `input` - Input token IDs (u32)
    ///
    /// # Returns
    ///
    /// Vector of f32 logits (stub implementation)
    ///
    /// # Example
    ///
    /// ```
    /// use trios_model::IglaModel;
    ///
    /// let model = IglaModel::new();
    /// let output = model.forward(&[1, 2, 3]);
    /// // Stub returns empty for now
    /// ```
    pub fn forward(&self, _input: &[u32]) -> Vec<f32> {
        // Stub forward pass — full implementation requires:
        // 1. Embedding lookup (GF16)
        // 2. Attention layers (GF16 QKV)
        // 3. FFN layers (Ternary gate/up, GF16 down)
        // 4. Output projection (GF16)
        vec![]
    }

    /// Load quantized weights into the model.
    ///
    /// # Arguments
    ///
    /// * `embedding` - GF16 quantized embedding weights
    /// * `attention_qkv` - GF16 quantized QKV weights
    /// * `ffn_gate` - Ternary quantized gate weights
    /// * `ffn_up` - Ternary quantized up weights
    /// * `ffn_down` - GF16 quantized down weights
    ///
    /// # Example
    ///
    /// ```
    /// use trios_model::IglaModel;
    /// use trios_golden_float::GF16;
    /// use trios_tri::Ternary;
    ///
    /// let mut model = IglaModel::new();
    /// model.load_weights(
    ///     vec![GF16::from_f32(1.0)],
    ///     vec![GF16::from_f32(1.0)],
    ///     vec![Ternary::PosOne],
    ///     vec![Ternary::PosOne],
    ///     vec![GF16::from_f32(1.0)],
    /// );
    /// assert_eq!(model.embedding.len(), 1);
    /// ```
    pub fn load_weights(
        &mut self,
        embedding: Vec<GF16>,
        attention_qkv: Vec<GF16>,
        ffn_gate: Vec<Ternary>,
        ffn_up: Vec<Ternary>,
        ffn_down: Vec<GF16>,
    ) {
        self.embedding = embedding;
        self.attention_qkv = attention_qkv;
        self.ffn_gate = ffn_gate;
        self.ffn_up = ffn_up;
        self.ffn_down = ffn_down;
    }

    /// Get the total number of parameters in the model.
    ///
    /// # Returns
    ///
    /// Total parameter count across all layers
    ///
    /// # Example
    ///
    /// ```
    /// use trios_model::IglaModel;
    ///
    /// let model = IglaModel::new();
    /// assert_eq!(model.num_parameters(), 0);
    /// ```
    pub fn num_parameters(&self) -> usize {
        self.embedding.len()
            + self.attention_qkv.len()
            + self.ffn_gate.len()
            + self.ffn_up.len()
            + self.ffn_down.len()
    }

    /// Calculate model size in bytes based on quantization.
    ///
    /// GF16: 2 bytes per parameter
    /// Ternary: ~0.2 bytes per parameter (1.58 bits)
    ///
    /// # Returns
    ///
    /// Total model size in bytes
    ///
    /// # Example
    ///
    /// ```
    /// use trios_model::IglaModel;
    /// use trios_golden_float::GF16;
    /// use trios_tri::Ternary;
    ///
    /// let mut model = IglaModel::new();
    /// model.load_weights(
    ///     vec![GF16::from_f32(1.0); 100],
    ///     vec![GF16::from_f32(1.0); 100],
    ///     vec![Ternary::PosOne; 100],
    ///     vec![Ternary::PosOne; 100],
    ///     vec![GF16::from_f32(1.0); 100],
    /// );
    /// let size = model.size_bytes();
    /// assert!(size > 600 && size < 800); // 400 GF16 (800 bytes) + 200 Ternary (~40 bytes)
    /// ```
    pub fn size_bytes(&self) -> usize {
        let gf16_bytes = (self.embedding.len() + self.attention_qkv.len() + self.ffn_down.len()) * 2;
        let ternary_bytes = (self.ffn_gate.len() + self.ffn_up.len()) / 5;
        gf16_bytes + ternary_bytes
    }
}

impl Default for IglaModel {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// TESTS
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_model() {
        let model = IglaModel::new();
        assert!(model.embedding.is_empty());
        assert!(model.attention_qkv.is_empty());
        assert!(model.ffn_gate.is_empty());
        assert!(model.ffn_up.is_empty());
        assert!(model.ffn_down.is_empty());
    }

    #[test]
    fn test_load_weights() {
        let mut model = IglaModel::new();
        model.load_weights(
            vec![GF16::from_f32(1.0)],
            vec![GF16::from_f32(1.0)],
            vec![Ternary::PosOne],
            vec![Ternary::PosOne],
            vec![GF16::from_f32(1.0)],
        );

        assert_eq!(model.embedding.len(), 1);
        assert_eq!(model.attention_qkv.len(), 1);
        assert_eq!(model.ffn_gate.len(), 1);
        assert_eq!(model.ffn_up.len(), 1);
        assert_eq!(model.ffn_down.len(), 1);
    }

    #[test]
    fn test_forward() {
        let model = IglaModel::new();
        let output = model.forward(&[1, 2, 3]);
        assert!(output.is_empty()); // Stub returns empty
    }

    #[test]
    fn test_num_parameters() {
        let mut model = IglaModel::new();
        model.load_weights(
            vec![GF16::from_f32(1.0); 100],
            vec![GF16::from_f32(1.0); 200],
            vec![Ternary::PosOne; 300],
            vec![Ternary::PosOne; 400],
            vec![GF16::from_f32(1.0); 500],
        );

        assert_eq!(model.num_parameters(), 1500);
    }

    #[test]
    fn test_size_bytes() {
        let mut model = IglaModel::new();
        model.load_weights(
            vec![GF16::from_f32(1.0); 100],
            vec![GF16::from_f32(1.0); 100],
            vec![Ternary::PosOne; 100],
            vec![Ternary::PosOne; 100],
            vec![GF16::from_f32(1.0); 100],
        );

        let size = model.size_bytes();
        // 300 GF16 params * 2 bytes = 600 bytes
        // 200 Ternary params / 5 = 40 bytes
        assert_eq!(size, 640);
    }

    #[test]
    fn test_default() {
        let model = IglaModel::default();
        assert_eq!(model.num_parameters(), 0);
    }

    #[test]
    fn test_hybrid_precision_counts() {
        let mut model = IglaModel::new();
        model.load_weights(
            vec![GF16::from_f32(1.0); 10],
            vec![GF16::from_f32(1.0); 20],
            vec![Ternary::PosOne; 30],
            vec![Ternary::PosOne; 40],
            vec![GF16::from_f32(1.0); 50],
        );

        // GF16: embedding + attention_qkv + ffn_down
        let gf16_count = model.embedding.len() + model.attention_qkv.len() + model.ffn_down.len();
        assert_eq!(gf16_count, 80);

        // Ternary: ffn_gate + ffn_up
        let ternary_count = model.ffn_gate.len() + model.ffn_up.len();
        assert_eq!(ternary_count, 70);
    }
}
