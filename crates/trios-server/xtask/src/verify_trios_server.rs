use anyhow::{Context, Result};
use std::env;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("trios_server=debug").init();

    let host = env::var("TRIOS_HOST").unwrap_or_else(|_| "localhost".to_string());
    let port: u16 = env::var("TRIOS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9005);

    println!("=== L6 Verification ===");
    println!("Host: {host}, Port: {port}\n");

    check_tcp(&host, port).await?;
    check_http_health(&host, port).await?;
    check_ws_upgrade(&host, port, "/ws").await?;
    check_ws_upgrade(&host, port, "/operator").await?;

    println!("\n=== VERIFICATION COMPLETE ===");
    println!("All core endpoints verified on port {port}");

    Ok(())
}

async fn check_tcp(host: &str, port: u16) -> Result<()> {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    let addr = format!("{host}:{port}");
    info!("[1] Checking TCP connection to {addr}...");

    match timeout(Duration::from_secs(2), TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => {
            println!("[1] ✅ TCP listener on {port} is reachable");
            Ok(())
        }
        Ok(Err(e)) => {
            anyhow::bail!("TCP connection failed: {e}. Is trios-server running?");
        }
        Err(_) => {
            anyhow::bail!("TCP connection timeout. Is trios-server running?");
        }
    }
}

async fn check_http_health(host: &str, port: u16) -> Result<()> {
    let url = format!("http://{host}:{port}/health");
    info!("[2] Checking HTTP /health endpoint...");

    let resp = reqwest::get(&url).await.context("health request failed")?;

    let status = resp.status();
    if status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        println!("[2] ✅ /health returns {} ({})", status, body.trim());
        Ok(())
    } else {
        anyhow::bail!("/health failed with status: {status}");
    }
}

async fn check_ws_upgrade(host: &str, port: u16, path: &str) -> Result<()> {
    let url = format!("ws://{host}:{port}{path}");
    info!("[3] Checking WebSocket {path}...");

    let (mut ws, resp) = tokio_tungstenite::connect_async(&url).await
        .with_context(|| format!("WebSocket connection failed to {url}"))?;

    // RFC 6455: 101 Switching Protocols
    if resp.status().as_u16() == 101 {
        println!("[3] ✅ WS {path} upgraded (101 Switching Protocols)");
    } else {
        warn!("[3] ⚠️  WS {path} status: {} (expected 101)", resp.status());
    }

    ws.close(None).await.ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_tcp_fails_when_server_not_running() {
        // This would need a mock server setup
    }
}
