//! trios-mcp CLI
//!
//! Run with: `cargo run -p trios-mcp`

use std::env;
use tracing_subscriber::fmt;

use trios_mcp::{ConnectionConfig, McpClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            env::var("RUST_LOG")
                .unwrap_or_else(|_| "trios_mcp=info,warn".to_string()),
        )
        .init();

    println!("════════════════════════════════════════════════════════════════");
    println!("🔥 trios-mcp — MCP Client Adapter");
    println!("════════════════════════════════════════════════════════════════");
    println!();

    // Get configuration from env or use defaults
    let host = env::var("MCP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("MCP_PORT")
        .unwrap_or_else(|_| "3025".to_string())
        .parse()
        .unwrap_or(3025);
    let username = env::var("MCP_USERNAME").unwrap_or_else(|_| "perplexity".to_string());
    let password = env::var("MCP_PASSWORD").unwrap_or_else(|_| "test123".to_string());
    let use_tls = env::var("MCP_TLS")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    println!("📍 Server: {}://{}:{}",
        if use_tls { "wss" } else { "ws" }, host, port);
    println!("🔐 Auth: {}/***", username);
    println!();

    let config = ConnectionConfig::new(host, port)
        .with_auth(trios_mcp::AuthConfig::new(username, password))
        .with_tls(use_tls);

    let mut client = McpClient::new(config);

    println!("🔌 Connecting...");
    client.connect().await?;
    println!("✅ Connected!");
    println!();

    println!("📋 Available tools:");
    let tools = client.list_tools().await?;
    for tool in &tools {
        println!("   • {}: {}", tool.name, tool.description);
    }
    println!();

    println!("════════════════════════════════════════════════════════════════");
    println!("Press Ctrl+C to exit");
    println!("════════════════════════════════════════════════════════════════");

    // Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n👋 Shutting down...");

    client.close().await?;
    println!("✅ Done!");

    Ok(())
}
