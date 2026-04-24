use clap::Parser;
use std::sync::{Arc, RwLock};
use tokio::task::JoinSet;
use tracing::info;
use rand::{Rng, SeedableRng, seq::SliceRandom, rngs::StdRng};

#[derive(Parser)]
#[command(name = "trios-igla-race")]
struct Cli {
    #[command(subcommand)]
    command: RaceCommand,
}

#[derive(clap::Subcommand)]
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
    let cli = Cli::try_parse()?;

    match cli.command {
        RaceCommand::Start { machine, workers } => {
            info!("IGLA RACE START | machine={} workers={}", machine, workers);
            let best_bpb = Arc::new(RwLock::new(f64::MAX));
            let mut set = JoinSet::new();
            for worker_id in 0..workers {
                let url = String::new();
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
                        if bpb < *best { *best = bpb; }
                    }
                    Ok(Err(e)) => eprintln!("worker error: {}", e),
                    Err(e) => eprintln!("join error: {}", e),
                }
            }
            info!("All workers completed");
        }
        RaceCommand::Status => {
            println!("IGLA RACE LEADERBOARD (TODO)");
        }
        RaceCommand::Best => {
            println!("BEST TRIAL (TODO)");
        }
    }
    Ok(())
}

async fn run_worker(
    _neon_url: &str,
    _machine_id: &str,
    _worker_id: usize,
    best_bpb: Arc<RwLock<f64>>,
) -> anyhow::Result<f64> {
    let mut rng = StdRng::from_entropy();

    loop {
        let d_model = *[128, 192, 256, 384].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No d_model"))?;
        let context = *[4, 5, 6, 7, 8].choose(&mut rng).ok_or_else(|| anyhow::anyhow!("No context"))?;

        let lr = rng.gen_range(0.0001..0.01);
        let _optimizer = if rng.gen_bool(0.5) { "adamw" } else { "muon" }.to_string();
        let _wd = rng.gen_range(0.001..0.1);

        let output = tokio::process::Command::new("./target/release/trios-igla-trainer")
            .arg("--config")
            .arg(format!("d_model={},context={},lr={}", d_model, context, lr))
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
