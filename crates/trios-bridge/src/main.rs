//! Trinity Agent Bridge — CLI entry point.
//!
//! ```bash
//! trios-bridge                          # Start server on port 7474
//! trios-bridge --port 8080              # Custom port
//! trios-bridge --repo gHashTag/trios    # GitHub repo for issue parsing
//! ```

use anyhow::Result;
use clap::Parser;
use std::sync::Arc;

use trios_bridge::BridgeServer;

/// Trinity Agent Bridge — WebSocket server for multi-agent orchestration.
#[derive(Parser, Debug)]
#[command(name = "trios-bridge")]
#[command(version)]
struct Args {
    /// Port to listen on (default: 7474 = T-R-I-N)
    #[arg(short, long, default_value_t = 7474)]
    port: u16,

    /// GitHub repository (owner/repo) for issue parsing
    #[arg(short, long, default_value = "gHashTag/trios")]
    repo: String,

    /// GitHub personal access token (optional, for private repos)
    #[arg(short, long)]
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("trios_bridge=info"))
        )
        .init();

    let args = Args::parse();
    let ws_addr = std::net::SocketAddr::from(([0, 0, 0, 0], args.port));
    let relay_port = args.port + 1; // Relay on port+1 (default: 7475)
    let relay_addr = std::net::SocketAddr::from(([0, 0, 0, 0], relay_port));

    tracing::info!("🚀 Starting Trinity Agent Bridge");
    tracing::info!("   Repo: {}", args.repo);
    tracing::info!("   WebSocket: ws://{}", ws_addr);
    tracing::info!("   Relay HTTP: http://{}", relay_addr);

    let server = Arc::new(BridgeServer::new(&args.repo, args.token));

    // Start relay HTTP+SSE server in background
    let relay_server = server.clone();
    tokio::spawn(async move {
        if let Err(e) = trios_bridge::relay::serve_relay(relay_server, relay_addr).await {
            tracing::error!("Relay server error: {}", e);
        }
    });

    // Start WebSocket server (blocking)
    server.serve(ws_addr).await?;

    Ok(())
}
