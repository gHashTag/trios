//! BR-XTASK: Launcher
//!
//! Single command `cargo run -p trios-mcp` to launch all rings in parallel.
//! Replaces `package.json` scripts from browser-tools-mcp (bun dev).

use anyhow::Result;
use std::env;
use tokio::{select, signal};
use tracing::{info, warn};

// Ring dependencies
use trios_mcp_sr00::BrowserConnector;
use trios_mcp_sr01::lib as LighthouseBridge;
use trios_mcp_sr02::McpServer;

/// Parse environment variables
fn parse_env() -> Result<(String, u16)> {
    let host = env::var("MCP_HOST")
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let port: u16 = env::var("MCP_PORT")
        .unwrap_or_else(|_| "3025".to_string())
        .parse()
        .context("Invalid MCP_PORT: must be a valid port number")?;

    Ok((host, port))
}

/// Initialize logging
fn init_logging() {
    let filter = env::var("RUST_LOG")
        .unwrap_or_else(|_| "trios_mcp=info,warn".to_string());

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logging();

    println!("══════════════════════════════════════════");
    println!("🔥 trios-mcp — Browser Tools MCP Server");
    println!("════════════════════════════════════════════");

    let (host, port) = parse_env()?;

    println!();
    println!("📍 Server: {}://{}:{} (port 3025)", "http", host, port);
    println!("🔐 Auth: admin/***");
    println!();
    println!("══════════════════════════════════════════════");

    // Launch SR-00 (HTTP + WebSocket) in background
    let sr00_host = host.clone();
    let sr00_port = port;

    let sr00_handle = tokio::spawn(async move {
        info!("🚀 Launching SR-00 (HTTP + WebSocket)...");
        let connector = BrowserConnector::new(sr00_host, sr00_port);
        connector.run().await
    });

    // Launch SR-02 (MCP stdio) in foreground
    let sr02_host = host.clone();
    let sr02_port = port;

    let sr02_handle = tokio::spawn(async move {
        info!("🚀 Launching SR-02 (MCP stdio)...");
        let server = McpServer::new(sr02_host, sr02_port);
        server.run_stdio().await
    });

    // Wait for Ctrl+C
    tokio::select! {
        signal::ctrl_c(),
        // If any ring panics, we'll exit via select
        _ = match &mut [sr00_handle, sr02_handle] {
            res => {
                warn!("{} exited unexpectedly", res);
                std::process::exit(1);
            }
        }
    }?;

    println!();
    println!("════════════════════════════════════════════");
    println!("👋 Shutting down...");
    println!("══════════════════════════════════════════");

    Ok(())
}
