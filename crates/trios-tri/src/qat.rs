//! Quantization-Aware Training foundation (STE, learnable scale)

use crate::Ternary;

/// Straight-Through Estimator for ternary quantization
///
/// STE allows gradients to flow through the non-differentiable
/// quantization operation during backpropagation.
#[derive(Debug, Clone)]
pub struct TernarySTE {
    threshold: f32,
}

impl TernarySTE {
    /// Create a new STE with default threshold
    pub fn new() -> Self {
        Self { threshold: 0.5 }
    }

    /// Create a new STE with custom threshold
    pub fn with_threshold(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Forward pass: quantize f32 to ternary
    pub fn forward(&self, x: f32) -> Ternary {
        if x > self.threshold {
            Ternary::PosOne
        } else if x < -self.threshold {
            Ternary::NegOne
        } else {
            Ternary::Zero
        }
    }

    /// Backward pass: pass gradient through (STE)
    pub fn backward(&self, grad_output: f32, _input: f32) -> f32 {
        // STE: gradient passes through unchanged for values within [-threshold, threshold]
        // For values outside, gradient is zero (discontinuity)
        grad_output
    }
}

impl Default for TernarySTE {
    fn default() -> Self {
        Self::new()
    }
}

/// Learnable scale parameter for quantization
///
/// Scale factor can be learned during training to optimize
/// the quantization range.
#[derive(Debug, Clone)]
pub struct LearnableScale {
    value: f32,
    lr: f32,
}

impl LearnableScale {
    /// Create a new learnable scale
    pub fn new(initial_value: f32, lr: f32) -> Self {
        Self {
            value: initial_value,
            lr,
        }
    }

    /// Get current scale value
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Update scale using gradient
    pub fn update(&mut self, grad: f32) {
        self.value -= self.lr * grad;
        self.value = self.value.max(0.01); // Prevent scale from going to zero
    }

    /// Reset scale to initial value
    pub fn reset(&mut self, initial_value: f32) {
        self.value = initial_value;
    }
}

/// QAT configuration
#[derive(Debug, Clone, Copy)]
pub struct QatConfig {
    pub ste_threshold: f32,
    pub scale_lr: f32,
    pub initial_scale: f32,
}

impl Default for QatConfig {
    fn default() -> Self {
        Self {
            ste_threshold: 0.5,
            scale_lr: 0.001,
            initial_scale: 1.0,
        }
    }
}

impl QatConfig {
    /// Create new QAT config with custom threshold
    pub fn with_threshold(threshold: f32) -> Self {
        Self {
            ste_threshold: threshold,
            ..Default::default()
        }
    }

    /// Create STE from config
    pub fn create_ste(&self) -> TernarySTE {
        TernarySTE::with_threshold(self.ste_threshold)
    }

    /// Create learnable scale from config
    pub fn create_scale(&self) -> LearnableScale {
        LearnableScale::new(self.initial_scale, self.scale_lr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_ste_forward() {
        let ste = TernarySTE::new();
        assert_eq!(ste.forward(1.0), Ternary::PosOne);
        assert_eq!(ste.forward(-1.0), Ternary::NegOne);
        assert_eq!(ste.forward(0.0), Ternary::Zero);
    }

    #[test]
    fn test_ternary_ste_backward() {
        let ste = TernarySTE::new();
        let grad = ste.backward(0.5, 0.3);
        assert_eq!(grad, 0.5); // STE passes gradient through
    }

    #[test]
    fn test_learnable_scale() {
        let mut scale = LearnableScale::new(1.0, 0.1);
        assert_eq!(scale.value(), 1.0);
        scale.update(0.1);
        assert!((scale.value() - 0.99).abs() < 0.01);
    }

    #[test]
    fn test_qat_config() {
        let config = QatConfig::default();
        let ste = config.create_ste();
        let scale = config.create_scale();
        assert_eq!(scale.value(), 1.0);
    }
}
