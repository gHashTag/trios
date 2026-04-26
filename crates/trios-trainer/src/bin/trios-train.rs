//! trios-train — CLI entry point for IGLA training
//!
//! Usage:
//! ```bash
//! cargo run --release -p trios-trainer -- \
//!     --config crates/trios-trainer/configs/champion.toml --seed 43
//! ```

use clap::Parser;
use anyhow::Result;
use trios_trainer::{Config, run};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "trios-train",
    about = "IGLA training — single source of truth",
    long_about = "Run IGLA training on any machine. All configs in TOML, all emits validated."
)]
struct Args {
    /// Path to config file (TOML format)
    #[arg(short, long)]
    config: PathBuf,

    /// Seed override (overrides config file)
    #[arg(long)]
    seed: Option<u64>,

    /// Steps override (overrides config file)
    #[arg(long)]
    steps: Option<usize>,

    /// Dry run — validate config but don't train
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("=== trios-train v0.1.0 ===");
    println!("Loading config from: {}", args.config.display());

    // Load config
    let mut config = Config::load(&args.config)?;

    // Apply CLI overrides
    if let Some(seed) = args.seed {
        config.training.seed = seed;
        println!("Seed overridden to {}", seed);
    }

    if let Some(steps) = args.steps {
        config.training.steps = steps;
        println!("Steps overridden to {}", steps);
    }

    // Validate INV-8: lr in phi-band
    if !trios_trainer::config::validate_lr_phi_band(config.training.lr) {
        anyhow::bail!("LR {} violates INV-8: must be in [0.001, 0.01]", config.training.lr);
    }

    println!("Config validated (INV-8 OK)");

    if args.dry_run {
        println!("\n=== DRY RUN — Config is valid ===");
        println!("Seed: {}", config.training.seed);
        println!("Steps: {}", config.training.steps);
        println!("LR: {}", config.training.lr);
        println!("d_model: {}", config.model.d_model);
        println!("n_layers: {}", config.model.n_layers);
        println!("Checkpoint interval: {}", config.training.checkpoint_interval);
        return Ok(());
    }

    // Run training
    println!("\n=== Starting training ===");
    let result = run(&config)?;

    // Print result
    println!("\n=== Training complete ===");
    println!("Final BPB: {:.4}", result.final_bpb);
    println!("Best BPB: {:.4}", result.best_bpb);
    println!("Steps: {}", result.steps_completed);

    // Gate-2 verdict
    if result.best_bpb < 1.50 {
        println!("✅ GATE-2 VICTORY CANDIDATE");
    } else if result.best_bpb < 1.85 {
        println!("🟡 Above target evidence");
    } else {
        println!("🔴 Below target evidence");
    }

    Ok(())
}
