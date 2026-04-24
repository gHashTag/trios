//! IGLA Race CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::{Arc, Mutex};
use tokio::task::JoinSet;
use tokio_postgres::{NoTls, types::ToSql};
use tracing::{info, error};
use rand::{Rng, SeedableRng, seq::SliceRandom, rngs::StdRng};
use uuid::Uuid;

use trios_igla_race::{
    neon::NeonDb,
    status::{print_leaderboard, print_best},
    lessons::TrialConfig,
    asha::AshaRung,
};

#[derive(Parser)]
#[command(name = "trios-igla-race", about = "IGLA RACE — Distributed Hunt for BPB < 1.5")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(Subcommand)]
enum RaceCommand {
    Start { #[arg(long, default_value = "4")] workers: usize },
    Status,
    Best,
}

const TARGET_BPB: f64 = 1.50;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    let neon_url = std::env::var("NEON_URL").expect("NEON_URL must be set");
    let machine_id = std::env::var("MACHINE_ID").unwrap_or_else(|_| "unknown".to_string());

    match cli.command {
        RaceCommand::Start { workers } => {
            info!("IGLA RACE START | machine={} | workers={}", machine_id, workers);
            let best_bpb = Arc::new(Mutex::new(f64::MAX));
            let mut set = JoinSet::new();

            for worker_id in 0..workers {
                let url = neon_url.clone();
                let mid = machine_id.clone();
                let best = Arc::clone(&best_bpb);
                let mut rng = StdRng::from_entropy().unwrap();
                
                set.spawn(async move {
                    run_worker(&url, &mid, worker_id as u64, best, &mut rng).await
                });
            }

            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(bpb)) if bpb < TARGET_BPB => {
                        info!("IGLA FOUND! BPB={:.4}", bpb);
                        return Ok(());
                    }
                    Ok(Ok(bpb)) => {
                        info!("worker done, best={:.4}", bpb);
                        let mut best = best_bpb.lock().unwrap();
                        if bpb < *best { *best = bpb; }
                    }
                    Ok(Err(e)) => error!("worker error: {}", e),
                    Err(e) => error!("join error: {}", e),
                }
            }
            info!("All workers completed");
        }
        RaceCommand::Status => {
            let db = NeonDb::connect(&neon_url).await?;
            print_leaderboard(db.client()).await?;
        }
        RaceCommand::Best => {
            show_best(&neon_url).await?;
        }
    }
    Ok(())
}

async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<Mutex<f64>>,
    rng: &mut StdRng
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;

    loop {
        let d_model_val = *[128, 192, 256, 384].choose(rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context_val = *[4, 5, 6, 7, 8].choose(rng).ok_or_else(|| anyhow::anyhow!("No context"))?;

        let config = TrialConfig {
            d_model: Some(d_model_val),
            context: Some(context_val),
            lr: Some(rng.gen_range(0.0001..0.01)),
            optimizer: Some(if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string()),
            weight_decay: Some(rng.gen_range(0.001..0.1)),
            use_attention: Some(rng.gen_bool(0.5)),
            hidden: Some(384),
            n_layers: Some(1),
            activation: Some("relu".to_string()),
            dropout: Some(0.0),
            warmup_steps: Some(0),
            max_steps: Some(27000),
        };

        let config_str = serde_json::to_string(&config)?;
        let trial_id = Uuid::new_v4();
        
        db.register_trial(trial_id, machine_id, worker_id as i32, &config_str).await?;

        let mut prev_bpb = f64::MAX;
        let mut pruned = false;

        for rung in AshaRung::all() {
            let step = rung.step();
            let bpb = simulate_training(&config, step as u64).await?;

            let query = format!("UPDATE igla_race_trials SET rung_{}_step = $1, rung_{}_bpb = $2, final_step = $1, final_bpb = $2 WHERE trial_id = $3", step, step);
            
            let args: [&(dyn ToSql + Sync); 3] = [&step as i32, &bpb as i32, &trial_id];
            db.client().execute(&query, &args).await?;

            if bpb > 2.7 && step == 1000 {
                let args: [&(dyn ToSql + Sync); 1] = [&trial_id];
                db.client().execute("UPDATE igla_race_trials SET status = 'pruned' WHERE trial_id = $1", &args).await?;
                pruned = true;
                break;
            }

            if bpb < 1.5 {
                let args: [&(dyn ToSql + Sync); 2] = [&bpb, &trial_id];
                db.client().execute("UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2", &args).await?;
                return Ok(bpb);
            }

            prev_bpb = bpb;
        }

        if !pruned {
            let args: [&(dyn ToSql + Sync); 2] = [&prev_bpb, &trial_id];
            db.client().execute("UPDATE igla_race_trials SET status = 'completed', final_bpb = $1 WHERE trial_id = $2", &args).await?;
        }

        let mut best = best_bpb.lock().unwrap();
        if prev_bpb < *best { *best = prev_bpb; }
    }
}

async fn simulate_training(config: &TrialConfig, steps: u64) -> Result<f64> {
    let base_bpb = 3.0;
    let lr_effect = config.lr.unwrap_or(0.004) * 100.0;
    let dim_effect = config.d_model.unwrap_or(256) as f64 / 100.0;
    let ctx_effect = config.context.unwrap_or(6) as f64 * 0.05;
    let simulated_bpb = base_bpb - lr_effect - dim_effect - ctx_effect + (steps as f64 * 0.0001);
    Ok(simulated_bpb.max(1.0))
}

async fn show_best(neon_url: &str) -> Result<()> {
    let (client, connection) = tokio_postgres::connect(neon_url, NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let row = client.query_one(
        "SELECT trial_id::text, machine_id, config::text, final_bpb::text, final_step::text FROM igla_race_trials WHERE final_bpb IS NOT NULL ORDER BY final_bpb ASC LIMIT 1",
        &[],
    ).await;

    if let Ok(row) = row {
        let trial_id: &str = row.get(0)?;
        let machine_id: &str = row.get(1)?;
        let config_json: &str = row.get(2)?;
        let final_bpb: &str = row.get(3)?;
        let final_step: &str = row.get(4)?;

        println!("BEST TRIAL");
        println!("  Trial ID: {}", trial_id);
        println!("  Machine:   {}", machine_id);
        println!("  BPB:       {}", final_bpb);
        println!("  Steps:     {}", final_step);
        println!("  Config:    {}", config_json);
    } else {
        println!("No trials completed yet");
    }
    Ok(())
}
