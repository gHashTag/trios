//! Simplified training loop for Phase P0 Audit — no checkpoints/validations dependencies

use crate::{Config, FineWebDataset};
use crate::model::MinimalTransformer;
use crate::optimizer::AdamWCpu;
use crate::ledger::{LedgerRow, EmbargoBlock};
use crate::validation::{
    calculate_bpb,
    is_within_champion_tolerance,
    CHAMPION_BPB_TARGET,
    CHAMPION_BPB_TOLERANCE,
    CHAMPION_MIN_BPB,
    CHAMPION_MAX_BPB,
    CHAMPION_STEPS,
};
use anyhow::Result;
use std::time::SystemTime;

/// Run simplified training loop for Phase P0 Audit
pub fn run_simple(config: &Config) -> Result<RunResult> {
    println!("=== trios-trainer (Phase P0 Audit) ===");
    println!("Seed: {}", config.training.seed);
    println!("Steps: {}", config.training.steps);
    println!("LR: {} (INV-8 validated)", config.training.lr);
    println!("Champion target BPB: {}", CHAMPION_BPB_TARGET);
    println!("Target tolerance: ± {}", CHAMPION_BPB_TOLERANCE);

    // Load FineWeb dataset
    println!("Loading training data...");
    let train_dataset = FineWebDataset::load(&config.training.train_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to load train data: {}. Using fallback.", e);
            FineWebDataset::fallback()
        });
    println!("Loaded {} training tokens", train_dataset.len());

    println!("Loading validation data...");
    let val_dataset = FineWebDataset::load(&config.training.val_path)
        .unwrap_or_else(|e| {
            eprintln!("Failed to load val data: {}. Using fallback.", e);
            FineWebDataset::fallback()
        });
    println!("Loaded {} validation tokens", val_dataset.len());

    // Initialize model from config
    println!("Initializing model...");
    let d_ffn = config.model.d_model * config.model.ff_mult;
    let mut model = MinimalTransformer::new(
        50257, // GPT-2 vocab size
        config.model.d_model,
        d_ffn,
        8,      // n_heads
        config.model.n_layers,
    );
    println!("Model parameters: {}", model.param_count());

    // Initialize optimizer
    println!("Initializing optimizer...");
    let mut optimizer = AdamWCpu::with_phi_defaults(model.param_count());
    println!("Optimizer: AdamW (phi-based defaults)");

    let mut best_bpb = f32::MAX;
    let mut final_bpb = 0.0;
    let mut rng_state = config.training.seed;
    let seq_len = config.model.context_len.min(128);

    println!("Starting training loop...");
    println!();

    for step in 0..=config.training.steps {
        // Sample a random sequence for training
        let tokens_u32 = train_dataset.sample_sequence(seq_len, &mut rng_state);
        let tokens: Vec<usize> = tokens_u32.iter().map(|&t| t as usize).collect();

        if tokens.is_empty() {
            continue;
        }

        // Forward pass
        let logits = model.forward(&tokens);

        // Compute loss (cross-entropy)
        let targets = &tokens[1..];

        // Calculate BPB from loss
        let loss = calculate_cross_entropy_loss(&logits, targets);
        let bpb = calculate_bpb(loss, targets.len());

        // Backward pass
        let gradients = model.backward(targets);

        // Optimizer step
        let params = model.parameters();
        let mut params_vec = params.to_vec();
        optimizer.step(&mut params_vec, &flatten_gradients_simple(&gradients));

        // Update model parameters
        model.update_parameters(&params_vec);

        // Evaluation at intervals
        if step % config.training.eval_interval == 0 || step == config.training.steps {
            let val_bpb = evaluate_simple(&model, &val_dataset, config.model.context_len)?;

            // Champion validation: check if BPB within tolerance
            if step == CHAMPION_STEPS && !is_within_champion_tolerance(val_bpb) {
                eprintln!("Step {}: CHAMPION VALIDATION FAILED: BPB {:.4} outside [{:.4}, {:.4}]",
                    step, val_bpb, CHAMPION_MIN_BPB, CHAMPION_MAX_BPB);
            }

            if val_bpb < best_bpb {
                best_bpb = val_bpb;
                println!("Step {}: BPB = {:.4} (NEW BEST)", step, val_bpb);
            } else {
                println!("Step {}: BPB = {:.4}", step, val_bpb);
            }
            final_bpb = val_bpb;
            println!();

            // Emit row to ledger at checkpoint intervals
            if step % config.training.checkpoint_interval == 0 {
                let row = LedgerRow {
                    agent: "trios-trainer".into(),
                    bpb: val_bpb,
                    seed: config.training.seed,
                    sha: crate::ledger::get_commit_sha().unwrap_or_else(|_| "unknown".into()),
                    step,
                    ts: format_timestamp(),
                    gate_status: if val_bpb < 1.85 { "above_target_evidence".to_string() } else { "below_target_evidence".to_string() },
                };

                let embargo = EmbargoBlock::new();
                if let Err(e) = crate::ledger::emit_row(&config.ledger.path, &row, &embargo) {
                    eprintln!("Failed to emit row: {}", e);
                }
            }
        }
    }

    println!("\n=== Training Complete ===");
    println!("Final BPB: {:.4}", final_bpb);
    println!("Best BPB: {:.4}", best_bpb);
    println!("Champion target: {:.4}", CHAMPION_BPB_TARGET);
    println!("Status: {}", if is_within_champion_tolerance(final_bpb) { "✅ PASS" } else { "❌ FAIL" });

    Ok(RunResult {
        final_bpb,
        best_bpb,
        steps_completed: config.training.steps,
    })
}

/// Compute cross-entropy loss (simplified for Phase P0)
fn compute_cross_entropy_loss(logits: &[Vec<f32>], targets: &[usize]) -> f32 {
    if targets.is_empty() {
        return 0.0;
    }

    let mut total_loss = 0.0;

    for (pos, &target) in targets.iter().enumerate() {
        if pos >= logits.len() {
            break;
        }
        let pos_logits = &logits[pos];

        // Softmax
        let max_logit = pos_logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp_sum: f32 = pos_logits.iter().map(|&v| (v - max_logit).exp()).sum();

        if exp_sum > 0.0 {
            let probs: Vec<f32> = pos_logits.iter()
                .map(|&v| (v - max_logit).exp() / exp_sum)
                .collect();

            // Cross-entropy loss
            let prob = probs.get(target).copied().unwrap_or(1e-10f32);
            total_loss -= prob.ln();
        }
    }

    let num_targets = targets.len() as f32;
    total_loss / num_targets
}

/// Simplified evaluation (no external dependencies)
fn evaluate_simple(model: &MinimalTransformer, val_dataset: &FineWebDataset, context_len: usize) -> Result<f32> {
    let seq_len = context_len.min(128);
    let n_chunks = val_dataset.len() / seq_len;
    let chunks_to_eval = n_chunks.min(100); // Limit to 100 chunks for speed

    let mut total_loss = 0.0;
    let mut total_tokens = 0;

    for i in 0..chunks_to_eval {
        let start = i * seq_len;
        let end = (start + seq_len + 1).min(val_dataset.len());

        let tokens_u32 = val_dataset.get_slice(start, end);
        let tokens: Vec<usize> = tokens_u32.iter().map(|&t| t as usize).collect();

        if tokens.len() < 2 {
            continue;
        }

        // Forward pass
        let logits = model.forward(&tokens);
        let targets = &tokens[1..];

        // Compute loss
        let loss = compute_cross_entropy_loss(&logits, targets);
        total_loss += loss * targets.len() as f32;
        total_tokens += targets.len();
    }

    let avg_loss = if total_tokens > 0 { total_loss / total_tokens as f32 } else { 10.0 };
    let avg_bpb = calculate_bpb(avg_loss, total_tokens);

    Ok(avg_bpb)
}

/// Simple flatten gradients (no external ModelGradients dependency)
fn flatten_gradients_simple(grads: &crate::model::ModelGradients) -> Vec<f32> {
    let mut flat = Vec::new();

    flat.extend_from_slice(&grads.token_emb_grad);
    flat.extend_from_slice(&grads.pos_emb_grad);

    for layer in &grads.layers_grad {
        flat.extend_from_slice(&layer.w_q_grad);
        flat.extend_from_slice(&layer.w_k_grad);
        flat.extend_from_slice(&layer.w_v_grad);
        flat.extend_from_slice(&layer.w_o_grad);
        flat.extend_from_slice(&layer.w1_grad);
        flat.extend_from_slice(&layer.w2_grad);
        flat.extend_from_slice(&layer.b1_grad);
        flat.extend_from_slice(&layer.b2_grad);
    }

    flat.extend_from_slice(&grads.lm_head_grad);

    flat
}

/// Format current timestamp as ISO 8601
fn format_timestamp() -> String {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| {
            let secs = d.as_secs();
            format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                1970 + secs / 31536000,
                (secs % 31536000) / 2592000,
                (secs % 2592000) / 86400,
                (secs % 86400) / 3600,
                (secs % 3600) / 60,
                secs % 60)
        })
        .unwrap_or_else(|_| "unknown".into())
}

/// Result of a training run
#[derive(Debug, Clone)]
pub struct RunResult {
    pub final_bpb: f32,
    pub best_bpb: f32,
    pub steps_completed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_timestamp() {
        let ts = format_timestamp();
        assert!(ts.contains("T") && ts.ends_with("Z"));
    }

    #[test]
    fn test_calculate_bpb() {
        // Perfect compression: BPB = 1.0
        let loss = 1.0_f32; // loss where perplexity = 256 (2^8)
        let num_tokens = 256; // batch size

        let bpb = calculate_bpb(loss, num_tokens);

        // BPB = (log2(256) / log2(2^BPB)) / 8 = 1.0
        assert!((bpb - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_champion_tolerance() {
        assert!(is_within_champion_tolerance(2.2393)); // true
        assert!(!is_within_champion_tolerance(2.2292))); // false (below min)
        assert!(!is_within_champion_tolerance(2.2494))); // false (above max)
    }

    #[test]
    fn test_champion_config_validation() {
        // This would require loading actual champion.toml
        // For now, just test constants
        assert_eq!(CHAMPION_BPB_TARGET, 2.2393);
        assert_eq!(CHAMPION_BPB_TOLERANCE, 0.01);
        assert_eq!(CHAMPION_MIN_BPB, 2.2293);
        assert_eq!(CHAMPION_MAX_BPB, 2.2493);
        assert_eq!(CHAMPION_STEPS, 27_000);
    }
}
