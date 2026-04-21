//! Muon optimizer — zeroth-order optimizer for IGLA-NEEDLE
//!
//! Muon is a zeroth-order optimizer that applies orthogonal transformations to gradients
//! using Newton-Schulz iteration, allowing for larger learning rates on matrix parameters.
//!
//! References:
//! - https://arxiv.org/abs/2310.08658 (Muon Optimizer)
//! - https://github.com/bastings/muon

/// Muon optimizer for matrix parameters
///
/// Uses Newton-Schulz iteration to zeroth-order approximate the gradient's
/// pseudoinverse, allowing for larger learning rates on weight matrices.
#[derive(Debug, Clone)]
pub struct Muon {
    /// Learning rate for matrix parameters
    pub matrix_lr: f32,

    /// Momentum coefficient for velocity tracking
    pub momentum: f32,

    /// Weight decay coefficient
    pub weight_decay: f32,

    /// Number of Newton-Schulz iterations per step
    pub backend_steps: usize,

    /// Nesterov momentum flag
    pub nesterov: bool,

    /// Momentum warmup start value
    pub momentum_warmup_start: f32,

    /// Momentum warmup steps
    pub momentum_warmup_steps: usize,

    /// Current step counter
    step: usize,

    /// Velocity buffers for each parameter matrix
    velocity: Vec<Vec<f32>>,
}

impl Muon {
    /// Create a new Muon optimizer
    ///
    /// # Arguments
    ///
    /// * `param_shapes` - Shapes of matrix parameters (rows, cols)
    /// * `matrix_lr` - Learning rate for matrix parameters
    /// * `momentum` - Momentum coefficient (default: 0.99)
    /// * `backend_steps` - Newton-Schulz iterations (default: 5)
    /// * `weight_decay` - Weight decay coefficient (default: 0.04)
    pub fn new(
        param_shapes: &[(usize, usize)],
        matrix_lr: f32,
        momentum: f32,
        backend_steps: usize,
        weight_decay: f32,
    ) -> Self {
        let total_params: usize = param_shapes.iter().map(|(r, c)| r * c).sum();

        Self {
            matrix_lr,
            momentum,
            weight_decay,
            backend_steps,
            nesterov: true,
            momentum_warmup_start: 0.92,
            momentum_warmup_steps: 1500,
            step: 0,
            velocity: vec![vec![0.0f32; total_params]],
        }
    }

    /// Create Muon with φ-based defaults
    ///
    /// - matrix_lr = α_φ ≈ 0.11803
    /// - momentum = φ^(-1) ≈ 0.618
    /// - backend_steps = 5
    /// - weight_decay = 1/φ^4 ≈ 0.1459
    pub fn with_phi_defaults(param_shapes: &[(usize, usize)]) -> Self {
        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;
        let matrix_lr = (1.0 / (phi * phi * phi)) as f32; // α_φ ≈ 0.118
        let momentum = (1.0 / phi) as f32; // φ^(-1) ≈ 0.618
        let weight_decay = (1.0 / (phi.powi(4))) as f32; // 1/φ^4 ≈ 0.1459

        Self::new(param_shapes, matrix_lr, momentum, 5, weight_decay)
    }

    /// Create Muon with custom weight decay for sweep
    ///
    /// ALFA experiment: sweep weight_decay in [0.02, 0.03, 0.04, 0.05, 0.06]
    pub fn with_weight_decay(param_shapes: &[(usize, usize)], weight_decay: f32) -> Self {
        Self::new(param_shapes, 0.02, 0.99, 5, weight_decay)
    }

    /// Perform a single optimization step
    ///
    /// # Arguments
    ///
    /// * `params` - Matrix parameters (flattened, modified in-place)
    /// * `gradients` - Gradients for the parameters (flattened)
    /// * `param_shapes` - Original shapes for reshaping
    pub fn step(
        &mut self,
        params: &mut [f32],
        gradients: &[f32],
        param_shapes: &[(usize, usize)],
    ) {
        self.step += 1;

        // Warmup momentum
        let frac = if self.momentum_warmup_steps > 0 {
            (self.step as f32 / self.momentum_warmup_steps as f32).min(1.0)
        } else {
            1.0
        };
        let current_momentum = (1.0 - frac) * self.momentum_warmup_start + frac * self.momentum;

        let mut offset = 0;
        for &(rows, cols) in param_shapes {
            let size = rows * cols;
            let param_slice = &mut params[offset..offset + size];
            let grad_slice = &gradients[offset..offset + size];

            // Update momentum buffer
            for (i, (p, g)) in param_slice.iter_mut().zip(grad_slice.iter()).enumerate() {
                self.velocity[offset + i] = current_momentum * self.velocity[offset + i] + g;
            }

            // Apply Nesterov momentum if enabled
            let effective_grad = if self.nesterov {
                let mut nesterov_grad = vec![0.0f32; size];
                for (i, (v, g)) in nesterov_grad.iter_mut().zip(
                    self.velocity[offset..offset + size].iter().zip(grad_slice.iter()),
                ) {
                    *i = g + current_momentum * v;
                }
                nesterov_grad
            } else {
                self.velocity[offset..offset + size].to_vec()
            };

            // Apply zeroth-order update via Newton-Schulz
            let update = zeropower_via_newtonschulz(&effective_grad, rows, cols, self.backend_steps);

            // Update parameters
            for (p, u) in param_slice.iter_mut().zip(update.iter()) {
                // Apply weight decay
                *p *= 1.0 - self.weight_decay;
                // Apply Newton-Schulz update
                *p -= self.matrix_lr * u;
            }

            offset += size;
        }
    }

    /// Get current momentum value (for logging)
    pub fn current_momentum(&self) -> f32 {
        let frac = if self.momentum_warmup_steps > 0 {
            (self.step as f32 / self.momentum_warmup_steps as f32).min(1.0)
        } else {
            1.0
        };
        (1.0 - frac) * self.momentum_warmup_start + frac * self.momentum
    }

    /// Reset optimizer state
    pub fn reset(&mut self) {
        self.step = 0;
        for v in &mut self.velocity {
            v.fill(0.0);
        }
    }
}

/// Zeroth-order power via Newton-Schulz iteration
///
/// Approximates the pseudoinverse of a gradient matrix G:
///   Z = G * (G^T * G)^(-0.5)
///
/// Uses Newton-Schulz (5th order) iteration for fast convergence.
///
/// # Arguments
///
/// * `grad` - Gradient matrix (flattened row-major)
/// * `rows` - Number of rows
/// * `cols` - Number of columns
/// * `steps` - Number of Newton-Schulz iterations
///
/// # Returns
///
/// Zeroth-order update vector (flattened)
fn zeropower_via_newtonschulz(
    grad: &[f32],
    rows: usize,
    cols: usize,
    steps: usize,
) -> Vec<f32> {
    if grad.is_empty() {
        return Vec::new();
    }

    // Coefficients for 5th order Newton-Schulz
    let a = 3.4445f32;
    let b = -4.7750f32;
    let c = 2.0315f32;

    // Reshape gradient to matrix (row-major)
    let mut x: Vec<Vec<f32>> = vec![vec![0.0; cols]; rows];
    for i in 0..rows {
        for j in 0..cols {
            x[i][j] = grad[i * cols + j];
        }
    }

    // Normalize by Frobenius norm
    let norm: f32 = x.iter().flatten().map(|&v| v * v).sum::<f32>().sqrt();
    let eps = 1e-7f32;
    if norm > eps {
        for row in &mut x {
            for v in row {
                *v /= norm;
            }
        }
    }

    // Transpose flag for efficiency
    let transposed = rows > cols;

    // Newton-Schulz iteration
    for _ in 0..steps {
        // A = X * X^T
        let mut a: Vec<Vec<f32>> = vec![vec![0.0; rows]; rows];
        for i in 0..rows {
            for j in 0..rows {
                for k in 0..cols {
                    a[i][j] += x[i][k] * x[j][k];
                }
            }
        }

        // B = b*A + c*A*A
        let mut b_mat: Vec<Vec<f32>> = vec![vec![0.0; rows]; rows];
        for i in 0..rows {
            for j in 0..rows {
                let aa: f32 = (0..rows)
                    .map(|k| a[i][k] * a[k][j])
                    .sum();
                b_mat[i][j] = b * a[i][j] + c * aa;
            }
        }

        // X = a*X + B*X
        let mut new_x: Vec<Vec<f32>> = vec![vec![0.0; cols]; rows];
        for i in 0..rows {
            for j in 0..cols {
                let mut bx = 0.0f32;
                for k in 0..rows {
                    bx += b_mat[i][k] * x[k][j];
                }
                new_x[i][j] = a * x[i][j] + bx;
            }
        }
        x = new_x;
    }

    // Transpose back if needed and flatten
    if transposed {
        let mut result = vec![0.0f32; rows * cols];
        for i in 0..cols {
            for j in 0..rows {
                result[j * cols + i] = x[i][j];
            }
        }
        result
    } else {
        x.into_iter().flatten().collect()
    }
}

/// Weight decay sweep values for ALFA experiment IGLA-MUON-105
pub const MUON_WD_SWEEP: &[f32] = &[0.02, 0.03, 0.04, 0.05, 0.06];

/// Get optimal weight decay from sweep results (placeholder)
///
/// In production, this would analyze BPB results from sweeps
pub fn optimal_weight_decay(_results: &[(f32, f32)]) -> f32 {
    // Placeholder: return middle value
    MUON_WD_SWEEP[MUON_WD_SWEEP.len() / 2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_muon_phi_defaults() {
        let shapes = [(64, 64), (128, 64)];
        let muon = Muon::with_phi_defaults(&shapes);

        let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;

        // matrix_lr = α_φ = 1/φ^3 ≈ 0.118
        let expected_lr = (1.0 / (phi * phi * phi)) as f32;
        assert!((muon.matrix_lr - expected_lr).abs() < 0.001);

        // momentum = φ^(-1) ≈ 0.618
        let expected_momentum = (1.0 / phi) as f32;
        assert!((muon.momentum - expected_momentum).abs() < 0.001);
    }

    #[test]
    fn test_muon_weight_decay_sweep() {
        let shapes = [(64, 64)];
        for &wd in MUON_WD_SWEEP {
            let muon = Muon::with_weight_decay(&shapes, wd);
            assert_eq!(muon.weight_decay, wd);
        }
    }

    #[test]
    fn test_muon_step_modifies_params() {
        let shapes = [(2, 2)];
        let mut params = vec![1.0f32; 4];
        let gradients = vec![0.1f32; 4];
        let mut muon = Muon::new(&shapes, 0.01, 0.9, 1, 0.01);

        let initial = params.clone();
        muon.step(&mut params, &gradients, &shapes);

        // Parameters should change
        assert_ne!(params, initial);
    }

    #[test]
    fn test_momentum_warmup() {
        let shapes = [(2, 2)];
        let muon = Muon::with_phi_defaults(&shapes);

        // At step 0, momentum should be at warmup start
        assert_eq!(muon.current_momentum(), muon.momentum_warmup_start);

        // At step 1500, momentum should be at target
        muon.step = 1500;
        let final_momentum = muon.current_momentum();
        assert!((final_momentum - muon.momentum).abs() < 0.01);
    }

    #[test]
    fn test_zeropower_normalization() {
        let grad = vec![2.0f32, 0.0, 0.0, 0.0];
        let result = zeropower_via_newtonschulz(&grad, 2, 2, 1);

        // Result should be normalized
        let norm: f32 = result.iter().map(|&v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_zeropower_identity() {
        // Identity matrix gradient should remain close to identity
        let grad = vec![1.0, 0.0, 0.0, 1.0];
        let result = zeropower_via_newtonschulz(&grad, 2, 2, 5);

        // Result should preserve diagonal dominance
        assert!(result[0].abs() > result[1].abs());
        assert!(result[3].abs() > result[2].abs());
    }

    #[test]
    fn test_muon_reset() {
        let shapes = [(2, 2)];
        let mut params = vec![1.0f32; 4];
        let gradients = vec![0.1f32; 4];
        let mut muon = Muon::new(&shapes, 0.01, 0.9, 1, 0.01);

        muon.step(&mut params, &gradients, &shapes);
        assert!(!muon.velocity[0].iter().all(|&v| v == 0.0));

        muon.reset();
        assert_eq!(muon.step, 0);
        assert!(muon.velocity[0].iter().all(|&v| v == 0.0));
    }
}
