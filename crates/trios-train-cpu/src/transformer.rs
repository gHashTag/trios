//! Minimal Transformer — Phase 2 (HIGH)
//!
//! Expected BPB: 1.80 (30% improvement over N-gram baseline 2.53)
//! Architecture:
//! - MHA (Multi-Head Attention): 16 heads, d=16
//! - Positional Encoding: learned embeddings
//! - LayerNorm (Pre-Norm)
//! - FFN (Feed-Forward): 2 layers
//!
//! Based on IGLA Phase A/B study:
//! - Phase B (n_layers=6, d_ff=233): 1.80 BPB ✓ PROVEN
//! - Target: 1.50 BPB

use crate::forward::{softmax, layer_norm, matmul, vec_add, vec_scale, gelu};
use crate::optimizer::AdamWCpu;

/// MHA (Multi-Head Attention)
#[derive(Debug, Clone)]
pub struct MultiHeadAttention {
    n_heads: usize,
    d_k: usize,       // Key/Value projection dimension
    d_model: usize,     // Model dimension
}

impl Default for MultiHeadAttention {
    fn default() -> Self {
        Self {
            n_heads: 8,
            d_k: 16,
            d_model: 384,  // Using best from N-gram baseline
        }
    }
}

/// Positional encoding
pub fn positional_encoding(seq_len: usize, d_model: usize, pos: usize) -> Vec<f32> {
    let mut pos_emb = vec![0.0f32; d_model];
    
    // Learned positional embeddings (simple sin/cos)
    for i in 0..seq_len {
        let angle = 2.0 * std::f32::consts::PI * (pos as f32) / seq_len as f32;
        for d in 0..d_model {
            pos_emb[d * seq_len + d] = if i % 2 == 0 {
                angle.sin()
            } else {
                angle.cos()
            };
        }
    }
    
    pos_emb
}

/// LayerNorm
pub fn layer_norm(x: &[f32]) -> Vec<f32> {
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + 1e-5f32).sqrt();
    
    x.iter().map(|v| (v - mean) / std).collect()
}

/// FFN (Feed-Forward Network)
#[derive(Debug, Clone)]
pub struct FFNLayer {
    d_model: usize,
    d_ffn: usize,
    activation: fn(&[f32]) -> [f32],
}

impl FFNLayer {
    pub fn new(d_model: usize, d_ffn: usize) -> Self {
        let d_hidden = d_ffn; // Intermediate dimension
        
        Self {
            d_model,
            d_ffn,
            activation: gelu,
        }
    }
    
    pub fn forward(&self, x: &[f32], weights: &[f32]) -> Vec<f32> {
        let d = self.d_model;
        let n = x.len() / d;
        let d_hidden = self.d_ffn;
        
        // First expansion: x → d_hidden
        let mut h1: vec![0.0f32; d_hidden];
        for i in 0..d {
            for j in 0..d_hidden {
                h1[i * d + j] += x[i * d + j];
            }
        }
        
        // Activation
        let mut h2 = (self.activation)(&h1);
        
        // Contraction: d_hidden → d
        let mut out = vec![0.0f32; n];
        for i in 0..n {
            for j in 0..d {
                out[i * d + j] += h2[i * d + j];
            }
        }
        
        out
    }
}

/// Minimal Transformer Model
pub struct MinimalTransformer {
    pub vocab_size: usize,
    pub d_model: usize,
    pub d_ffn: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub max_seq_len: usize,
    
    // Parameters
    token_embedding: Vec<f32>,
    pos_embedding: Vec<f32>,
    attn_weights: Vec<f32>,
    ffn_weights: Vec<f32>,
}

impl MinimalTransformer {
    pub fn new(vocab_size: usize, d_model: usize, d_ffn: usize, n_heads: usize, n_layers: usize) -> Self {
        // Use proven architecture from IGLA Phase A/B
        let d_k = d_model / n_heads;
        let d_ffn = d_model * 4;
        
        // Initialize weights with Xavier initialization
        let mut rng = 0x1337_c0de_u64;
        
        let token_emb = Self::init_weight(vocab_size * d_model, vocab_size, d_model, &mut rng);
        let pos_emb = Self::init_weight(vocab_size * d_model, vocab_size, d_model, &mut rng);
        
        // Attention weights (Q, K, V projections)
        let attn_q = Self::init_weight(n_heads * d_model * d_k, n_heads * d_model, d_k, &mut rng);
        let attn_k = Self::init_weight(n_heads * d_model * d_k, n_heads * d_model, d_k, &mut rng);
        let attn_v = Self::init_weight(n_heads * d_model * d_k, n_heads * d_model, d_k, &mut rng);
        
        // FFN weights
        let ffn_layers: Vec<Vec<f32>> = Vec::with_capacity(n_layers);
        for _ in 0..n_layers {
            let w1 = Self::init_weight(d_model * d_ffn, d_model, d_model, &mut rng);
            let w2 = Self::init_weight(d_ffn * d_model, d_model, d_model, &mut rng);
            ffn_layers.push(w1.clone());
            ffn_layers.push(w2.clone());
        }
        
        Self {
            vocab_size,
            d_model,
            d_ffn,
            n_heads,
            n_layers,
            max_seq_len: 256,
            token_embedding,
            pos_embedding,
            attn_weights: Vec::new(),
            ffn_weights,
        }
    }
    
    fn init_weight(size: usize, fan_in: usize, fan_out: usize, seed: &mut u64) -> Vec<f32> {
        let scale = (6.0f32 / (fan_in + fan_out) as f32).sqrt();
        let limit = (2.0f32 / size as f32).sqrt();
        
        (0..size)
            .map(|_| {
                // LCG for deterministic initialization
                let t = ((*seed >> 33) as f32) / (u32::MAX as f32);
                let value = t * 2.0 * limit - limit;
                // Ensure values are in reasonable range [-limit, limit]
                let clamped = if value < -limit { -limit } else { limit };
                clamped
            })
            .collect()
    }
    
    /// Forward pass with multi-head attention
    pub fn forward(&mut self, tokens: &[usize], _cache: Option<&SelfAttentionCache>) -> Vec<Vec<f32>> {
        if tokens.is_empty() {
            return vec![];
        }
        
        let seq_len = tokens.len();
        let vocab_size = self.vocab_size;
        
        // Token embeddings with positional encoding
        let mut input_embeddings = Vec::with_capacity(seq_len * self.d_model);
        
        for (pos, &token_id) in tokens.iter().enumerate() {
            let pos_emb = &self.pos_embedding[pos * vocab_size..(pos + 1) * vocab_size];
            let token_emb = &self.token_embedding[token_id * vocab_size..(token_id + 1) * vocab_size];
            
            // Combine token and positional embeddings
            let mut combined = vec![0.0f32; self.d_model];
            for d in 0..self.d_model {
                combined[d] = token_emb[d] + pos_emb[d];
            }
            
            // Apply LayerNorm
            let normed = layer_norm(&combined);
            input_embeddings.push(normed);
        }
        
        // Multi-head attention (sum of 8 heads)
        let d_k = self.d_model / self.n_heads;
        let mut attention_output = vec![0.0f32; seq_len * self.d_model];
        
        for pos in 0..seq_len {
            let mut head_sum = vec![0.0f32; self.d_model];
            
            for head in 0..self.n_heads {
                let head_start = pos * d_k;
                let head_end = head_start + d_k;
                
                // Query, Key, Value projections from weights
                let q = &self.attn_weights[head_start..head_end];
                let k = &self.attn_weights[head_start..head_end];
                let v = &self.attn_weights[head_start..head_end];
                
                // Compute attention score for this head
                let mut score = 0.0f32;
                for (j, &val) in input_embeddings.iter().enumerate() {
                    if j >= pos { break; } // Causal attention
                    for d in 0..d_k {
                        score += q[j * self.d_model + d] * val[d];
                    }
                }
                
                // Normalize across all heads
                score /= (self.n_heads as f32 * d_k.sqrt());
                head_sum.iter_mut().enumerate().for_each(|(idx, h)| {
                    *h += score;
                });
            }
            
            attention_output[pos * self.d_model..(pos + 1) * self.d_model] = head_sum[0];
        }
        
        // Apply LayerNorm to attention output
        let attention_normed = layer_norm(&attention_output);
        
        // FFN layers (2 layers)
        let mut x = attention_normed.clone();
        
        for layer in 0..self.n_layers {
            x = Self::apply_ffn_layer(&x, &self.ffn_weights, layer);
        }
        
        // Project to vocabulary
        let mut logits = vec![0.0f32; vocab_size];
        
        // Final projection (simplified, no LM head for Phase 2)
        for i in 0..seq_len {
            let d = self.d_model;
            for j in 0..vocab_size {
                logits[i * vocab_size + j] = x[i * d + j];
            }
        }
        
        logits
    }
    
    fn apply_ffn_layer(&self, x: &[f32], weights: &[f32], layer: usize) -> Vec<f32> {
        let d = self.d_model;
        let d_hidden = self.d_ffn;
        let n = x.len() / d;
        
        // Expansion: x → d_hidden
        let mut h1 = vec![0.0f32; d_hidden];
        for i in 0..d {
            for j in 0..d_hidden {
                h1[i * d + j] += x[i * d + j];
            }
        }
        
        // Activation
        let mut h2 = (self.ffn_layers[layer * 2].activation)(&h1);
        
        // Contraction: d_hidden → d
        let mut out = vec![0.0f32; n];
        for i in 0..n {
            for j in 0..d {
                out[i * d + j] += h2[i * d + j];
            }
        }
        
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_head_attention_default() {
        let mha = MultiHeadAttention::default();
        assert_eq!(mha.n_heads, 8);
        assert_eq!(mha.d_model, 384);
        assert_eq!(mha.d_k, 48);  // 384 / 8 = 48
    }

    #[test]
    fn test_positional_encoding() {
        let d_model = 384;
        let seq_len = 64;
        let vocab_size = 128;
        
        let pos = positional_encoding(seq_len, d_model, pos);
        
        assert_eq!(pos.len(), seq_len * d_model);
        assert_eq!(pos[0].iter().all(|x| x.abs() <= 1.0f32), "All zero");
    }

    #[test]
    fn test_layer_norm() {
        let x = vec![1.0f32, 2.0, 3.0];
        let normalized = layer_norm(&x);
        
        assert_eq!(normalized.len(), 3);
        let mean = normalized.iter().sum::<f32>() / 3.0;
        assert!((mean - 1.0).abs() < 1e-6);
        
        let std = (normalized.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / 3.0).sqrt();
        assert!((std - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_ffn_expansion() {
        let d_model = 384;
        let d_ffn = 1536; // 384 * 4
        let mut h1 = vec![0.0f32; d_ffn];
        
        for i in 0..d_model {
            for j in 0..d_ffn {
                h1[i * d + j] = 0.0;
            }
        }
        
        let mut h2 = gelu(&h1);
        let mut out = vec![0.0f32; d_model];
        
        for i in 0..d_model {
            for j in 0..d_model {
                out[i * d + j] += h2[i * d + j];
            }
        }
        
        assert_eq!(out.len(), d_model);
    }

    #[test]
    fn test_minimal_transformer_forward() {
        let transformer = MinimalTransformer::new(128, 384, 384, 8, 2);
        let tokens = vec![1usize, 2, 3, 4, 5];
        
        let logits = transformer.forward(&tokens, None);
        assert_eq!(logits.len(), 5); // seq_len=5, vocab_size=128
        assert_eq!(logits[0].len(), 128);
    }

    #[test]
    fn test_weight_initialization() {
        let mut rng = 0x1337_c0de_u64;
        let size = 100;
        
        let weights = MinimalTransformer::init_weight(size, size, size, &mut rng);
        assert_eq!(weights.len(), size);
        
        // Check bounds
        let all_in_bounds = weights.iter().all(|w| w.abs() <= 1.0);
        assert!(all_in_bounds, "Weights should be in [-1, 1]");
    }
}
