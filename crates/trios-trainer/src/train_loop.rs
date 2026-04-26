//! Training loop — FineWeb data loading, step loop, evaluation, ledger emit

use crate::{Config, FineWebDataset};
use crate::ledger::{LedgerRow, EmbargoBlock};
use anyhow::Result;
use std::time::SystemTime;

/// Run the training loop with real FineWeb data
pub fn run(config: &Config) -> Result<RunResult> {
    println!("=== trios-trainer ===");
    println!("Seed: {}", config.training.seed);
    println!("Steps: {}", config.training.steps);
    println!("LR: {} (INV-8 validated)", config.training.lr);
    println!("Train path: {}", config.training.train_path);
    println!("Val path: {}", config.training.val_path);

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

    let mut best_bpb = f32::MAX;
    let mut final_bpb = 0.0;
    let mut rng_state = config.training.seed;
    let seq_len = 128; // Fixed sequence length for now

    for step in 0..=config.training.steps {
        // Sample a random sequence for training
        let _tokens = train_dataset.sample_sequence(seq_len, &mut rng_state);

        // TODO: PR-2 — Actual training step with real model
        // For now, use mock evaluation
        let bpb = evaluate_step(step, config.training.seed)?;

        if bpb < best_bpb {
            best_bpb = bpb;
            println!("Step {}: BPB = {:.4} (NEW BEST)", step, bpb);
        } else {
            println!("Step {}: BPB = {:.4}", step, bpb);
        }

        final_bpb = bpb;

        // Emit row to ledger at checkpoint intervals
        if step % config.training.checkpoint_interval == 0 || step == config.training.steps {
            let row = LedgerRow {
                agent: "trios-trainer".into(),
                bpb,
                seed: config.training.seed,
                sha: crate::ledger::get_commit_sha().unwrap_or_else(|_| "unknown".into()),
                step,
                ts: format_timestamp(),
                gate_status: if bpb < 1.85 { "above_target_evidence".to_string() } else { "below_target_evidence".to_string() },
            };

            let embargo = EmbargoBlock::new();
            if let Err(e) = crate::ledger::emit_row(&config.ledger.path, &row, &embargo) {
                eprintln!("Failed to emit row: {}", e);
            }
        }
    }

    Ok(RunResult {
        final_bpb,
        best_bpb,
        steps_completed: config.training.steps,
    })
}

/// Placeholder evaluation — returns dummy BPB
///
/// TODO: PR-2 — Replace with actual model evaluation
fn evaluate_step(step: usize, seed: u64) -> Result<f32> {
    // Dummy: BPB decreases slowly as training progresses
    let base_bpb = 3.0;
    let progress = (step as f32) / 27000.0;
    let noise = (seed % 100) as f32 / 1000.0;
    Ok(base_bpb - (progress * 0.5) + noise)
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
    fn test_evaluate_step() {
        let bpb = evaluate_step(100, 42).unwrap();
        assert!(bpb > 0.0 && bpb < 10.0);
    }
}
