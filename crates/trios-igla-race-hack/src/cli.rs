//! CLI — command-line interface for IGLA RACE.
//!
//! Provides user-facing commands for running races, querying status,
//! and managing the IGLA RACE system.

use clap::{Parser, Subcommand};
use anyhow::Result;
use std::env;
use tracing::info;

use super::status::{QueryStatus, RaceStatus};

/// IGLA RACE CLI.
#[derive(Parser)]
#[command(name = "trios-igla-race", about = "IGLA RACE — distributed hunt for BPB < 1.5")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Start a race
    Start {
        /// Number of parallel workers
        #[arg(long, default_value = "4")]
        workers: usize,

        /// Trials per worker
        #[arg(long, default_value = "10")]
        trials_per_worker: usize,

        /// Machine ID for this worker
        #[arg(long, env = "MACHINE_ID", default_value = "local")]
        machine: String,

        /// ASHA initial trials
        #[arg(long, default_value = "64")]
        asha_trials: usize,
    },

    /// Query race status
    Status {
        /// Show detailed trial information
        #[arg(long)]
        verbose: bool,
    },

    /// Show best BPB found
    Best,

    /// Start dashboard
    Dashboard {
        /// Port for dashboard server
        #[arg(long, default_value = "5173")]
        port: u16,

        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
}

/// Run the CLI.
pub async fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { workers, trials_per_worker, machine, asha_trials } => {
            cmd_start(workers, trials_per_worker, machine, asha_trials).await?
        }
        Commands::Status { verbose } => {
            cmd_status(verbose).await?
        }
        Commands::Best => {
            cmd_best().await?
        }
        Commands::Dashboard { port, bind } => {
            cmd_dashboard(port, bind).await?
        }
    }

    Ok(())
}

/// Start a new race.
async fn cmd_start(workers: usize, trials_per_worker: usize, machine: String, asha_trials: usize) -> Result<()> {
    info!("IGLA RACE START");
    info!("  workers: {}", workers);
    info!("  trials per worker: {}", trials_per_worker);
    info!("  machine: {}", machine);
    info!("  ASHA initial trials: {}", asha_trials);

    // TODO: Implement actual race start logic
    // This is a stub - will be filled with real race execution

    info!("Race complete");
    Ok(())
}

/// Query race status.
async fn cmd_status(verbose: bool) -> Result<()> {
    info!("Querying race status...");

    let status = QueryStatus::new()?.execute().await?;

    println!("IGLA RACE Status");
    println!("================");
    println!("Best BPB: {:.4}", status.best_bpb);
    println!("Total trials: {}", status.total_trials);
    println!("Active workers: {}", status.active_workers);

    if verbose {
        println!("\nRecent trials:");
        for trial in status.recent_trials {
            println!("  Trial {}: BPB={:.4}, rung={}",
                     trial.trial_id, trial.bpb, trial.rung);
        }
    }

    Ok(())
}

/// Show best BPB.
async fn cmd_best() -> Result<()> {
    info!("Querying best BPB...");

    let status = QueryStatus::new()?.execute().await?;

    println!("{:.4}", status.best_bpb);

    Ok(())
}

/// Start dashboard.
async fn cmd_dashboard(port: u16, bind: String) -> Result<()> {
    info!("Starting dashboard on {}:{}", bind, port);

    // TODO: Implement dashboard server
    // This is a stub - will be filled with real dashboard

    info!("Dashboard running at http://{}:{}", bind, port);
    Ok(())
}
