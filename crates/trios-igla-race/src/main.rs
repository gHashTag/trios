use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;
use tracing::info;

use trios_igla_race::asha::run_worker;
use trios_igla_race::neon::NeonDb;
use trios_igla_race::status;

#[derive(Parser)]
#[command(name = "trios-igla-race", about = "IGLA RACE — distributed hunt for BPB < 1.5")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(Subcommand)]
enum RaceCommand {
    Start {
        #[arg(long, env = "MACHINE_ID", default_value = "local")]
        machine: String,
        #[arg(long, default_value = "4")]
        workers: usize,
    },
    Status,
    Best,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let neon_url = std::env::var("NEON_URL")
        .expect("NEON_URL env var must be set");

    match cli.command {
        RaceCommand::Start { machine, workers } => {
            info!("IGLA RACE START | machine={machine} | workers={workers}");

            let best_bpb = Arc::new(RwLock::new(f64::MAX));
            let mut set = JoinSet::new();

            for wid in 0..workers {
                let url = neon_url.clone();
                let mid = machine.clone();
                let best = Arc::clone(&best_bpb);
                set.spawn(async move {
                    run_worker(&url, &mid, wid as u64, best).await
                });
            }

            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(bpb)) if bpb < 1.5 => {
                        info!("IGLA FOUND! BPB={bpb:.4}");
                        return Ok(());
                    }
                    Ok(Ok(bpb)) => {
                        info!("worker finished, best={bpb:.4}");
                        let mut best = best_bpb.write().unwrap();
                        if bpb < *best { *best = bpb; }
                    }
                    Ok(Err(e)) => eprintln!("worker error: {e}"),
                    Err(e) => eprintln!("join error: {e}"),
                }
            }
        }
        RaceCommand::Status => {
            let db = NeonDb::connect(&neon_url).await?;
            status::show_status(&db).await?;
        }
        RaceCommand::Best => {
            let db = NeonDb::connect(&neon_url).await?;
            status::show_best(&db).await?;
        }
    }

    Ok(())
}
