//! IGLA Race CLI — Distributed hyperparameter hunt
//!
//! Subcommands:
//! - `trios-igla-race start --workers 4` — run ASHA workers
//! - `trios-igla-race status` — show leaderboard from Neon
//! - `trios-igla-race best` — show best trial


use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "trios-igla-race", about = "IGLA RACE — Distributed Hunt for BPB < 1.5")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(Subcommand)]
enum RaceCommand {
    /// Launch ASHA workers
    Start {
        #[arg(long, env = "MACHINE_ID", default_value = "unknown")]
        machine: String,
        #[arg(long, default_value = "4")]
        workers: usize,
    },
    /// Show race leaderboard
    Status,
    /// Show best BPB result
    Best,
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
            info!("Target BPB: 1.50");
            info!("Neon API: {}", neon_url);

            // TODO: Call trios_igla_race::run_worker via API
            info!("Workers stub: spawning keep-alive tasks");
            
            use std::time::Duration;
            for worker_id in 0..workers {
                info!("Worker {} started (stub)", worker_id);
                tokio::time::sleep(Duration::from_secs(60)).await;
                info!("Worker {} completed (stub)", worker_id);
            }
            
            info!("All workers completed (stub)");
        }
        RaceCommand::Status => {
            trios_igla_race::status::show_status().await?;
        }
        RaceCommand::Best => {
            trios_igla_race::status::show_best().await?;
        }
    }

    Ok(())
}
