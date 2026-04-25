use anyhow::{Context, Result};
use tokio::process::Command as AsyncCommand;

const TAILSCALE_CLI: &str = "/Applications/Tailscale.app/Contents/MacOS/Tailscale";

pub async fn start(port: u16) -> Result<()> {
    // First, check if funnel is already running
    let status = check_status().await?;
    if status {
        anyhow::bail!("Funnel is already running. Stop it first with `tri-tunnel stop`");
    }

    let output = AsyncCommand::new(TAILSCALE_CLI)
        .args(["funnel", "--bg", &port.to_string()])
        .output()
        .await
        .context("Failed to start funnel")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to start funnel: {}", stderr);
    }

    // Give it more time to initialize
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Check if funnel is actually accessible
    let funnel_active = check_status().await?;
    if !funnel_active {
        // Try alternative check - see if port is listening
        if !is_port_listening(port).await? {
            anyhow::bail!("Funnel started but port {} is not listening. Check Tailscale logs.", port);
        }
    }

    Ok(())
}

async fn is_port_listening(port: u16) -> Result<bool> {
    let output = AsyncCommand::new("lsof")
        .args(["-i", &format!(":{}", port)])
        .output()
        .await;

    match output {
        Ok(result) => Ok(result.status.success()),
        Err(_) => Ok(false),
    }
}

pub async fn stop() -> Result<()> {
    let output = AsyncCommand::new(TAILSCALE_CLI)
        .args(["funnel", "--https=443", "off"])
        .output()
        .await
        .context("Failed to stop funnel")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to stop funnel: {}", stderr);
    }

    Ok(())
}

pub async fn status() -> Result<()> {
    let output = AsyncCommand::new(TAILSCALE_CLI)
        .args(["status", "--json"])
        .output()
        .await
        .context("Failed to get Tailscale status")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to get status: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .context("Failed to parse Tailscale status JSON")?;

    // Check if funnel is active
    let funnel_active = check_status().await?;

    println!("╔════════════════════════════════════════╗");
    println!("║      tri-tunnel Status               ║");
    println!("╠════════════════════════════════════════╣");

    if let Some(self_info) = json.get("Self").and_then(|v| v.as_object()) {
        if let Some(name) = self_info.get("HostName").and_then(|v| v.as_str()) {
            println!("║ Device: {:<28} ║", name);
        }
        if let Some(tailnet) = self_info.get("TailnetName").and_then(|v| v.as_str()) {
            println!("║ Tailnet: {:<27} ║", tailnet);
        }
    }

    println!("║ Funnel: {:<28} ║", if funnel_active { "ACTIVE ✅" } else { "INACTIVE ❌" });

    if funnel_active {
        if let Some(self_info) = json.get("Self").and_then(|v| v.as_object()) {
            if let Some(dns_name) = self_info.get("DNSName").and_then(|v| v.as_str()) {
                let url = format!("https://{}/", dns_name.trim_end_matches('.'));
                println!("║ URL: {:<31} ║", url);
            }
        }
        println!("║ Port: 9005                            ║");
    }

    println!("╚════════════════════════════════════════╝");

    Ok(())
}

async fn check_status() -> Result<bool> {
    let output = AsyncCommand::new(TAILSCALE_CLI)
        .args(["funnel", "status", "--json"])
        .output()
        .await;

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({}));

            // Check if "Web" key exists (indicates funnel is active)
            if json.get("Web").and_then(|v| v.as_object()).is_some() {
                return Ok(true);
            }

            // Check for Handlers with proxy to our port
            if let Some(web) = json.get("Web").and_then(|v| v.as_object()) {
                for (_domain, config) in web {
                    if let Some(handlers) = config.get("Handlers").and_then(|v| v.as_object()) {
                        if handlers.get("/").is_some() {
                            return Ok(true);
                        }
                    }
                }
            }

            Ok(false)
        }
        _ => Ok(false),
    }
}
