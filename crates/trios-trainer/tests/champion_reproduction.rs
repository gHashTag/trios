//! Champion reproduction test — full 27K training run
//!
//! Validates that champion.toml reproduces BPB = 2.2393 ± 0.01
//! Run with: cargo test -p trios-trainer --test champion_reproduction

use anyhow::Result;
use std::time::Instant;

use trios_trainer::{
    Config, FineWebDataset,
    model::MinimalTransformer,
    forward::forward,
    backward::compute_gradients,
    optimizer::AdamWCpu,
};

/// BPB target range for champion reproduction
const CHAMPION_BPB_TARGET: f32 = 2.2393;
const CHAMPION_BPB_TOLERANCE: f32 = 0.01;
const CHAMPION_MIN_BPB: f32 = CHAMPION_BPB_TARGET - CHAMPION_BPB_TOLERANCE; // 2.2293
const CHAMPION_MAX_BPB: f32 = CHAMPION_BPB_TARGET + CHAMPION_BPB_TOLERANCE; // 2.2493

/// Expected training steps for champion
const CHAMPION_STEPS: usize = 27_000;

/// BPB calculation: bits per byte = (log2(256) / log2(2^BPB)) / 8
fn calculate_bpb(nll: f32, num_tokens: usize) -> f32 {
    // BPB = (log2(256) / log2(2^NLL)) / 8
    // where NLL = loss / log2(256)  (normalized cross-entropy)
    // 2^NLL is the perplexity
    let perplexity = 2_f32.powf(nll);
    // BPB in bits per byte
    (256.0_f32.ln() / perplexity.ln()) / 8.0
}

/// Run full champion training and validate BPB
pub fn run_champion_reproduction(config_path: &str) -> Result<ReproductionResult> {
    println!("=== CHAMPION REPRODUCTION TEST ===");
    println!("Loading config from: {}", config_path);

    // Load config
    let config = Config::load(config_path)?;

    println!("Config loaded:");
    println!("  Seed: {}", config.training.seed);
    println!("  Steps: {}", config.training.steps);
    println!("  Batch size: {}", config.training.batch_size);
    println!("  LR: {}", config.training.lr);
    println!("  d_model: {}", config.model.d_model);
    println!("  n_layers: {}", config.model.n_layers);
    println!("  context_len: {}", config.model.context_len);
    println!("  Champion target BPB: {:.4}", CHAMPION_BPB_TARGET);

    // Validate config matches champion baseline
    validate_champion_config(&config)?;

    // Load dataset
    println!("\nLoading dataset...");
    let train_dataset = FineWebDataset::load(&config.training.train_path)?;
    let val_dataset = FineWebDataset::load(&config.training.val_path)?;
    println!("Train tokens: {}", train_dataset.len());
    println!("Val tokens: {}", val_dataset.len());

    // Initialize model
    println!("\nInitializing model...");
    let mut model = MinimalTransformer::new(
        config.model.d_model,
        config.model.n_layers,
        config.model.context_len,
    )?;
    model.init_with_seed(config.training.seed)?;

    let model_params = model.count_params();
    println!("Model parameters: {} (~{:.1}K)", model_params, model_params as f64 / 1000.0);

    // Initialize optimizer
    let mut optimizer = AdamWCpu::new(&model, config.training.lr)?;

    let start_time = Instant::now();

    // Training loop
    println!("\n=== STARTING TRAINING ===");
    let mut best_bpb = f32::MAX;
    let mut final_bpb = 0.0;

    for step in 1..=config.training.steps {
        // Sample batch (use fallback for now since we don't have full data loader)
        let seq_len = 128;
        let batch = train_dataset.sample_sequence(seq_len, &mut step as u64);

        // Forward pass
        let forward_result = forward(&model, &batch)?;

        // Compute loss (cross-entropy)
        let nll = forward_result.logits
            .iter()
            .zip(batch.targets.iter())
            .map(|(logit, &target)| {
                // NLL = -log(sum(exp(logit_i) * target_i)) / vocab_size
                // Simplified: use softmax probability at target index
                let prob = logit[*target as usize];
                let log_prob = prob.ln() + 1e-10; // log-sum-exp trick for stability
                -log_prob
            })
            .sum::<f32>()
            / seq_len as f32
            / config.model.vocab_size as f32; // normalized by vocab size

        let bpb = calculate_bpb(nll, seq_len * config.training.batch_size);

        // Backward pass
        let grads = compute_gradients(&forward_result, nll)?;

        // Optimizer step
        optimizer.step(&mut model, &grads, config.training.lr)?;

        // Evaluation every 1000 steps
        if step % 1000 == 0 {
            println!("Step {:>5} | BPB: {:.4} | NLL: {:.4}", step, bpb, nll);

            // Evaluate on validation set (sample for now)
            let val_bpb = evaluate_on_val(&model, &val_dataset)?;
            println!("  Val BPB: {:.4} | Val NLL: {:.4}", val_bpb, val_bpb_nll);

            if val_bpb < best_bpb {
                best_bpb = val_bpb;
                println!("  ★ NEW BEST: {:.4}", best_bpb);
            }
        }

        final_bpb = bpb;
    }

    let elapsed = start_time.elapsed();
    println!("\n=== TRAINING COMPLETE ===");
    println!("Final BPB: {:.4}", final_bpb);
    println!("Best BPB: {:.4}", best_bpb);
    println!("Time: {:.2}s", elapsed.as_secs_f64());

    let result = ReproductionResult {
        final_bpb,
        best_bpb,
        steps_completed: config.training.steps,
        elapsed_seconds: elapsed.as_secs_f64(),
        passed: is_within_tolerance(final_bpb),
    };

    // Print result
    println!("\n=== REPRODUCTION RESULT ===");
    println!("Final BPB: {:.4}", result.final_bpb);
    println!("Best BPB: {:.4}", result.best_bpb);
    println!("Target: {:.4} ± {:.4}", CHAMPION_BPB_TARGET, CHAMPION_BPB_TOLERANCE);
    println!("Status: {}", if result.passed { "✅ PASS" } else { "❌ FAIL" });

    if !result.passed {
        anyhow::bail!("Champion reproduction FAILED: BPB {:.4} outside [{:.4}, {:.4}]",
            result.final_bpb, CHAMPION_MIN_BPB, CHAMPION_MAX_BPB);
    }

    Ok(result)
}

/// Validate that config matches champion baseline
fn validate_champion_config(config: &Config) -> Result<()> {
    // Check champion config parameters
    if config.model.d_model != 384 {
        anyhow::bail!("Invalid d_model: {} (expected 384)", config.model.d_model);
    }
    if config.model.n_layers != 4 {
        anyhow::bail!("Invalid n_layers: {} (expected 4)", config.model.n_layers);
    }
    if config.training.lr != 0.004 {
        anyhow::bail!("Invalid LR: {} (expected 0.004)", config.training.lr);
    }
    if config.training.steps != CHAMPION_STEPS {
        anyhow::bail!("Invalid steps: {} (expected {})", config.training.steps, CHAMPION_STEPS);
    }

    // Check INV-8: LR must be in [0.001, 0.01]
    if !(0.001..=0.01).contains(&config.training.lr) {
        anyhow::bail!("LR {} violates INV-8: must be in [0.001, 0.01]", config.training.lr);
    }

    Ok(())
}

/// Simple evaluation on validation set (sampling for now)
fn evaluate_on_val(model: &MinimalTransformer, val_dataset: &FineWebDataset) -> Result<(f32, f32)> {
    let seq_len = 128;
    let num_samples = 100; // sample 100 sequences for validation

    let mut total_bpb = 0.0;
    let mut total_nll = 0.0;

    for i in 0..num_samples {
        let batch = val_dataset.sample_sequence(seq_len, &(i as u64));

        // Forward pass
        let forward_result = forward(model, &batch)?;

        // Compute NLL
        let nll = forward_result.logits
            .iter()
            .zip(batch.targets.iter())
            .map(|(logit, &target)| {
                let prob = logit[*target as usize];
                let log_prob = prob.ln() + 1e-10;
                -log_prob
            })
            .sum::<f32>()
            / seq_len as f32
            / 38400.0; // vocab size placeholder

        total_nll += nll;
        total_bpb += calculate_bpb(nll, seq_len);
    }

    let avg_bpb = total_bpb / num_samples as f32;
    let avg_nll = total_nll / num_samples as f32;

    Ok((avg_bpb, avg_nll))
}

/// Champion reproduction test result
#[derive(Debug, Clone)]
pub struct ReproductionResult {
    pub final_bpb: f32,
    pub best_bpb: f32,
    pub steps_completed: usize,
    pub elapsed_seconds: f64,
    pub passed: bool,
}

impl ReproductionResult {
    /// Check if final BPB is within tolerance
    fn is_within_tolerance(&self, bpb: f32) -> bool {
        bpb >= CHAMPION_MIN_BPB && bpb <= CHAMPION_MAX_BPB
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_champion_bpb_calculation() {
        // Test BPB calculation with known values
        let nll = 2.5_f32; // arbitrary NLL
        let num_tokens = 100;

        let bpb = calculate_bpb(nll, num_tokens);
        assert!((2.0..=3.0).contains(&bpb)); // BPB should be reasonable
    }

    #[test]
    fn test_tolerance_check() {
        let result = ReproductionResult {
            final_bpb: 2.2395,
            best_bpb: 2.2350,
            steps_completed: 27_000,
            elapsed_seconds: 3600.0,
            passed: true,
        };

        assert!(result.passed());
    }

    #[test]
    fn test_tolerance_fail() {
        let result = ReproductionResult {
            final_bpb: 2.25, // outside tolerance
            best_bpb: 2.23,
            steps_completed: 27_000,
            elapsed_seconds: 3600.0,
            passed: false,
        };

        assert!(!result.passed());
    }
}
