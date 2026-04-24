use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;
use tracing::{error, info};
use rand::{Rng, SeedableRng, seq::SliceRandom, rngs::StdRng};

use trios_igla_race::neon::NeonDb;

#[derive(Parser)]
#[command(name = "trios-igla-race")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(Subcommand)]
enum RaceCommand {
    Start {
        #[arg(long, env = "MACHINE_ID")]
        machine: String,
        #[arg(long, default_value = "4")]
        workers: usize,
    },
    Status,
    Best,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let neon_url = std::env::var("NEON_URL").expect("NEON_URL must be set");
    let cli = Cli::parse();

    match cli.command {
        RaceCommand::Start { machine, workers } => {
            info!("IGLA RACE START | machine={} workers={}", machine, workers);
            let best_bpb = Arc::new(RwLock::new(f64::MAX));
            let mut set = JoinSet::new();
            for worker_id in 0..workers {
                let url = neon_url.clone();
                let mid = machine.clone();
                let best = Arc::clone(&best_bpb);
                set.spawn(async move {
                    run_worker(&url, &mid, worker_id as u64, best).await
                });
            }
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(bpb)) if bpb < 1.50 => {
                        info!("IGLA FOUND! BPB={:.4}", bpb);
                        return Ok(());
                    }
                    Ok(Ok(bpb)) => {
                        let mut best = best_bpb.write().unwrap();
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
            println!("IGLA RACE LEADERBOARD");
            // TODO: Implement leaderboard display
        }
        RaceCommand::Best => {
            println!("BEST TRIAL");
            // TODO: Implement best trial display
        }
    }
    Ok(())
}

async fn run_worker(
    neon_url: &str,
    machine_id: &str,
    worker_id: u64,
    best_bpb: Arc<RwLock<f64>>,
) -> Result<f64> {
    let db = NeonDb::connect(neon_url).await?;
    let mut rng = StdRng::from_entropy();

    loop {
        let d_model = *[128, 192, 256, 384].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context = *[4, 5, 6, 7, 8].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No context"))?;

        let lr = rng.gen_range(0.0001..0.01);
        let optimizer = if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string();
        let wd = rng.gen_range(0.001..0.1);

        let config = format!(
            r#"{{"d_model": {}, "context": {}, "lr": {}, "optimizer": "{}", "wd": {}}}"#,
            d_model, context, lr, optimizer, wd
        );

        let trial_id = uuid::Uuid::new_v4();
        db.register_trial(trial_id, machine_id, worker_id as i32, &config).await?;

        let output = tokio::process::Command::new("./target/release/trios-igla-trainer")
            .arg("--config")
            .arg(&format!("d_model={},context={},lr={}", d_model, context, lr))
            .arg("--steps")
            .arg("12000")
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let bpb = parse_bpb(&stdout).unwrap_or(f64::MAX);

        if bpb < 1.50 {
            return Ok(bpb);
        }

        let mut best = best_bpb.write().unwrap();
        if bpb < *best { *best = bpb; }
    }
}

fn parse_bpb(stdout: &str) -> Option<f64> {
    stdout.lines()
        .rev()
        .find(|l| l.starts_with("BPB="))
        .and_then(|l| l.trim_start_matches("BPB=").parse().ok())
}
