//! `tri railway` — Railway deployment commands
//!
//! Usage:
//!   tri railway deploy --seeds 4
//!   tri railway status

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum RailwayCommand {
    /// Deploy N Railway instances with unique seeds
    Deploy {
        /// Number of instances to deploy (default: 1)
        #[arg(long, default_value_t = 1)]
        seeds: u32,

        /// Starting seed value (default: 42)
        #[arg(long, default_value_t = 42)]
        start_seed: u64,

        /// Dry run: show what would be deployed without deploying
        #[arg(long)]
        dry_run: bool,
    },
    /// Show Railway deployment status
    Status,
}

pub fn run(cmd: RailwayCommand) -> Result<()> {
    match cmd {
        RailwayCommand::Deploy {
            seeds,
            start_seed,
            dry_run,
        } => deploy_parallel(seeds, start_seed, dry_run),
        RailwayCommand::Status => show_status(),
    }
}

fn deploy_parallel(seeds: u32, start_seed: u64, dry_run: bool) -> Result<()> {
    // L-R1: MAX 4 parallel instances (Railway law from #143)
    const MAX_INSTANCES: u32 = 4;

    if seeds > MAX_INSTANCES {
        anyhow::bail!(
            "L-R1 violation: MAX {} Railway instances allowed, requested {}. \
             See #143 for Railway laws. Use --seeds {} or less.",
            MAX_INSTANCES,
            seeds,
            MAX_INSTANCES
        );
    }

    if dry_run {
        println!("🔍 Dry run: would deploy {} Railway instances", seeds);
        for i in 0..seeds {
            let seed = start_seed + i as u64;
            let service_name = format!("igla-trainer-seed-{}", seed);
            println!("  - {} (seed: {})", service_name, seed);
        }
        return Ok(());
    }

    println!("🚀 Railway deployment plan for {} seeds ({}-{})...", seeds, start_seed, start_seed + seeds as u64 - 1);
    println!();

    // Railway CLI requires TTY for interactive prompts - provide manual commands
    println!("📋 Run these commands in a TTY terminal:");
    println!();

    for i in 0..seeds {
        let seed = start_seed + i as u64;
        let service_name = format!("igla-trainer-seed-{}", seed);

        println!("# Seed {}", seed);
        println!("railway add --service {} --variables \"RAILWAY_SEED={}\"", service_name, seed);
        println!("railway up --service {}", service_name);
        println!();
    }

    println!("Or one-liner for all seeds:");
    println!("for seed in {} {}; do", start_seed, start_seed + seeds as u64 - 1);
    println!("  railway add --service \"igla-trainer-seed-{{seed}}\" --variables \"RAILWAY_SEED={{seed}}\"");
    println!("  railway up --service \"igla-trainer-seed-{{seed}}\"");
    println!("done");

    Ok(())
}

fn show_status() -> Result<()> {
    println!("📊 Checking Railway status...");

    let result = std::process::Command::new("railway")
        .args(["status"])
        .status();

    match result {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => {
            eprintln!("❌ Railway status check failed");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Failed to run railway CLI: {}", e);
            eprintln!("   Install Railway CLI: https://docs.railway.app/reference/cli");
            Ok(())
        }
    }
}
