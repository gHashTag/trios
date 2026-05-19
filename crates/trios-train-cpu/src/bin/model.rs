//! Ngram model architecture for T-JEPA training.
//!
//! Module: model
//! Size: ≤ 60 lines (NASA Rule 4)

const VOCAB: usize = 128;
const DIM: usize = 64;
const HIDDEN: usize = 384;
const NUM_CTX: usize = 4;
const NGRAM: usize = NUM_CTX + 2;
const SEQ: usize = 64;

/// N-gram language model with context embeddings.
pub struct NgramModel {
    pub embed: Vec<f32>,
    pub ctx: Vec<Vec<f32>>,
    pub ctx_weights: Vec<f32>,
    pub proj: Vec<f32>,
    pub lm_head: Vec<f32>,
}

impl NgramModel {
    /// Initialize model with deterministic seeding.
    pub fn new(seed: u64) -> Self {
        let mut s = seed;
        let mut rng = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((s >> 33) as f32) / (u32::MAX as f32) * 2.0 - 1.0
        };
        let lim = (6.0f32 / (3 * DIM) as f32).sqrt();
        let lim_h = (6.0f32 / (DIM + HIDDEN) as f32).sqrt();
        let lim_o = (6.0f32 / (HIDDEN + VOCAB) as f32).sqrt();
        let base_weights: Vec<f32> = vec![0.7, 0.3, 0.2, 0.15];
        let ctx_weights: Vec<f32> = base_weights.iter().take(NUM_CTX).cloned().collect();
        Self {
            embed: (0..VOCAB * DIM).map(|_| rng() * lim).collect(),
            ctx: (0..NUM_CTX)
                .map(|_| (0..VOCAB * DIM).map(|_| rng() * lim).collect())
                .collect(),
            ctx_weights,
            proj: (0..HIDDEN * DIM).map(|_| rng() * lim_h).collect(),
            lm_head: (0..VOCAB * HIDDEN).map(|_| rng() * lim_o).collect(),
        }
    }

    /// Compute hidden representation from context tokens.
    pub fn compute_hidden(&self, context: &[usize]) -> Vec<f32> {
        assert!(context.len() >= 2, "context too short for hidden");
        let t0 = context[context.len() - 1].min(VOCAB - 1);
        let mut combined = self.embed[t0 * DIM..(t0 + 1) * DIM].to_vec();
        for (ci, cw) in self.ctx_weights.iter().enumerate() {
            let ctx_idx = context.len() - 2 - ci;
            let t = context[ctx_idx].min(VOCAB - 1);
            let cv = &self.ctx[ci][t * DIM..(t + 1) * DIM];
            for j in 0..DIM {
                combined[j] += cv[j] * cw;
            }
        }
        layer_norm(&combined, 1e-5)
    }

    /// Predict logits from hidden representation.
    pub fn predict(&self, hidden: &[f32]) -> Vec<f32> {
        assert_eq!(hidden.len(), HIDDEN, "hidden dim mismatch");
        let mut logits = vec![0.0f32; VOCAB];
        for (vi, logit) in logits.iter_mut().enumerate() {
            for (hi, hn) in hidden.iter().enumerate() {
                *logit += self.lm_head[vi * HIDDEN + hi] * hn;
            }
        }
        logits
    }

    /// Compute loss on a sequence of tokens.
    pub fn loss_on_seq(&self, tokens: &[usize]) -> f32 {
        if tokens.len() < NGRAM + 1 {
            return 0.0;
        }
        let count = tokens.len() - NGRAM;
        assert!(count > 0, "no n-gram pairs in sequence");
        let mut total = 0.0f32;
        for i in 0..count {
            let context = &tokens[i..i + NGRAM];
            let target = tokens[i + NGRAM].min(VOCAB - 1);
            let mut logits = self.predict(&self.compute_hidden(context));
            softmax(&mut logits);
            total -= logits[target].max(1e-10).ln();
        }
        total / count as f32
    }

    /// Update embeddings using EMA.
    pub fn update_ema(&mut self, target: &NgramModel, decay: f32) {
        ema_inplace(&mut self.embed, &target.embed, decay);
    }
}

/// Layer normalization with epsilon.
fn layer_norm(x: &[f32], eps: f32) -> Vec<f32> {
    assert!(!x.is_empty(), "layer_norm: empty input");
    let n = x.len() as f32;
    let mean = x.iter().sum::<f32>() / n;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    let std = (var + eps).sqrt();
    x.iter().map(|v| (v - mean) / std).collect()
}

/// Softmax activation (in-place).
fn softmax(v: &mut [f32]) {
    assert!(!v.is_empty(), "softmax: empty input");
    let max = v.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0.0f32;
    for x in v.iter_mut() {
        *x = (*x - max).exp();
        sum += *x;
    }
    assert!(sum > 0.0, "softmax: zero sum");
    for x in v.iter_mut() {
        *x /= sum;
    }
}

/// Exponential moving average update (in-place).
fn ema_inplace(target: &mut [f32], online: &[f32], decay: f32) {
    assert_eq!(target.len(), online.len(), "EMA size mismatch");
    assert!((0.0..1.0).contains(&decay), "EMA decay out of range");
    for (t, o) in target.iter_mut().zip(online.iter()) {
        *t = decay * *t + (1.0 - decay) * *o;
    }
}
