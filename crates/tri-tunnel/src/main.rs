mod tunnel;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::info;

/// tri-tunnel — Tailscale Funnel management for trios-server
#[derive(Parser, Debug)]
#[command(name = "tri-tunnel")]
#[command(about = "Manage Tailscale Funnel for trios-server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the funnel on the trios-server port
    Start {
        /// Port to forward (default: 9005)
        #[arg(short, long, default_value = "9005")]
        port: u16,
    },
    /// Stop the funnel
    Stop,
    /// Show funnel status
    Status,
}

const TAILSCALE_CLI: &str = "/Applications/Tailscale.app/Contents/MacOS/Tailscale";

fn check_tailscale() -> Result<()> {
    if !std::path::Path::new(TAILSCALE_CLI).exists() {
        bail!(
            "Tailscale CLI not found at {}\nInstall from App Store: https://apps.apple.com/app/tailscale/id1475387142",
            TAILSCALE_CLI
        );
    }
    Ok(())
}

async fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tri_tunnel=info")
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { port } => {
            check_tailscale()?;

            info!("Starting Tailscale Funnel on port {}...", port);
            tunnel::start(port).await?;
            info!("Funnel started successfully");
            info!("Your trios-server is now accessible via:");
            info!("  https://<your-device>.ts.net/");
        }
        Commands::Stop => {
            check_tailscale()?;
            info!("Stopping Tailscale Funnel...");
            tunnel::stop().await?;
            info!("Funnel stopped");
        }
        Commands::Status => {
            check_tailscale()?;
            tunnel::status().await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
    Ok(())
}
