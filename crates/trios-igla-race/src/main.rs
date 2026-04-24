//! IGLA Race CLI — Distributed hyperparameter hunt
//!
//! Commands:
//! - `trios-igla-race start --workers 4` — run ASHA workers
//! - `trios-igla-race status` — show leaderboard from Neon
//! - `trios-igla-race best` — show best trial

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;
use tracing::{info, error, debug, warn};
use rand::seq::SliceRandom;
use rand::StdRng;

use trios_igla_race::{
    asha::AshaRung,
    neon::NeonDb,
    asha::register_trial,
    asha::record_checkpoint,
    asha::should_prune,
    asha::mark_completed,
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

/// Trial configuration for sampling
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
}

const TARGET_BPB: f64 = 1.50;
const ASHA_KEEP_FRACTION: f64 = 0.33;

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
            let db = NeonDb::connect(&neon_url).await?;

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
            trios_igla_race::status::show_status(&db).await?;
        }

        RaceCommand::Best => {
            show_best(&neon_url).await?;
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
    let mut rng = StdRng::from_rng(rand::rngs::OsRng);

    loop {
        // Sample a new config
        let config = TrialConfig {
            seed: rng.gen() % 1000 + 40,
            d_model: [128, 192, 256, 384]
                .choose(&mut rng)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No d_model options"))?,
            context: [4, 5, 6, 7, 8]
                .choose(&mut rng)
                .copied()
                .ok_or_else(|| anyhow::anyhow!("No context options"))?,
            lr: (rng.gen::<f64>() % 0.0099) + 0.0001,
            optimizer: if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string(),
            wd: Some(rng.gen::<f64>() % 0.099 + 0.001),
            use_attention: Some(rng.gen_bool(0.5)),
        };

        let config_json = serde_json::to_string(&config)?;

        // Register trial (using asha::register_trial which creates Uuid)
        let trial_id = trios_igla_race::asha::register_trial(
            &db, machine_id, worker_id as usize, &config_json,
        ).await?;

        debug!(
            "[worker-{}-{}] Starting trial: lr={:.4}, dim={}, ctx={}",
            machine_id, worker_id, config.lr, config.d_model, config.context
        );

        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        // Run through all ASHA rungs
        for rung in AshaRung::all() {
            let step = rung.step();

            // Run trainer as subprocess
            let bpb = run_trainer_step(&config, step, worker_id)?;

            // Record checkpoint in Neon
            trios_igla_race::asha::record_checkpoint(
                &db, &trial_id, rung, step, bpb,
            ).await?;

            debug!(
                "[worker-{}-{}] Rung {}: BPB={:.4}",
                machine_id, worker_id, step, bpb
            );

            // Check if should prune (ASHA logic)
            if should_prune_at_rung(&db, rung, bpb).await? {
                trios_igla_race::asha::handle_pruning(
                    &db, &trial_id, rung, bpb,
                    &trios_igla_race::lessons::TrialConfig {
                        lr: Some(config.lr),
                        d_model: Some(config.d_model),
                        hidden: None,
                        n_layers: None,
                        optimizer: Some(config.optimizer.clone()),
                        activation: None,
                        weight_decay: config.wd,
                        dropout: None,
                        warmup_steps: None,
                        max_steps: None,
                    },
                ).await?;
                pruned = true;
                warn!(
                    "[worker-{}-{}] Trial pruned at rung {}: BPB={:.4}",
                    machine_id, worker_id, step, bpb
                );
                break;
            }

            // Check if target reached
            if bpb < TARGET_BPB {
                trios_igla_race::asha::mark_completed(&db, &trial_id, step, bpb).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        // Trial completed all rungs without pruning
        if !pruned {
            trios_igla_race::asha::mark_completed(&db, &trial_id, 27000, prev_bpb).await?;

            // Check if new global best
            {
                let mut best = best_bpb.lock().unwrap();
                if prev_bpb < *best {
                    *best = prev_bpb;
                }
            }

            return Ok(prev_bpb);
        }
    }
}

/// Run trios-igla-trainer for one step (rung)
fn run_trainer_step(
    config: &TrialConfig,
    steps: usize,
    worker_id: u64,
) -> Result<f64> {
    let exp_id = format!("trial-w{}-step-{}", worker_id, steps);

    let output = Command::new("./target/release/trios-igla-trainer")
        .arg("--seed")
        .arg(config.seed.to_string())
        .arg("--steps")
        .arg(steps.to_string())
        .arg("--hidden")
        .arg(config.d_model.to_string())
        .arg("--context")
        .arg(config.context.to_string())
        .arg("--lr")
        .arg(config.lr.to_string())
        .arg("--exp-id")
        .arg(&exp_id)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()?;

    parse_bpb_from_output(&output.stdout)
}

/// Parse BPB from trainer output
fn parse_bpb_from_output(stdout: &[u8]) -> Result<f64> {
    let output = std::str::from_utf8(stdout)?;

    for line in output.lines() {
        if let Some(bpb_str) = line.strip_prefix("BPB=") {
            return Ok(bpb_str.trim().parse()?);
        }
        if let Some(bpb_str) = line.strip_prefix("final BPB: ") {
            return Ok(bpb_str.trim().parse()?);
        }
        if line.contains("BPB") {
            if let Some(pos) = line.find("BPB") {
                let rest = &line[pos..];
                let parts: Vec<&str> = rest
                    .split_whitespace()
                    .filter_map(|s| if s.starts_with("=") { Some(&s[1..]) } else { Some(s) })
                    .collect();

                for part in parts {
                    if let Ok(bpb) = part.parse::<f64>() {
                        return Ok(bpb);
                    }
                }
            }
        }
    }

    anyhow::bail!("Could not parse BPB from trainer output")
}

/// Check if trial should be pruned at this rung
async fn should_prune_at_rung(
    db: &NeonDb,
    _rung: AshaRung,
    current_bpb: f64,
) -> Result<bool> {
    // If target reached, never prune
    if current_bpb <= TARGET_BPB {
        return Ok(false);
    }

    // Get all BPB values at this rung
    let column = match _rung {
        AshaRung::Rung1000 => "rung_1000_bpb",
        AshaRung::Rung3000 => "rung_3000_bpb",
        AshaRung::Rung9000 => "rung_9000_bpb",
        AshaRung::Rung27000 => "rung_27000_bpb",
    };

    let query = format!(
        "SELECT {} FROM igla_race_trials WHERE {} IS NOT NULL AND status IN ('running', 'completed')",
        column, column
    );

    let rows = db.client().query(&query, &[]).await?;
    let all_bpb: Vec<f64> = rows.iter()
        .filter_map(|row| row.get::<usize, Option<f64>>(0))
        .collect();

    // ASHA: keep top 33% at each rung, but need minimum data
    if all_bpb.len() < 10 {
        return Ok(false);
    }

    let mut sorted = all_bpb.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let keep_count = ((sorted.len() as f64) * ASHA_KEEP_FRACTION).ceil() as usize;

    if keep_count < sorted.len() {
        let threshold = sorted[keep_count];
        Ok(current_bpb > threshold)
    } else {
        Ok(false)
    }
}

/// Show best trial
async fn show_best(neon_url: &str) -> Result<()> {
    let db = NeonDb::connect(neon_url).await?;

    let row = db.client().query_one(
        "SELECT trial_id, machine_id, config, final_bpb, final_step
         FROM igla_race_trials
         WHERE final_bpb IS NOT NULL
         ORDER BY final_bpb ASC
         LIMIT 1",
        &[],
    ).await?;

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

    Ok(())
}
