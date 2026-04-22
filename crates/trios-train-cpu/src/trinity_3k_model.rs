//! Trinity 3k Model for Parameter Golf #110 - Pure Rust Implementation
//!
//! Byte-level Trinity 3^k transformer:
//! - vocab_size: 729 (3^6)
//! - hidden_dim: 243 (3^5)  
//! - n_heads: 27 (3^3)
//! - head_dim: 9 (3^2)
//! - activation: ReLU^2
//! - normalization: QK-Norm + LayerNorm

use std::f32::consts::LN_2;

// ── helpers ──────────────────────────────────────────────────────────────────

/// ReLU^2 activation: max(0, x)^2
#[inline]
fn relu_squared(x: f32) -> f32 {
    let relu_x = x.max(0.0);
    relu_x * relu_x
}

/// Numerically-stable softmax (in-place).
fn softmax(v: &mut [f32]) {
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }
    for x in v.iter_mut() {
        *x /= sum;
    }
}

/// Layer norm: (x - μ) / (σ + ε) * γ  (γ = ones, no learned bias)
fn layer_norm(x: &[f32], eps: f32) -> Vec<f32> {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + eps).sqrt();
    
    x.iter().map(|v| (v - mean) / std).collect()
}
fn matvec(a: &[f32], rows: usize, cols: usize, v: &[f32]) -> Vec<f32> {
    assert_eq!(a.len(), rows * cols);
    assert_eq!(v.len(), cols);
    (0..rows)
        .map(|r| {
            let row = &a[r * cols..(r + 1) * cols];
            row.iter().zip(v.iter()).map(|(w, x)| w * x).sum()
        })
        .collect()
}

/// Left matrix-vector multiply: v (1 × cols) · A (rows × cols) → out (1 × rows)
fn left_matvec(a: &[f32], rows: usize, cols: usize, v: &[f32]) -> Vec<f32> {
    assert_eq!(a.len(), rows * cols);
    assert_eq!(v.len(), cols);
    (0..rows)
        .map(|r| {
            let row_start = r * cols;
            let row_end = row_start + cols;
            let row = &a[row_start..row_end];
            v.iter().zip(row.iter()).map(|(&x, &w)| x * w).sum()
        })
        .collect()
}


/// Xavier initialization with phi-based depth scaling
fn xavier_phi_init(size: usize, fan_in: usize, fan_out: usize, layer_idx: usize, total_layers: usize, seed: &mut u64) -> Vec<f32> {
    let phi: f64 = 1.618033988749895; // Golden ratio
    let phi_scale = phi.powf(-(layer_idx as f64 / total_layers as f64));
    
    // Standard Xavier bound
    let std = (2.0 / (fan_in + fan_out) as f32).sqrt() * phi_scale as f32;
    
    let mut rng = *seed;
    let mut weights = Vec::with_capacity(size);
    
    for _ in 0..size {
        // Simple LCG for deterministic initialization
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        let uniform = (rng & 0x7fffffff) as f32 / 2147483648.0; // [0, 1)
        let normal = (uniform - 0.5) * 2.0 * std * 3.0; // approximate normal
        weights.push(normal);
    }
    
    *seed = rng;
    weights
}

// ── Trinity 3k Components ─────────────────────────────────────────────────────

/// Trinity 3k Attention Head with QK-Norm
struct Trinity3kAttentionHead {
    /// Query projection: d_model × head_dim
    w_q: Vec<f32>,
    /// Key projection: d_model × head_dim  
    w_k: Vec<f32>,
    /// Value projection: d_model × head_dim
    w_v: Vec<f32>,
    /// Output projection: head_dim × head_dim
    w_o: Vec<f32>,
    /// Q-Norm scale (learnable)
    q_norm_scale: Vec<f32>,
    /// K-Norm scale (learnable)
    k_norm_scale: Vec<f32>,
    head_dim: usize,
}

impl Trinity3kAttentionHead {
    fn new(d_model: usize, head_dim: usize, layer_idx: usize, total_layers: usize, seed: &mut u64) -> Self {
        Self {
            w_q: xavier_phi_init(d_model * head_dim, d_model, head_dim, layer_idx, total_layers, seed),
            w_k: xavier_phi_init(d_model * head_dim, d_model, head_dim, layer_idx, total_layers, seed),
            w_v: xavier_phi_init(d_model * head_dim, d_model, head_dim, layer_idx, total_layers, seed),
            w_o: xavier_phi_init(head_dim * head_dim, head_dim, head_dim, layer_idx, total_layers, seed),
            q_norm_scale: vec![1.0; head_dim],
            k_norm_scale: vec![1.0; head_dim],
            head_dim,
        }
    }

    /// Forward pass for one attention head
    /// Input: seq_len × d_model
    /// Output: seq_len × head_dim
    fn forward(&self, xs: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let seq_len = xs.len();
        let d_model = xs[0].len();

        // Project to Q, K, V
        let mut qs = Vec::with_capacity(seq_len);
        let mut ks = Vec::with_capacity(seq_len);
        let mut vs = Vec::with_capacity(seq_len);

        for x in xs {
            qs.push(left_matvec(&self.w_q, self.head_dim, d_model, x));
            ks.push(left_matvec(&self.w_k, self.head_dim, d_model, x));
            vs.push(left_matvec(&self.w_v, self.head_dim, d_model, x));
        }

        // Apply QK-Norm
        for i in 0..seq_len {
            for j in 0..self.head_dim {
                qs[i][j] *= self.q_norm_scale[j];
                ks[i][j] *= self.k_norm_scale[j];
            }
        }

        // Simple attention (no RoPE for now, can be added later)
        let mut output = Vec::with_capacity(seq_len);
        for i in 0..seq_len {
            let mut head_output = vec![0.0; self.head_dim];
            
            // Attention weights
            let mut attn_weights = Vec::with_capacity(seq_len);
            for j in 0..seq_len {
                let mut score = 0.0;
                for k in 0..self.head_dim {
                    score += qs[i][k] * ks[j][k];
                }
                attn_weights.push(score);
            }
            
            // Normalize attention weights
            softmax(&mut attn_weights);
            
            // Weighted sum of values
            for j in 0..seq_len {
                for k in 0..self.head_dim {
                    head_output[k] += attn_weights[j] * vs[j][k];
                }
            }
            
            output.push(head_output);
        }

        output
    }
}

/// Trinity 3k Transformer Layer with QK-Norm and ReLU^2
struct Trinity3kLayer {
    /// Multi-head attention (heads stored separately)
    attention_heads: Vec<Trinity3kAttentionHead>,
    /// Feed-forward network: d_model × 4*d_model
    w_ff1: Vec<f32>,
    /// Feed-forward network: 4*d_model × d_model
    w_ff2: Vec<f32>,
    /// Layer norm 1
    norm1_scale: Vec<f32>,
    /// Layer norm 2  
    norm2_scale: Vec<f32>,
    d_model: usize,
    n_heads: usize,
    head_dim: usize,
    ffn_dim: usize,
}

impl Trinity3kLayer {
    fn new(d_model: usize, n_heads: usize, layer_idx: usize, total_layers: usize, seed: &mut u64) -> Self {
        let head_dim = d_model / n_heads;
        let ffn_dim = d_model * 4; // 4x expansion

        let mut attention_heads = Vec::with_capacity(n_heads);
        for _ in 0..n_heads {
            attention_heads.push(Trinity3kAttentionHead::new(d_model, head_dim, layer_idx, total_layers, seed));
        }

        Self {
            attention_heads,
            w_ff1: xavier_phi_init(d_model * ffn_dim, d_model, ffn_dim, layer_idx, total_layers, seed),
            w_ff2: xavier_phi_init(ffn_dim * d_model, ffn_dim, d_model, layer_idx, total_layers, seed),
            norm1_scale: vec![1.0; d_model],
            norm2_scale: vec![1.0; d_model],
            d_model,
            n_heads,
            head_dim,
            ffn_dim,
        }
    }

    fn forward(&self, xs: &[Vec<f32>]) -> Vec<Vec<f32>> {
        let seq_len = xs.len();
        let eps = 1e-5;

        // Pre-norm + Multi-head attention
        let mut normed_xs = Vec::with_capacity(seq_len);
        for x in xs {
            normed_xs.push(layer_norm(x, eps));
        }

        // Multi-head attention
        let mut head_outputs = Vec::with_capacity(self.n_heads);
        for head in &self.attention_heads {
            head_outputs.push(head.forward(&normed_xs));
        }

        // Concatenate heads and project
        let mut attn_output = Vec::with_capacity(seq_len);
        for i in 0..seq_len {
            let mut concatenated: Vec<f32> = Vec::with_capacity(self.d_model);
            for head in &head_outputs {
                concatenated.extend(&head[i]);
            }
            attn_output.push(concatenated);
        }

        // Residual connection
        let mut residual1 = Vec::with_capacity(seq_len);
        for i in 0..seq_len {
            let mut res = vec![0.0; self.d_model];
            for j in 0..self.d_model {
                res[j] = xs[i][j] + attn_output[i][j];
            }
            residual1.push(res);
        }

        // Pre-norm + Feed-forward with ReLU^2
        let mut normed_residual1 = Vec::with_capacity(seq_len);
        for x in &residual1 {
            normed_residual1.push(layer_norm(x, eps));
        }

        let mut ffn_hidden = Vec::with_capacity(seq_len);
        for x in &normed_residual1 {
            let hidden = left_matvec(&self.w_ff1, self.ffn_dim, self.d_model, x);
            // ReLU^2 activation
            let mut activated = hidden;
            for val in &mut activated {
                *val = relu_squared(*val);
            }
            ffn_hidden.push(activated);
        }

        let mut ffn_output = Vec::with_capacity(seq_len);
        for x in &ffn_hidden {
            ffn_output.push(left_matvec(&self.w_ff2, self.d_model, self.ffn_dim, x));
        }

        // Residual connection 2
        let mut output = Vec::with_capacity(seq_len);
        for i in 0..seq_len {
            let mut out = vec![0.0; self.d_model];
            for j in 0..self.d_model {
                out[j] = residual1[i][j] + ffn_output[i][j];
            }
            output.push(out);
        }

        output
    }
}

/// Trinity 3k Model Configuration
pub struct Trinity3kConfig {
    pub vocab_size: usize,      // 729 = 3^6
    pub hidden_dim: usize,      // 243 = 3^5
    pub n_heads: usize,         // 27 = 3^3
    pub head_dim: usize,        // 9 = 3^2
    pub n_layers: usize,        // ~11 for FP16
    pub max_seq_len: usize,
}

impl Default for Trinity3kConfig {
    fn default() -> Self {
        Self {
            vocab_size: 729,    // 3^6
            hidden_dim: 243,   // 3^5
            n_heads: 27,       // 3^3
            head_dim: 9,       // 3^2
            n_layers: 11,
            max_seq_len: 1024,
        }
    }
}

impl Trinity3kConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.hidden_dim != self.n_heads * self.head_dim {
            return Err(format!(
                "hidden_dim ({}) must equal n_heads ({}) * head_dim ({}), got {}",
                self.hidden_dim, self.n_heads, self.head_dim, self.n_heads * self.head_dim
            ));
        }
        Ok(())
    }

    pub fn total_params(&self) -> usize {
        let emb_params = self.vocab_size * self.hidden_dim; // tied embeddings
        
        let layer_params = (
            // Attention: 4 projections per head * n_heads
            4 * self.head_dim * self.hidden_dim * self.n_heads +
            // QK-Norm scales: 2 * head_dim * n_heads
            2 * self.head_dim * self.n_heads +
            // FFN: 2 linear layers
            2 * self.hidden_dim * (self.hidden_dim * 4) +
            // Layer norms: 2 * hidden_dim
            2 * self.hidden_dim
        ) * self.n_layers;
        
        // Final layer norm
        let final_norm = self.hidden_dim;
        
        emb_params + layer_params + final_norm
    }
}

/// Trinity 3k Byte-Level Model
pub struct Trinity3kModel {
    /// Token embeddings: vocab_size × hidden_dim
    token_embeddings: Vec<f32>,
    /// Transformer layers
    layers: Vec<Trinity3kLayer>,
    /// Final layer norm
    final_norm_scale: Vec<f32>,
    /// Configuration
    config: Trinity3kConfig,
}

impl Trinity3kModel {
    pub fn new(config: Trinity3kConfig) -> Result<Self, String> {
        config.validate()?;

        let mut seed = 42u64;
        
        let token_embeddings = xavier_phi_init(
            config.vocab_size * config.hidden_dim,
            config.vocab_size,
            config.hidden_dim,
            0, // layer 0 for embeddings
            config.n_layers,
            &mut seed
        );

        let mut layers = Vec::with_capacity(config.n_layers);
        for i in 0..config.n_layers {
            layers.push(Trinity3kLayer::new(
                config.hidden_dim,
                config.n_heads,
                i,
                config.n_layers,
                &mut seed
            ));
        }

        let final_norm_scale = vec![1.0; config.hidden_dim];

        Ok(Self {
            token_embeddings,
            layers,
            final_norm_scale,
            config,
        })
    }

    /// Forward pass: tokens → logits
    pub fn forward(&self, input_ids: &[usize]) -> Vec<Vec<f32>> {
        let seq_len = input_ids.len();
        let d_model = self.config.hidden_dim;

        // Token embeddings
        let mut xs = Vec::with_capacity(seq_len);
        for &token_id in input_ids {
            let start = token_id * d_model;
            let embedding = self.token_embeddings[start..start + d_model].to_vec();
            xs.push(embedding);
        }

        // Transformer layers
        let mut hidden = xs;
        for layer in &self.layers {
            hidden = layer.forward(&hidden);
        }

        // Final layer norm
        let eps = 1e-5;
        let mut normed = Vec::with_capacity(seq_len);
        for x in &hidden {
            normed.push(layer_norm(x, eps));
        }

        // Language model head (tied with token embeddings)
        let mut logits = Vec::with_capacity(seq_len);
        for x in &normed {
            let mut logit = vec![0.0; self.config.vocab_size];
            for vocab_idx in 0..self.config.vocab_size {
                let start = vocab_idx * d_model;
                let emb = &self.token_embeddings[start..start + d_model];
                let mut dot = 0.0;
                for (i, &val) in emb.iter().enumerate() {
                    dot += val * x[i];
                }
                logit[vocab_idx] = dot;
            }
            logits.push(logit);
        }

        logits
    }

    /// Compute cross-entropy loss and BPB
    pub fn loss_bpb(&self, tokens: &[usize]) -> (f32, f32) {
        if tokens.len() < 2 {
            return (0.0, 0.0);
        }

        // Input tokens (all but last)
        let input_ids = &tokens[..tokens.len() - 1];
        // Target tokens (all but first)  
        let target_ids = &tokens[1..];

        // Forward pass
        let logits = self.forward(input_ids);

        // Compute cross-entropy loss
        let mut total_loss = 0.0;
        let mut total_tokens = 0;

        for (i, &target) in target_ids.iter().enumerate() {
            if i >= logits.len() {
                break;
            }

            let mut logprobs = logits[i].clone();
            softmax(&mut logprobs);
            
            // Avoid log(0)
            let prob = logprobs[target].max(1e-9);
            total_loss += -prob.ln();
            total_tokens += 1;
        }

        let loss = total_loss / total_tokens as f32;
        let bpb = loss / LN_2; // Convert to bits per byte

        (loss, bpb)
    }

    /// Simple SGD step (for testing)
    pub fn sgd_step(&mut self, _tokens: &[usize], learning_rate: f32) {
        // This is a placeholder - in real implementation we'd need
        // proper backward pass and gradient computation
        // For now, just small random updates to test the pipeline
        for param in &mut self.token_embeddings {
            *param += (rand::random::<f32>() - 0.5) * learning_rate * 0.01;
        }
    }

    pub fn config(&self) -> &Trinity3kConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trinity_3k_config() {
        let config = Trinity3kConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.vocab_size, 729);
        assert_eq!(config.hidden_dim, 243);
        assert_eq!(config.n_heads, 27);
        assert_eq!(config.head_dim, 9);
        assert_eq!(config.total_params(), 729 * 243 + 11 * (4 * 9 * 243 * 27 + 2 * 9 * 27 + 2 * 243 * (243 * 4) + 2 * 243) + 243);
    }

    #[test]
    fn test_trinity_3k_model_creation() {
        let config = Trinity3kConfig::default();
        let model = Trinity3kModel::new(config);
        assert!(model.is_ok());
    }

    #[test]
    fn test_forward_pass() {
        let config = Trinity3kConfig::default();
        let model = Trinity3kModel::new(config).unwrap();
        let tokens = vec![1, 2, 3, 4];
        let logits = model.forward(&tokens);
        assert_eq!(logits.len(), 3); // seq_len - 1
        assert_eq!(logits[0].len(), 729); // vocab_size
    }

    #[test]
    fn test_loss_bpb_finite() {
        let config = Trinity3kConfig::default();
        let model = Trinity3kModel::new(config).unwrap();
        let tokens: Vec<usize> = (0..16).map(|i| i % 729).collect();
        let (loss, bpb) = model.loss_bpb(&tokens);
        assert!(loss.is_finite(), "loss must be finite");
        assert!(bpb.is_finite(), "bpb must be finite");
        assert!(bpb > 0.0, "bpb must be positive");
        println!("Initial BPB (Trinity 3k): {:.4}", bpb);
    }
}
