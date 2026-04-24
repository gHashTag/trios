//! JEPA predictor head
//!
//! Projects context embeddings to predict masked target embeddings.

/// Predictor configuration
#[derive(Debug, Clone)]
pub struct PredictorConfig {
    pub d_model: usize,
    pub hidden_dim: usize,
    pub num_layers: usize,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self {
            d_model: 384,
            hidden_dim: 256,
            num_layers: 2,
        }
    }
}

/// Prediction output
#[derive(Debug, Clone)]
pub struct PredictionOutput {
    pub predicted: Vec<f32>,
    pub target: Vec<f32>,
    pub loss: f64,
}

impl PredictionOutput {
    pub fn new(predicted: Vec<f32>, target: Vec<f32>, loss: f64) -> Self {
        Self { predicted, target, loss }
    }
}

/// JEPA predictor head
///
/// Simple MLP-like predictor that takes context embeddings
/// and predicts target embeddings at masked positions.
pub struct Predictor {
    config: PredictorConfig,
    weights: Vec<f32>,
}

impl Predictor {
    /// Create a new predictor with random initialization
    pub fn new(config: PredictorConfig) -> Self {
        // Calculate total parameters: input -> hidden -> ... -> output
        let total_params = config.d_model * config.hidden_dim
            + config.hidden_dim * config.hidden_dim * (config.num_layers - 1).max(1)
            + config.hidden_dim * config.d_model;

        Self {
            config,
            weights: vec![0.0; total_params],
        }
    }

    /// Create predictor with default configuration
    pub fn default_with_dim(d_model: usize) -> Self {
        Self::new(PredictorConfig {
            d_model,
            ..Default::default()
        })
    }

    /// Forward pass: predict target embeddings from context embeddings
    ///
    /// # Arguments
    /// * `context` - Unmasked context embeddings (flattened)
    /// * `target_positions` - Positions to predict (number of targets)
    ///
    /// # Returns
    /// Flattened predicted embeddings for target positions
    ///
    /// This is a simplified forward pass. A full implementation would
    /// include actual MLP layers with activations.
    pub fn forward(
        &self,
        context: &[f32],
        target_positions: &[usize],
    ) -> Vec<f32> {
        let d_model = self.config.d_model;
        let num_targets = target_positions.len();

        let mut predicted = Vec::with_capacity(num_targets * d_model);

        for &pos in target_positions {
            let context_pos = pos % (context.len() / d_model);
            let start = context_pos * d_model;
            let end = (start + d_model).min(context.len());

            // Simple projection: copy context with small perturbation
            if start < context.len() {
                for i in start..end {
                    predicted.push(context[i] + 0.01 * (pos as f32 - context_pos as f32));
                }
            } else {
                predicted.extend(vec![0.0; d_model]);
            }
        }

        // Pad if needed
        while predicted.len() < num_targets * d_model {
            predicted.push(0.0);
        }

        predicted
    }

    /// Compute prediction loss (MSE) between predicted and target
    pub fn compute_loss(&self, predicted: &[f32], target: &[f32]) -> f64 {
        predicted
            .iter()
            .zip(target.iter())
            .map(|(p, t)| (p - t).powi(2) as f64)
            .sum::<f64>() / predicted.len().max(1) as f64
    }

    /// Full prediction pass with loss computation
    ///
    /// # Arguments
    /// * `context` - Context embeddings
    /// * `target_positions` - Positions to predict
    /// * `target_embeddings` - Actual target embeddings
    pub fn predict(
        &self,
        context: &[f32],
        target_positions: &[usize],
        target_embeddings: &[f32],
    ) -> PredictionOutput {
        let predicted = self.forward(context, target_positions);
        let loss = self.compute_loss(&predicted, target_embeddings);

        PredictionOutput::new(predicted, target_embeddings.to_vec(), loss)
    }

    /// Get number of parameters
    pub fn num_params(&self) -> usize {
        self.weights.len()
    }

    /// Get configuration
    pub fn config(&self) -> &PredictorConfig {
        &self.config
    }
}

/// Reshape flat embeddings to 2D matrix
///
/// # Arguments
/// * `flat` - Flattened embeddings
/// * `d_model` - Embedding dimension
///
/// # Returns
/// 2D matrix as Vec<Vec<f32>> where each inner vector is one embedding
pub fn reshape_to_matrix(flat: &[f32], d_model: usize) -> Vec<Vec<f32>> {
    let n = flat.len() / d_model;
    let mut matrix = Vec::with_capacity(n);

    for i in 0..n {
        let start = i * d_model;
        let end = (start + d_model).min(flat.len());
        matrix.push(flat[start..end].to_vec());
    }

    matrix
}

/// Flatten 2D matrix to 1D vector
pub fn flatten_matrix(matrix: &[Vec<f32>]) -> Vec<f32> {
    matrix.iter().flatten().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_creation() {
        let predictor = Predictor::new(PredictorConfig::default());
        assert_eq!(predictor.config.d_model, 384);
        assert!(predictor.num_params() > 0);
    }

    #[test]
    fn test_predictor_forward() {
        let predictor = Predictor::default_with_dim(4);

        let context = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32, 5.0_f32, 6.0_f32, 7.0_f32, 8.0_f32];
        let target_positions = vec![0, 2];

        let predicted = predictor.forward(&context, &target_positions);

        // Should have 2 * 4 = 8 elements
        assert_eq!(predicted.len(), 8);
    }

    #[test]
    fn test_predictor_compute_loss() {
        let predictor = Predictor::default_with_dim(4);

        let predicted = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32];
        let target = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32];

        let loss = predictor.compute_loss(&predicted, &target);
        assert_eq!(loss, 0.0);
    }

    #[test]
    fn test_predictor_compute_loss_nonzero() {
        let predictor = Predictor::default_with_dim(4);

        let predicted = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32];
        let target = vec![2.0_f32, 3.0_f32, 4.0_f32, 5.0_f32];

        let loss = predictor.compute_loss(&predicted, &target);
        assert!(loss > 0.0);
        assert_eq!(loss, 1.0);
    }

    #[test]
    fn test_predictor_predict() {
        let predictor = Predictor::default_with_dim(4);

        let context = vec![1.0_f32; 16];
        let target_positions = vec![0, 1];
        let target_embeddings = vec![2.0_f32; 8];

        let output = predictor.predict(&context, &target_positions, &target_embeddings);

        assert_eq!(output.predicted.len(), 8);
        assert_eq!(output.target.len(), 8);
        assert!(output.loss > 0.0);
    }

    #[test]
    fn test_reshape_to_matrix() {
        let flat = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32, 5.0_f32, 6.0_f32];
        let matrix = reshape_to_matrix(&flat, 3);

        assert_eq!(matrix.len(), 2);
        assert_eq!(matrix[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(matrix[1], vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_flatten_matrix() {
        let matrix = vec![
            vec![1.0_f32, 2.0_f32],
            vec![3.0_f32, 4.0_f32],
            vec![5.0_f32, 6.0_f32],
        ];

        let flat = flatten_matrix(&matrix);

        assert_eq!(flat, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_reshape_flatten_roundtrip() {
        let original = vec![1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32, 5.0_f32, 6.0_f32];

        let matrix = reshape_to_matrix(&original, 2);
        let recovered = flatten_matrix(&matrix);

        assert_eq!(recovered, original);
    }

    #[test]
    fn test_prediction_output_new() {
        let pred = vec![1.0_f32, 2.0_f32];
        let target = vec![2.0_f32, 3.0_f32];
        let loss = 0.5;

        let output = PredictionOutput::new(pred.clone(), target.clone(), loss);

        assert_eq!(output.predicted, pred);
        assert_eq!(output.target, target);
        assert_eq!(output.loss, loss);
    }

    #[test]
    fn test_predictor_out_of_bounds_position() {
        let predictor = Predictor::default_with_dim(4);

        let context = vec![1.0_f32; 8];
        let target_positions = vec![10, 20]; // Out of bounds

        let predicted = predictor.forward(&context, &target_positions);

        // Should still produce output (wraps around)
        assert_eq!(predicted.len(), 8);
        // Values should be close to context values (1.0 + small perturbation)
        assert!(predicted.iter().all(|&x| (x - 1.0).abs() < 10.0));
    }

    #[test]
    fn test_predictor_custom_config() {
        let config = PredictorConfig {
            d_model: 128,
            hidden_dim: 64,
            num_layers: 3,
        };

        let predictor = Predictor::new(config);

        assert_eq!(predictor.config().d_model, 128);
        assert_eq!(predictor.config().hidden_dim, 64);
        assert_eq!(predictor.config().num_layers, 3);
    }

    #[test]
    fn test_predictor_empty_positions() {
        let predictor = Predictor::default_with_dim(4);

        let context = vec![1.0_f32; 8];
        let target_positions = vec![];

        let predicted = predictor.forward(&context, &target_positions);

        assert_eq!(predicted.len(), 0);
    }

    #[test]
    fn test_predictor_single_target() {
        let predictor = Predictor::default_with_dim(4);

        let context = vec![1.0_f32; 12];
        let target_positions = vec![1];

        let predicted = predictor.forward(&context, &target_positions);

        assert_eq!(predicted.len(), 4);
    }
}
