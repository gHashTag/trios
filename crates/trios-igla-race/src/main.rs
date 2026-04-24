use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;
use tracing::info;
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
        #[arg(long, default_value = "unknown")]
        machine: String,
        #[arg(long, default_value = "4")]
        workers: usize,
    },
    Status,
    Best,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let cli = Cli::parse();
    let neon_url = std::env::var("NEON_URL").expect("NEON_URL must be set");
    let machine_id = std::env::var("MACHINE_ID").unwrap_or_else(|_| "unknown".to_string());

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
                    run_worker(&url, &mid, worker_id, best).await
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
                        if bpb < *best {
                            *best = bpb;
                            info!("NEW BEST BPB={:.4}", bpb);
                        }
                    }
                    Ok(Err(e)) => eprintln!("worker error: {}", e),
                    Err(e) => eprintln!("join error: {}", e),
                }
            }
            info!("All workers completed");
        }
        RaceCommand::Status => {
            let _db = NeonDb::connect(&neon_url).await?;
            println!("IGLA RACE Status (STUB MODE)");
        }
        RaceCommand::Best => {
            println!("BEST TRIAL (STUB MODE)");
        }
    }
    Ok(())
}

async fn run_worker(
    _neon_url: &str,
    machine_id: &str,
    worker_id: usize,
    best_bpb: Arc<RwLock<f64>>,
) -> anyhow::Result<f64> {
    let db = NeonDb::connect(_neon_url).await?;
    let mut rng = StdRng::from_entropy();

    loop {
        let d_model = *[128, 192, 256, 384].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context = *[4, 5, 6, 7, 8].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No context"))?;
        let lr = rng.gen_range(0.0001..0.01);

        let config = format!(r#"{{"d_model": {}, "context": {}, "lr": {}}}"#, d_model, context, lr);
        let trial_id = uuid::Uuid::new_v4();
        db.register_trial(trial_id, machine_id, worker_id as i32, &config).await?;

        let output = tokio::process::Command::new("./target/release/trios-igla-trainer")
            .arg("--arch")
            .arg("ngram")
            .arg("--steps")
            .arg("5000")
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        info!("Trainer output received, length: {}", stdout.len());
        
        let bpb = parse_bpb(&stdout).unwrap_or_else(|| {
            info!("parse_bpb returned None, using f64::MAX");
            f64::MAX
        });

        for rung_step in [1000, 3000, 9000, 27000] {
            db.record_checkpoint(&trial_id, rung_step, bpb).await?;
        }

        if bpb < 1.50 {
            return Ok(bpb);
        }

        let mut best = best_bpb.write().unwrap();
        if bpb < *best {
            *best = bpb;
        }
    }
}

fn parse_bpb(stdout: &str) -> Option<f64> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(stdout) {
        if let Some(results) = parsed.get("results").and_then(|r| r.as_array()) {
            if let Some(last) = results.last() {
                let bpb = last.get("bpb").and_then(|b| b.as_f64());
                info!("parse_bpb extracted BPB: {:?}", bpb);
                return bpb;
            }
        }
    }
    info!("parse_bpb: JSON parse failed or structure incorrect");
    None
}
