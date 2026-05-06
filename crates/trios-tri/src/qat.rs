//! Quantization-Aware Training foundation (∓)
//!
//! Φ3: STE (Straight-Through Estimator) for gradient flow through ternarization.
//!
//! This module provides the foundation for QAT with ternary weights.
//! Full implementation is deferred to training phase (Priority 3 per IGLA-GF16 synthesis).

use super::Ternary;

/// Straight-Through Estimator for ternary gradients.
///
/// During forward pass, weights are ternarized to {-1, 0, +1}.
/// During backward pass, gradients pass through unchanged (identity),
/// allowing training to learn meaningful scales despite discrete weights.
///
/// # Theory
/// The STE approximates the gradient of the non-differentiable ternarization:
/// `dL/dw ≈ dL/dq` (where q is the ternarized weight)
///
/// # Example
/// ```
/// use trios_tri::qat::TernarySTE;
///
/// let ste = TernarySTE::new(0.5);
/// let ternary = ste.forward(0.8);
/// let grad = ste.backward(0.1); // identity: grad_in = grad_out
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TernarySTE {
    /// Threshold for ternarization
    /// Values > threshold become +1, < -threshold become -1, else 0
    threshold: f32,
}

impl TernarySTE {
    /// Create a new TernarySTE with the given threshold.
    ///
    /// # Arguments
    /// * `threshold` - Threshold value (typically 0.5)
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    pub fn with_default_threshold() -> Self {
        Self { threshold: 0.5 }
    }

    /// Forward pass: ternarize the input.
    ///
    /// # Arguments
    /// * `x` - Input value (typically a weight)
    ///
    /// # Returns
    /// Ternarized value
    pub fn forward(&self, x: f32) -> Ternary {
        if x > self.threshold {
            Ternary::PosOne
        } else if x < -self.threshold {
            Ternary::NegOne
        } else {
            Ternary::Zero
        }
    }

    /// Backward pass: identity gradient (STE).
    ///
    /// The gradient passes through unchanged, approximating
    /// the gradient of the discontinuous ternarization function.
    ///
    /// # Arguments
    /// * `grad` - Incoming gradient
    ///
    /// # Returns
    /// Same gradient (identity)
    #[inline]
    pub fn backward(&self, grad: f32) -> f32 {
        grad // Identity: gradient passes through unchanged
    }

    /// Get the current threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Set a new threshold.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold;
    }

    /// Forward pass for a batch of values.
    ///
    /// # Arguments
    /// * `xs` - Input values
    ///
    /// # Returns
    /// Vector of ternarized values
    pub fn forward_batch(&self, xs: &[f32]) -> Vec<Ternary> {
        xs.iter().map(|&x| self.forward(x)).collect()
    }

    /// Backward pass for a batch of gradients.
    ///
    /// # Arguments
    /// * `grads` - Incoming gradients
    ///
    /// # Returns
    /// Same gradients (identity for each)
    pub fn backward_batch(&self, grads: &[f32]) -> Vec<f32> {
        grads.to_vec() // Identity: same gradients
    }
}

impl Default for TernarySTE {
    fn default() -> Self {
        Self { threshold: 0.5 }
    }
}

/// Learnable scale factor for ternary quantization.
///
/// During training, the scale factor is learned to optimize
/// the dynamic range preservation of ternarized weights.
///
/// # Theory
/// The scale factor transforms the weight space: `w_ternary = ternarize(w * scale)`.
/// A well-learned scale ensures that significant weights
/// are mapped to ±1 while noise is mapped to 0.
///
/// # Example
/// ```
/// use trios_tri::qat::LearnableScale;
///
/// let mut scale = LearnableScale::new(1.0);
/// scale.update(0.1, 0.01); // gradient 0.1, learning rate 0.01
/// ```
#[derive(Debug, Clone)]
pub struct LearnableScale {
    /// Current scale value
    value: f32,
    /// Minimum allowed scale (prevents division by zero)
    min_scale: f32,
}

impl LearnableScale {
    /// Create a new learnable scale with initial value.
    ///
    /// # Arguments
    /// * `initial` - Initial scale value (must be positive)
    ///
    /// # Panics
    /// Panics if `initial <= 0`
    pub fn new(initial: f32) -> Self {
        assert!(initial > 0.0, "scale must be positive");
        Self {
            value: initial,
            min_scale: 1e-6,
        }
    }

    /// Create with custom minimum scale.
    ///
    /// # Arguments
    /// * `initial` - Initial scale value
    /// * `min_scale` - Minimum allowed scale
    pub fn with_min_scale(initial: f32, min_scale: f32) -> Self {
        assert!(initial > 0.0, "scale must be positive");
        assert!(min_scale > 0.0, "min_scale must be positive");
        Self {
            value: initial,
            min_scale,
        }
    }

    /// Get the current scale value.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Update the scale using gradient descent.
    ///
    /// Uses standard gradient descent: `value -= gradient * learning_rate`.
    /// A positive gradient decreases the scale; a negative gradient increases it.
    ///
    /// # Arguments
    /// * `gradient` - Gradient with respect to the scale
    /// * `learning_rate` - Learning rate for the update
    ///
    /// # Example
    /// ```
    /// use trios_tri::qat::LearnableScale;
    ///
    /// let mut scale = LearnableScale::new(1.0);
    /// scale.update(0.1, 0.01); // Decrease scale with positive gradient
    /// assert!(scale.value() < 1.0);
    /// ```
    pub fn update(&mut self, gradient: f32, learning_rate: f32) {
        self.value -= gradient * learning_rate;
        // Clamp to minimum scale
        self.value = self.value.max(self.min_scale);
    }

    /// Apply the scale to a value.
    ///
    /// # Arguments
    /// * `x` - Input value
    ///
    /// # Returns
    /// Scaled value
    #[inline]
    pub fn apply(&self, x: f32) -> f32 {
        x * self.value
    }

    /// Invert the scale (divide by scale).
    ///
    /// # Arguments
    /// * `x` - Input value
    ///
    /// # Returns
    /// Value divided by scale
    #[inline]
    pub fn invert(&self, x: f32) -> f32 {
        x / self.value
    }

    /// Apply scale to a batch of values.
    pub fn apply_batch(&self, xs: &[f32]) -> Vec<f32> {
        xs.iter().map(|&x| self.apply(x)).collect()
    }

    /// Invert scale for a batch of values.
    pub fn invert_batch(&self, xs: &[f32]) -> Vec<f32> {
        xs.iter().map(|&x| self.invert(x)).collect()
    }

    /// Reset to initial value.
    pub fn reset(&mut self, initial: f32) {
        assert!(initial > 0.0, "scale must be positive");
        self.value = initial;
    }
}

/// QAT configuration for ternary training.
///
/// Bundles STE and learnable scale into a single configuration.
#[derive(Debug, Clone)]
pub struct QatConfig {
    /// Straight-Through Estimator
    pub ste: TernarySTE,
    /// Learnable scale factor
    pub scale: LearnableScale,
}

impl QatConfig {
    /// Create a new QAT configuration.
    ///
    /// # Arguments
    /// * `threshold` - STE threshold
    /// * `initial_scale` - Initial scale value
    pub fn new(threshold: f32, initial_scale: f32) -> Self {
        Self {
            ste: TernarySTE::new(threshold),
            scale: LearnableScale::new(initial_scale),
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            ste: TernarySTE::default(),
            scale: LearnableScale::new(1.0),
        }
    }

    /// Forward pass: apply scale and ternarize.
    ///
    /// # Arguments
    /// * `x` - Input value
    ///
    /// # Returns
    /// Ternarized value
    pub fn forward(&self, x: f32) -> Ternary {
        let scaled = self.scale.apply(x);
        self.ste.forward(scaled)
    }

    /// Backward pass: apply STE and scale gradient.
    ///
    /// # Arguments
    /// * `grad` - Incoming gradient
    ///
    /// # Returns
    /// Gradient w.r.t. input
    pub fn backward(&self, grad: f32) -> f32 {
        // STE identity * scale (chain rule)
        self.ste.backward(grad) * self.scale.value()
    }

    /// Update the learnable scale.
    ///
    /// # Arguments
    /// * `gradient` - Gradient w.r.t. scale
    /// * `learning_rate` - Learning rate
    pub fn update_scale(&mut self, gradient: f32, learning_rate: f32) {
        self.scale.update(gradient, learning_rate);
    }

    /// Update the STE threshold.
    ///
    /// # Arguments
    /// * `gradient` - Gradient w.r.t. threshold
    /// * `learning_rate` - Learning rate
    pub fn update_threshold(&mut self, gradient: f32, learning_rate: f32) {
        let old_threshold = self.ste.threshold();
        let new_threshold = old_threshold - gradient * learning_rate;
        self.ste.set_threshold(new_threshold.max(0.01)); // Prevent going too low
    }

    /// Forward pass for a batch.
    pub fn forward_batch(&self, xs: &[f32]) -> Vec<Ternary> {
        let scaled = self.scale.apply_batch(xs);
        self.ste.forward_batch(&scaled)
    }

    /// Dequantize: ternary back to f32 using current scale.
    ///
    /// # Arguments
    /// * `t` - Ternary value
    ///
    /// # Returns
    /// Dequantized f32 value
    pub fn dequantize(&self, t: Ternary) -> f32 {
        self.scale.invert(t.to_f32())
    }

    /// Dequantize a batch.
    pub fn dequantize_batch(&self, ts: &[Ternary]) -> Vec<f32> {
        ts.iter().map(|&t| self.dequantize(t)).collect()
    }
}

impl Default for QatConfig {
    fn default() -> Self {
        Self {
            ste: TernarySTE::default(),
            scale: LearnableScale::new(1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_ste_forward() {
        let ste = TernarySTE::new(0.5);

        assert_eq!(ste.forward(1.0), Ternary::PosOne);
        assert_eq!(ste.forward(0.8), Ternary::PosOne);
        assert_eq!(ste.forward(0.5), Ternary::Zero); // Equal to threshold
        assert_eq!(ste.forward(0.0), Ternary::Zero);
        assert_eq!(ste.forward(-0.5), Ternary::Zero);
        assert_eq!(ste.forward(-0.8), Ternary::NegOne);
        assert_eq!(ste.forward(-1.0), Ternary::NegOne);
    }

    #[test]
    fn test_ternary_ste_backward() {
        let ste = TernarySTE::new(0.5);

        // Identity: gradient passes through unchanged
        assert_eq!(ste.backward(0.5), 0.5);
        assert_eq!(ste.backward(-0.3), -0.3);
        assert_eq!(ste.backward(1.0), 1.0);
    }

    #[test]
    fn test_ternary_ste_batch() {
        let ste = TernarySTE::new(0.5);
        let inputs = vec![1.0, 0.0, -1.0, 0.6];
        let output = ste.forward_batch(&inputs);

        assert_eq!(output.len(), 4);
        assert_eq!(output[0], Ternary::PosOne);
        assert_eq!(output[1], Ternary::Zero);
        assert_eq!(output[2], Ternary::NegOne);
        assert_eq!(output[3], Ternary::PosOne);
    }

    #[test]
    fn test_learnable_scale() {
        let mut scale = LearnableScale::new(1.0);

        assert_eq!(scale.value(), 1.0);
        assert_eq!(scale.apply(2.0), 2.0);
        assert_eq!(scale.invert(2.0), 2.0);

        // Positive gradient decreases value (gradient descent: value -= grad * lr)
        scale.update(0.1, 0.01);
        assert!(scale.value() < 1.0);

        // Another positive gradient makes it even smaller
        scale.update(1.0, 0.01);
        assert!(scale.value() < 1.0);
    }

    #[test]
    fn test_learnable_scale_clamping() {
        let mut scale = LearnableScale::new(1.0);

        // Try to push below minimum with large positive gradient
        scale.update(100.0, 10.0);
        assert!(scale.value() >= scale.min_scale);
    }

    #[test]
    fn test_qat_config() {
        let config = QatConfig::default();

        // Forward: scale then ternarize
        let t = config.forward(1.0);
        assert_eq!(t, Ternary::PosOne);

        // Backward: STE identity * scale
        let grad = config.backward(0.5);
        assert_eq!(grad, 0.5); // 0.5 * 1.0

        // Dequantize
        let f32 = config.dequantize(Ternary::PosOne);
        assert_eq!(f32, 1.0);
    }

    #[test]
    fn test_qat_config_batch() {
        let config = QatConfig::new(0.5, 2.0);

        let inputs = vec![0.3, 0.8, -0.3, -0.8];
        let ternary = config.forward_batch(&inputs);

        // With scale=2.0: 0.3*2=0.6 → +1, 0.8*2=1.6 → +1, -0.3*2=-0.6 → -1, -0.8*2=-1.6 → -1
        assert_eq!(ternary[0], Ternary::PosOne);
        assert_eq!(ternary[1], Ternary::PosOne);
        assert_eq!(ternary[2], Ternary::NegOne);
        assert_eq!(ternary[3], Ternary::NegOne);

        // Dequantize back
        let f32s = config.dequantize_batch(&ternary);
        // ±1 / 2.0 = ±0.5
        assert_eq!(f32s[0], 0.5);
        assert_eq!(f32s[1], 0.5);
        assert_eq!(f32s[2], -0.5);
        assert_eq!(f32s[3], -0.5);
    }

    #[test]
    fn test_qat_update_scale() {
        let mut config = QatConfig::default();

        // Positive gradient decreases scale (gradient descent)
        config.update_scale(0.1, 0.01);
        assert!(config.scale.value() < 1.0);
    }

    #[test]
    fn test_qat_update_threshold() {
        let mut config = QatConfig::default();

        // Positive gradient decreases threshold
        config.update_threshold(0.1, 0.1);
        assert!(config.ste.threshold() < 0.5);

        // Try to go too low (should clamp)
        config.update_threshold(1.0, 10.0);
        assert!(config.ste.threshold() >= 0.01);
    }

    #[test]
    #[should_panic(expected = "scale must be positive")]
    fn test_learnable_scale_invalid_initial() {
        LearnableScale::new(-1.0);
    }

    #[test]
    #[should_panic(expected = "scale must be positive")]
    fn test_learnable_scale_invalid_min_scale() {
        LearnableScale::with_min_scale(1.0, -0.1);
    }
}
