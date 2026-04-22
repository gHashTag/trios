//! Trinity 3^k Byte-Level Transformer for Parameter Golf #110
//!
//! Architecture based on powers of 3:
//! - vocab_size: 729 (3^6) - byte-level
//! - hidden_dim: 243 (3^5)
//! - n_heads: 27 (3^3) 
//! - head_dim: 9 (3^2)
//! - activation: ReLU^2
//! - position: RoPE
//! - normalization: QK-Norm + RMSNorm
//! - embeddings: tied (in == out)

use burn::{
    module::Module,
    nn::{Embedding, EmbeddingConfig, Linear, LinearConfig, RmsNorm, RmsNormConfig},
    tensor::{backend::Backend, Int, Tensor},
};

pub type TrinityBackend = burn::backend::NdArray<f32>;

/// Trinity 3^k configuration - all dimensions are powers of 3
#[derive(Debug, Clone)]
pub struct Trinity3kConfig {
    pub vocab_size: usize,      // 729 = 3^6
    pub hidden_dim: usize,      // 243 = 3^5
    pub n_heads: usize,         // 27 = 3^3
    pub head_dim: usize,        // 9 = 3^2
    pub n_layers: usize,        // ~11 for FP16, ~44 for INT4
    pub ffn_ratio: usize,       // Usually 4x hidden_dim
    pub tie_embeddings: bool,
    pub max_seq_len: usize,
}

impl Default for Trinity3kConfig {
    fn default() -> Self {
        Self {
            vocab_size: 729,    // 3^6
            hidden_dim: 243,    // 3^5
            n_heads: 27,        // 3^3
            head_dim: 9,        // 3^2
            n_layers: 11,       // FP16 target
            ffn_ratio: 4,        // 4x hidden_dim
            tie_embeddings: true,
            max_seq_len: 1024,
        }
    }
}

impl Trinity3kConfig {
    pub fn validate(&self) -> Result<(), String> {
        // Check all dimensions are powers of 3
        let check_power_of_3 = |value: usize, name: &str, expected_power: u32| -> Result<(), String> {
            if value != 3i32.pow(expected_power) as usize {
                return Err(format!("{} must be 3^{} = {}, got {}", name, expected_power, 3i32.pow(expected_power), value));
            }
            Ok(())
        };

        check_power_of_3(self.vocab_size, "vocab_size", 6)?;
        check_power_of_3(self.hidden_dim, "hidden_dim", 5)?;
        check_power_of_3(self.n_heads, "n_heads", 3)?;
        check_power_of_3(self.head_dim, "head_dim", 2)?;

        // Check hidden_dim == n_heads * head_dim
        if self.hidden_dim != self.n_heads * self.head_dim {
            return Err(format!("hidden_dim ({}) must equal n_heads ({}) * head_dim ({}), got {}",
                self.hidden_dim, self.n_heads, self.head_dim, self.n_heads * self.head_dim));
        }

        Ok(())
    }

    pub fn ffn_dim(&self) -> usize {
        self.hidden_dim * self.ffn_ratio
    }

    pub fn total_params(&self) -> usize {
        let emb_params = if self.tie_embeddings { 
            self.vocab_size * self.hidden_dim 
        } else { 
            2 * self.vocab_size * self.hidden_dim 
        };
        
        let layer_params = (
            // Attention: QKV + Output projections
            3 * self.hidden_dim * self.hidden_dim + // QKV
            self.hidden_dim * self.hidden_dim +     // Output
            // QK-Norm + RMSNorm
            2 * self.hidden_dim +
            // FFN: 2 linear layers
            2 * self.hidden_dim * self.ffn_dim()
        ) * self.n_layers;
        
        // Final norm
        let final_norm = self.hidden_dim;
        
        emb_params + layer_params + final_norm
    }

    pub fn estimate_size_mb(&self) -> f64 {
        let total_params = self.total_params();
        let bytes_per_param = 2.0; // FP16
        total_params as f64 * bytes_per_param / (1024.0 * 1024.0)
    }
}

/// ReLU^2 activation: max(0, x)^2
pub fn relu_squared<B: Backend>(x: Tensor<B, 3>) -> Tensor<B, 3> {
    x.clone().clamp(0.0, f32::MAX).powf(2.0)
}

/// QK-Norm: Normalize query and key before attention
#[derive(Module, Debug)]
pub struct QKNorm<B: Backend> {
    q_norm: RmsNorm<B>,
    k_norm: RmsNorm<B>,
}

impl<B: Backend> QKNorm<B> {
    pub fn new(device: &B::Device, hidden_dim: usize) -> Self {
        let q_norm_cfg = RmsNormConfig::new(hidden_dim);
        let k_norm_cfg = RmsNormConfig::new(hidden_dim);
        
        Self {
            q_norm: RmsNormConfig::init(&q_norm_cfg, device),
            k_norm: RmsNormConfig::init(&k_norm_cfg, device),
        }
    }

    pub fn forward(&self, q: Tensor<B, 3>, k: Tensor<B, 3>) -> (Tensor<B, 3>, Tensor<B, 3>) {
        let q_normed = self.q_norm.forward(q);
        let k_normed = self.k_norm.forward(k);
        (q_normed, k_normed)
    }
}

/// Trinity 3^k Multi-Head Attention with QK-Norm
#[derive(Module, Debug)]
pub struct Trinity3kAttention<B: Backend> {
    qkv_proj: Linear<B>,
    out_proj: Linear<B>,
    qk_norm: QKNorm<B>,
    rope: RotaryPosEncoding<B>,
    n_heads: usize,
    head_dim: usize,
    hidden_dim: usize,
}

#[derive(Module, Debug)]
pub struct RotaryPosEncoding<B: Backend> {
    dim: usize,
    theta: Tensor<B, 1>,
}

impl<B: Backend> RotaryPosEncoding<B> {
    pub fn new(device: &B::Device, dim: usize) -> Self {
        let theta_data: Vec<f32> = (0..dim / 2)
            .map(|i| 10000.0f32.powf(-2.0 * i as f32 / dim as f32))
            .collect();
        let theta = Tensor::from_floats(theta_data.as_slice(), device);
        Self { dim, theta }
    }

    pub fn rotate_half(&self, x: Tensor<B, 4>) -> Tensor<B, 4> {
        let shape = x.dims();
        let batch = shape[0];
        let heads = shape[1];
        let seq = shape[2];

        let half_dim = self.dim / 2;
        let x1 = x.clone().narrow(3, 0, half_dim);
        let x2 = x.clone().narrow(3, half_dim, half_dim);

        let theta = self.theta.clone().reshape([1, 1, 1, half_dim]);
        let sin = theta.clone().sin();
        let cos = theta.clone().cos();

        let x1_rot = x1 * cos.clone();
        let x2_rot = x2 * sin.clone();
        let neg_x2_rot = x2_rot * -1.0;

        let x_rot = Tensor::cat(vec![x1_rot - neg_x2_rot, x1_rot * sin + x2_rot * cos], 3);
        x_rot
    }
}

impl<B: Backend> Trinity3kAttention<B> {
    pub fn new(device: &B::Device, config: &Trinity3kConfig) -> Self {
        let qkv_cfg = LinearConfig::new(config.hidden_dim, config.hidden_dim * 3).with_bias(false);
        let out_cfg = LinearConfig::new(config.hidden_dim, config.hidden_dim).with_bias(false);

        Self {
            qkv_proj: LinearConfig::init(&qkv_cfg, device),
            out_proj: LinearConfig::init(&out_cfg, device),
            qk_norm: QKNorm::new(device, config.hidden_dim),
            rope: RotaryPosEncoding::new(device, config.head_dim),
            n_heads: config.n_heads,
            head_dim: config.head_dim,
            hidden_dim: config.hidden_dim,
        }
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let shape = x.dims();
        let batch = shape[0];
        let seq = shape[1];

        // QKV projection
        let qkv = self.qkv_proj.forward(x.clone());
        let qkv = qkv.reshape([batch, seq, 3, self.n_heads, self.head_dim]);

        let q = qkv.clone().select(0, Tensor::from_ints([0], &x.device()));
        let k = qkv.clone().select(0, Tensor::from_ints([1], &x.device()));
        let v = qkv.select(0, Tensor::from_ints([2], &x.device()));

        // Reshape for multi-head
        let q = q.reshape([batch, self.n_heads, seq, self.head_dim]);
        let k = k.reshape([batch, self.n_heads, seq, self.head_dim]);
        let v = v.reshape([batch, self.n_heads, seq, self.head_dim]);

        // QK-Norm
        let (q_normed, k_normed) = self.qk_norm.forward(q, k);

        // RoPE
        let q_rot = self.rope.rotate_half(q_normed);
        let k_rot = self.rope.rotate_half(k_normed);

        // Attention scores
        let scores = q_rot.matmul(k_rot.swap_dims(2, 3));
        let scale = (self.head_dim as f32).sqrt();
        let scores = scores / scale;

        // Softmax
        let weights = burn::tensor::activation::softmax(scores, -1);

        // Context
        let context = weights.matmul(v.swap_dims(2, 3));
        let context = context.swap_dims(2, 3);
        let context = context.reshape([batch, seq, self.hidden_dim]);

        // Output projection
        self.out_proj.forward(context)
    }
}

/// Trinity 3k Feed-Forward Network with ReLU^2
#[derive(Module, Debug)]
pub struct Trinity3kFFN<B: Backend> {
    w1: Linear<B>,
    w2: Linear<B>,
}

impl<B: Backend> Trinity3kFFN<B> {
    pub fn new(device: &B::Device, hidden_dim: usize, ffn_dim: usize) -> Self {
        let w1_cfg = LinearConfig::new(hidden_dim, ffn_dim);
        let w2_cfg = LinearConfig::new(ffn_dim, hidden_dim).with_bias(false);

        Self {
            w1: LinearConfig::init(&w1_cfg, device),
            w2: LinearConfig::init(&w2_cfg, device),
        }
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        let hidden = relu_squared(self.w1.forward(x.clone()));
        self.w2.forward(hidden)
    }
}

/// Trinity 3k Transformer Layer
#[derive(Module, Debug)]
pub struct Trinity3kLayer<B: Backend> {
    norm1: RmsNorm<B>,
    attn: Trinity3kAttention<B>,
    norm2: RmsNorm<B>,
    ffn: Trinity3kFFN<B>,
}

impl<B: Backend> Trinity3kLayer<B> {
    pub fn new(device: &B::Device, config: &Trinity3kConfig) -> Self {
        let norm1_cfg = RmsNormConfig::new(config.hidden_dim);
        let norm2_cfg = RmsNormConfig::new(config.hidden_dim);

        Self {
            norm1: RmsNormConfig::init(&norm1_cfg, device),
            attn: Trinity3kAttention::new(device, config),
            norm2: RmsNormConfig::init(&norm2_cfg, device),
            ffn: Trinity3kFFN::new(device, config.hidden_dim, config.ffn_dim()),
        }
    }

    pub fn forward(&self, x: Tensor<B, 3>) -> Tensor<B, 3> {
        // Pre-norm + Residual for attention
        let normed = self.norm1.forward(x.clone());
        let attn_out = self.attn.forward(normed);
        let x = x + attn_out;

        // Pre-norm + Residual for FFN
        let normed = self.norm2.forward(x.clone());
        let ffn_out = self.ffn.forward(normed);
        x + ffn_out
    }
}

/// Trinity 3k Byte-Level Transformer Model
#[derive(Module, Debug)]
pub struct Trinity3kModel<B: Backend> {
    tok_emb: Embedding<B>,
    layers: Vec<Trinity3kLayer<B>>,
    final_norm: RmsNorm<B>,
    config: Trinity3kConfig,
}

impl<B: Backend> Trinity3kModel<B> {
    pub fn new(device: &B::Device, config: Trinity3kConfig) -> Result<Self, String> {
        config.validate()?;

        let emb_cfg = EmbeddingConfig::new(config.vocab_size, config.hidden_dim);
        let tok_emb = EmbeddingConfig::init(&emb_cfg, device);

        let mut layers = Vec::with_capacity(config.n_layers);
        for _ in 0..config.n_layers {
            layers.push(Trinity3kLayer::new(device, &config));
        }

        let final_norm_cfg = RmsNormConfig::new(config.hidden_dim);
        let final_norm = RmsNormConfig::init(&final_norm_cfg, device);

        Ok(Self {
            tok_emb,
            layers,
            final_norm,
            config,
        })
    }

    pub fn forward(&self, tokens: Tensor<B, 2, Int>) -> Tensor<B, 3> {
        let mut x = self.tok_emb.forward(tokens);

        for layer in &self.layers {
            x = layer.forward(x);
        }

        self.final_norm.forward(x)
    }

    pub fn forward_with_loss(
        &self,
        tokens: Tensor<B, 2, Int>,
        device: &B::Device,
    ) -> (Tensor<B, 3>, Tensor<B, 1>) {
        use burn::nn::loss::CrossEntropyLossConfig;

        let seq_len = tokens.dims()[1];
        let inputs = tokens.clone().narrow(1, 0, seq_len - 1);
        let targets = tokens.narrow(1, 1, seq_len - 1);

        let hidden = self.forward(inputs.clone());
        let batch = hidden.dims()[0];
        let seq_m1 = hidden.dims()[1];

        let hidden_2d = hidden.clone().reshape([batch * seq_m1, self.config.hidden_dim]);
        let emb_weights = self.tok_emb.weight.val();

        let logits_2d = hidden_2d.matmul(emb_weights);
        let logits_for_loss = logits_2d.clone();
        let logits = logits_2d.reshape([batch, seq_m1, self.config.vocab_size]);
        let targets_1d = targets.reshape([batch * seq_m1]);

        let loss_cfg = CrossEntropyLossConfig::new();
        let loss = loss_cfg.init(device).forward(logits_for_loss, targets_1d);

        (logits, loss)
    }

    pub fn config(&self) -> &Trinity3kConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burn::backend::NdArray;

    #[test]
    fn test_trinity_3k_config() {
        let config = Trinity3kConfig::default();
        assert!(config.validate().is_ok());
        
        // Check dimensions
        assert_eq!(config.vocab_size, 729);    // 3^6
        assert_eq!(config.hidden_dim, 243);   // 3^5
        assert_eq!(config.n_heads, 27);       // 3^3
        assert_eq!(config.head_dim, 9);       // 3^2
        assert_eq!(config.hidden_dim, config.n_heads * config.head_dim);
    }

    #[test]
    fn test_trinity_3k_model_creation() {
        let device = NdArrayDevice::default();
        let config = Trinity3kConfig::default();
        let model = Trinity3kModel::new(&device, config);
        assert!(model.is_ok());
    }
}
