//! IGLA Race CLI — Distributed hyperparameter hunt
//!
//! Commands:
//! - `trios-igla-race start --workers 4` — run ASHA workers
//! - `trios-igla-race status` — show leaderboard from Neon
//! - `trios-igla-race best` — show best trial

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;
use tracing::{info, error};
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use uuid::Uuid;

use trios_igla_race::{
    asha::AshaRung,
    neon::NeonDb,
};

/// IGLA Race CLI
#[derive(Parser)]
#[command(name = "trios-igla-race", about = "IGLA RACE — Distributed Hunt for BPB < 1.5")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(Subcommand)]
enum RaceCommand {
    /// Start ASHA workers on current machine
    Start {
        /// Number of parallel workers (default: 4)
        #[arg(long, default_value = "4")]
        workers: usize,
    },
    /// Show leaderboard from Neon
    Status,
    /// Show best trial with config
    Best,
}

const TARGET_BPB: f64 = 1.50;

/// Trial configuration for sampling (local to main.rs)
#[derive(Debug, Clone, serde::Serialize)]
struct TrialConfig {
    seed: i64,
    d_model: usize,
    context: usize,
    lr: f64,
    optimizer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    wd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_attention: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hidden: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_layers: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    activation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight_decay: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dropout: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warmup_steps: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_steps: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();

    let neon_url = std::env::var("NEON_URL")
        .expect("NEON_URL environment variable must be set");

    let machine_id = std::env::var("MACHINE_ID")
        .unwrap_or_else(|_| "unknown".to_string());

    match cli.command {
        RaceCommand::Start { workers } => {
            info!("IGLA RACE START | machine={} | workers={}", machine_id, workers);
            info!("Target BPB: {}", TARGET_BPB);

            // Connect to Neon
            let _db = NeonDb::connect(&neon_url).await?;

            // Shared best BPB across all workers
            let best_bpb = Arc::new(Mutex::new(f64::MAX));

            // Spawn workers
            let mut set = JoinSet::new();
            for worker_id in 0..workers {
                let url = neon_url.clone();
                let mid = machine_id.clone();
                let best = Arc::clone(&best_bpb);
                set.spawn(async move {
                    run_worker(&url, &mid, worker_id as u64, best).await
                });
            }

            // Wait for IGLA or worker completion
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(bpb)) if bpb < TARGET_BPB => {
                        info!("IGLA FOUND! BPB={:.4}", bpb);
                        return Ok(());
                    }
                    Ok(Ok(bpb)) => {
                        info!("worker done, best={:.4}", bpb);
                        // Check if new global best
                        {
                            let mut best = best_bpb.lock().unwrap();
                            if bpb < *best {
                                *best = bpb;
                                info!("NEW BEST BPB={:.4}", bpb);
                            }
                        }
                    }
                    Ok(Err(e)) => error!("worker error: {}", e),
                    Err(e) => error!("join error: {}", e),
                }
            }

            info!("All workers completed");
        }

        RaceCommand::Status => {
            let db = NeonDb::connect(&neon_url).await?;
            trios_igla_race::status::print_leaderboard(db.client()).await?;
        }

        RaceCommand::Best => {
            let db = NeonDb::connect(&neon_url).await?;
            print_best(db.client()).await?;
        }
    }

    Ok(())
}

/// Single worker: runs trials until IGLA found
async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<Mutex<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = StdRng::from_entropy();

    loop {
        // Sample a new config
        let config = TrialConfig {
            seed: rng.gen_range(40..1040) + 40,
            d_model: [128, 192, 256, 384].choose(&mut rng).copied().unwrap_or(256),
            context: [4, 5, 6, 7, 8].choose(&mut rng).copied().unwrap_or(6),
            lr: (rng.gen_range(0.0001..0.01) % 0.0099) + 0.0001,
            optimizer: if rng.gen_bool(0.5) { "adamw".to_string() } else { "muon".to_string() },
            wd: Some(rng.gen_range(0.0001..0.01) % 0.099 + 0.001),
            use_attention: Some(rng.gen_bool(0.5)),
            hidden: Some(384),
            n_layers: Some(1),
            activation: Some("relu".to_string()),
            weight_decay: Some(0.01),
            dropout: Some(0.0),
            warmup_steps: Some(0),
            max_steps: Some(27000),
        };

        let config_json = serde_json::to_string(&config)?;

        // Register trial
        let trial_id = Uuid::new_v4();
        db.register_trial(trial_id, machine_id, worker_id as i32, &config_json).await?;

        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        // Run through all ASHA rungs
        for rung in AshaRung::all() {
            let step = rung.step() as i32;

            // Simulate training (would run trainer binary in production)
            let bpb = simulate_training(&config, step as u64).await?;

            // Update rung
            db.record_checkpoint(&trial_id, step, bpb).await?;

            // Check if should prune
            if bpb > 2.7 && step == 1000 {
                db.mark_pruned(&trial_id, step, bpb).await?;
                pruned = true;
                break;
            }

            if bpb < 1.5 {
                db.mark_completed(&trial_id, step, bpb).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        if !pruned {
            db.mark_completed(&trial_id, 27000, prev_bpb).await?;
        }

        // Update global best
        {
            let mut b = best_bpb.lock().unwrap();
            if prev_bpb < *b {
                *b = prev_bpb;
            }
        }
    }
}

/// Simulate training (placeholder - would run trainer binary)
async fn simulate_training(config: &TrialConfig, steps: u64) -> Result<f64> {
    // Simple simulation based on config parameters
    let base_bpb = 3.0;
    let lr_effect = config.lr * 100.0;
    let dim_effect = config.d_model as f64 / 100.0;
    let ctx_effect = config.context as f64 * 0.05;

    let simulated_bpb = base_bpb - lr_effect - dim_effect - ctx_effect + (steps as f64 * 0.0001);
    Ok(simulated_bpb.max(1.0))
}

/// Print best trial
async fn print_best(client: &tokio_postgres::Client) -> Result<()> {
    let row = client.query_one(
        "SELECT trial_id::text, machine_id, config::text, final_bpb::text, final_step::text
         FROM igla_race_trials
         WHERE final_bpb IS NOT NULL
         ORDER BY final_bpb ASC
         LIMIT 1",
        &[],
    ).await;

    if let Ok(row) = row {
        let trial_id: String = row.get(0);
        let machine_id: String = row.get(1);
        let config: String = row.get(2);
        let final_bpb: f64 = row.get(3);
        let final_step: i32 = row.get(4);

        println!("BEST TRIAL");
        println!("  Trial ID: {}", trial_id);
        println!("  Machine:   {}", machine_id);
        println!("  BPB:       {:.4}", final_bpb);
        println!("  Steps:     {}", final_step);
        println!("  Config:    {}", config);
    } else {
        println!("No trials completed yet");
    }

    Ok(())
}
