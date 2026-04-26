//! L-f2: Gate-final Trainer Extensions
//!
//! This module provides the Gate-final extensions to the hybrid trainer:
//! - φ-scaled hidden width: round(φ * 512) = 828
//! - 3-seed loop on seeds {42, 43, 44}
//! - EMA with β = φ⁻¹
//! - GF16 floor activation from step 56700
//! - Schedule extension to 81K steps (cosine from 54K)
//!
//! Refs: trios#143 Gate-final DRAFT §6, L-f2

use std::f64::consts::PI;

// ═══════════════════════════════════════════════════════════════════
// Gate-final Constants (pre-registered)
// ═══════════════════════════════════════════════════════════════════

/// φ = (1 + √5) / 2
pub const PHI: f64 = 1.618_033_988_749_895;

/// φ-scaled hidden width for n-gram block: round(φ * 512) = 828
/// Coq: trinity-clara/proofs/igla/golden_width.v (L-f2 reference)
pub const PHI_SCALED_HIDDEN: usize = 828;

/// EMA decay factor β = φ⁻¹ ≈ 0.618
/// Coq: trinity-clara/proofs/igla/ema_stability.v (INV-6)
pub const EMA_BETA: f64 = PHI.recip(); // φ⁻¹

/// GF16 weight floor activation step: floor(0.7 * 81000) = 56700
/// This is the last 30% of training where GF16 quantization becomes active
pub const GF16_FLOOR_STEP: usize = 56700;

/// Gate-final max steps: 81K ≈ φ³ * 30K
/// Schedule: cosine warm-restart at 54K (Gate-2 checkpoint)
pub const GATE_FINAL_MAX_STEPS: usize = 81000;

/// Gate-2 checkpoint step (cosine warm-restart point)
pub const GATE_2_CHECKPOINT: usize = 54000;

/// Valid seeds for Gate-final 3-seed sweep
pub const VALID_SEEDS: [u64; 3] = [42, 43, 44];

// ═══════════════════════════════════════════════════════════════════
// EMA Tracker (INV-6)
// ═══════════════════════════════════════════════════════════════════

/// EMA tracker for validation BPB (INV-6)
#[derive(Debug, Clone)]
pub struct EmaTracker {
    ema: f64,
    beta: f64,
    initialized: bool,
}

impl EmaTracker {
    pub fn new(beta: f64) -> Self {
        Self {
            ema: 0.0,
            beta,
            initialized: false,
        }
    }

    pub fn update(&mut self, value: f64) -> f64 {
        if !self.initialized {
            self.ema = value;
            self.initialized = true;
        } else {
            self.ema = self.beta * self.ema + (1.0 - self.beta) * value;
        }
        self.ema
    }

    pub fn get(&self) -> f64 {
        self.ema
    }

    pub fn variance_reduction(&self, raw_history: &[f64]) -> f64 {
        if raw_history.is_empty() {
            return 0.0;
        }
        let raw_var: f64 = raw_history.iter()
            .map(|x| (x - raw_history.iter().sum::<f64>() / raw_history.len() as f64).powi(2))
            .sum::<f64>() / (raw_history.len() - 1).max(1) as f64;
        let ema_var = (raw_history.last().unwrap() - self.ema).powi(2);
        // Return ratio: < 1.0 means EMA reduces variance
        if raw_var > 0.0 {
            ema_var / raw_var
        } else {
            1.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Cosine LR Schedule (extended to 81K)
// ═══════════════════════════════════════════════════════════════════

/// Cosine learning rate schedule with warm-restart at Gate-2 checkpoint
pub fn get_cosine_lr(
    step: usize,
    max_steps: usize,
    warmup_steps: usize,
    base_lr: f64,
    min_lr: f64,
    checkpoint_step: Option<usize>,
) -> f64 {
    let effective_max = if let Some(cp) = checkpoint_step {
        // If we're past the checkpoint, treat it as a new cosine cycle
        if step >= cp {
            // Warm-restart: normalize from checkpoint to max_steps
            let remaining = max_steps - cp;
            let elapsed = step - cp;
            let progress = (elapsed as f64) / (remaining.max(1) as f64);
            // Cosine decay from checkpoint
            min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (progress * PI).cos())
        } else {
            // Before checkpoint: standard cosine to checkpoint
            let progress = (step as f64) / (cp as f64);
            min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (progress * PI).cos())
        }
    } else {
        // Standard cosine decay
        let progress = (step as f64) / (max_steps as f64);
        min_lr + 0.5 * (base_lr - min_lr) * (1.0 + (progress * PI).cos())
    };

    // Warmup: linearly increase from 0 to base_lr
    if step < warmup_steps {
        base_lr * (step as f64) / (warmup_steps as f64)
    } else {
        effective_max
    }
}

/// Gate-final specific cosine LR schedule
pub fn gate_final_lr(step: usize, base_lr: f64) -> f64 {
    get_cosine_lr(
        step,
        GATE_FINAL_MAX_STEPS,
        3000, // warmup steps
        base_lr,
        0.0001, // min_lr
        Some(GATE_2_CHECKPOINT), // warm-restart at Gate-2 checkpoint
    )
}

// ═══════════════════════════════════════════════════════════════════
// GF16 Floor (lever 4)
// ═══════════════════════════════════════════════════════════════════

/// Check if GF16 weight floor should be active at this step
pub fn gf16_floor_active(step: usize) -> bool {
    step >= GF16_FLOOR_STEP
}

/// Apply GF16 quantization floor to weights
/// This quantizes weights to GF(16) representation during the final 30%
pub fn apply_gf16_floor(weights: &mut [f32]) {
    for w in weights.iter_mut() {
        *w = (*w * 256.0).round() / 256.0;
    }
}

// ═══════════════════════════════════════════════════════════════════
// 3-Seed Loop (lever 6)
// ═══════════════════════════════════════════════════════════════════

/// Run training loop across 3 seeds with ASHA promotion logic
/// 
/// Per INV-2 (Proven): promote only configs that survive on >= 2/3 seeds
pub fn run_3_seed_loop<F>(mut train_fn: F) -> Vec<(u64, f64)>
where
    F: FnMut(u64) -> f64,
{
    let mut results = Vec::new();
    
    for &seed in &VALID_SEEDS {
        let final_bpb = train_fn(seed);
        results.push((seed, final_bpb));
    }
    
    results
}

/// Check ASHA promotion criteria: config must survive on >= 2/3 seeds
pub fn check_asha_promotion(results: &[(u64, f64)], bpb_threshold: f64) -> bool {
    let survivors = results.iter()
        .filter(|(_, bpb)| *bpb < bpb_threshold)
        .count();
    survivors * 3 >= results.len() * 2 // >= 2/3
}

// ═══════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phi_scaled_hidden() {
        assert_eq!(PHI_SCALED_HIDDEN, 828);
        // Verify: round(1.618... * 512) = round(828.9...) = 828
    }

    #[test]
    fn test_ema_tracker() {
        let mut tracker = EmaTracker::new(EMA_BETA);
        
        // First value initializes
        let ema1 = tracker.update(2.0);
        assert_eq!(ema1, 2.0);
        
        // Second value is weighted average
        let ema2 = tracker.update(1.0);
        assert!(ema2 > 1.0 && ema2 < 2.0);
        
        // Verify EMA reduces variance
        let history = vec![2.0, 1.8, 2.2, 1.5, 1.9];
        for &val in &history {
            tracker.update(val);
        }
        let ratio = tracker.variance_reduction(&history);
        assert!(ratio < 1.0, "EMA should reduce variance");
    }

    #[test]
    fn test_gate_final_lr_schedule() {
        // Check LR at various steps
        let lr_0 = gate_final_lr(0, 0.0035);
        assert_eq!(lr_0, 0.0, "LR should be 0 at step 0 (warmup)");
        
        let lr_3k = gate_final_lr(3000, 0.0035);
        assert!(lr_3k > 0.0, "LR should be > 0 after warmup");
        
        let lr_54k = gate_final_lr(54000, 0.0035);
        assert!(lr_54k > 0.0, "LR should be > 0 at Gate-2 checkpoint");
        
        let lr_81k = gate_final_lr(81000, 0.0035);
        assert!(lr_81k < 0.0035, "LR should decay to min_lr at end");
    }

    #[test]
    fn test_gf16_floor_activation() {
        assert!(!gf16_floor_active(50000), "Should not be active before 56700");
        assert!(gf16_floor_active(56700), "Should be active at exactly 56700");
        assert!(gf16_floor_active(80000), "Should be active after 56700");
    }

    #[test]
    fn test_apply_gf16_floor() {
        let mut weights = vec![0.123456789, 0.987654321, -0.5];
        let original = weights.clone();
        
        apply_gf16_floor(&mut weights);
        
        // Weights should be quantized to ~1/256 precision
        for (orig, quantized) in original.iter().zip(weights.iter()) {
            let diff = (orig - quantized).abs();
            assert!(diff < 1.0 / 256.0, "GF16 floor should quantize to 1/256 precision");
        }
    }

    #[test]
    fn test_check_asha_promotion() {
        // All 3 seeds below threshold -> should promote
        let results_all_good = [(42, 1.4), (43, 1.45), (44, 1.48)];
        assert!(check_asha_promotion(&results_all_good, 1.50));
        
        // Only 1 seed below threshold -> should not promote
        let results_one_good = [(42, 1.4), (43, 1.60), (44, 1.70)];
        assert!(!check_asha_promotion(&results_one_good, 1.50));
        
        // 2 seeds below threshold -> should promote (exactly 2/3)
        let results_two_good = [(42, 1.4), (43, 1.45), (44, 1.60)];
        assert!(check_asha_promotion(&results_two_good, 1.50));
    }
}
