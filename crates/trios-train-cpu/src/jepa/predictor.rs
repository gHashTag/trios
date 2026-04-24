//! TASK-5A.3 — JEPA Prediction Head
//!
//! Skeleton: linear projection placeholder.
//! Theory: https://github.com/gHashTag/trinity/tree/main/docs/research/models/JEPA-T/

#[derive(Debug, Clone)]
pub struct PredictorConfig {
    pub d_model: usize,
    pub hidden_dim: usize,
    pub num_layers: usize,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self { d_model: 384, hidden_dim: 256, num_layers: 2 }
    }
}

#[derive(Debug, Clone)]
pub struct PredictionOutput {
    pub predicted: Vec<f32>,
    pub loss: f64,
}

pub struct Predictor {
    pub config: PredictorConfig,
    weights: Vec<f32>,
}

impl Predictor {
    pub fn new(config: PredictorConfig) -> Self {
        let n = config.d_model * config.hidden_dim * 2;
        Self { config, weights: vec![0.0_f32; n] }
    }

    pub fn forward(&self, context_embeddings: &[f32], target_positions: &[usize]) -> PredictionOutput {
        let d = self.config.d_model;
        let ctx_len = context_embeddings.len() / d.max(1);
        let mut predicted = Vec::with_capacity(target_positions.len() * d);
        for &pos in target_positions {
            if pos < ctx_len {
                let start = pos * d;
                let end = (start + d).min(context_embeddings.len());
                let slice = &context_embeddings[start..end];
                predicted.extend_from_slice(slice);
                if slice.len() < d {
                    predicted.extend(vec![0.0_f32; d - slice.len()]);
                }
            } else {
                predicted.extend(vec![0.0_f32; d]);
            }
        }
        PredictionOutput { predicted, loss: 0.0 }
    }

    pub fn num_params(&self) -> usize { self.weights.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_output_shape() {
        let cfg = PredictorConfig { d_model: 8, hidden_dim: 4, num_layers: 2 };
        let p = Predictor::new(cfg);
        let ctx = vec![1.0_f32; 16 * 8];
        let out = p.forward(&ctx, &[0, 3, 7]);
        assert_eq!(out.predicted.len(), 3 * 8);
    }

    #[test]
    fn test_full_jepa_forward() {
        use crate::jepa::masking::{MaskConfig, mask_spans, get_masked};
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        let mut rng = StdRng::seed_from_u64(42);
        let result = mask_spans(32, MaskConfig::default(), &mut rng);
        let tgt = get_masked(&result.mask);
        let ctx_emb: Vec<f32> = (0..32 * 8).map(|i| i as f32 * 0.01).collect();
        let p = Predictor::new(PredictorConfig { d_model: 8, hidden_dim: 4, num_layers: 2 });
        let out = p.forward(&ctx_emb, &tgt);
        assert_eq!(out.predicted.len(), tgt.len() * 8);
    }

    #[test]
    fn test_predictor_zero_targets() {
        let p = Predictor::new(PredictorConfig::default());
        let ctx = vec![1.0_f32; 10 * 384];
        let out = p.forward(&ctx, &[]);
        assert!(out.predicted.is_empty());
    }
}
