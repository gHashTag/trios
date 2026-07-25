//! HC-02 — the host agent's poll loop.
//!
//! Speaks the trios-server WS protocol (`{method, params}` → `{result}`):
//! 1. `browser/poll {agent_id}` — fetch pending SR-03 commands;
//! 2. execute each through a `CommandExecutor` (HC-01 over CDP);
//! 3. `browser/result {command_id, agent_id, success, result, error}`.
//!
//! The server interleaves broadcast events (`{"event": ...}`) on the same
//! socket — those are skipped. Connection loss triggers reconnect with
//! backoff.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use trios_a2a::BrowserCommand;

/// Executes one command; the value becomes `BrowserResult.data`.
#[async_trait::async_trait]
pub trait CommandExecutor: Send + Sync {
    async fn execute(&self, command: &BrowserCommand) -> Result<Value>;
}

/// HC-01 executor over any CDP transport.
pub struct CdpExecutor<T: trios_host_cdp_hc01::CdpCall> {
    pub cdp: T,
}

#[async_trait::async_trait]
impl<T: trios_host_cdp_hc01::CdpCall> CommandExecutor for CdpExecutor<T> {
    async fn execute(&self, command: &BrowserCommand) -> Result<Value> {
        trios_host_cdp_hc01::execute_command(&self.cdp, command).await
    }
}

#[derive(Debug, Clone)]
pub struct PollerConfig {
    /// trios-server WS endpoint, e.g. `ws://127.0.0.1:9005/ws`.
    pub server_ws: String,
    /// Agent id used by `/agent/run` callers (`browser_agent_id`).
    pub agent_id: String,
    pub poll_interval: Duration,
    /// Stop after N poll rounds (tests); `None` — run forever.
    pub max_polls: Option<usize>,
    pub reconnect_backoff: Duration,
}

impl PollerConfig {
    pub fn from_env() -> Self {
        Self {
            server_ws: std::env::var("TRIOS_SERVER_WS")
                .unwrap_or_else(|_| "ws://127.0.0.1:9005/ws".into()),
            agent_id: std::env::var("TRIOS_BROWSER_AGENT_ID")
                .unwrap_or_else(|_| "host-cdp".into()),
            poll_interval: Duration::from_millis(
                std::env::var("TRIOS_POLL_INTERVAL_MS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1000),
            ),
            max_polls: None,
            reconnect_backoff: Duration::from_secs(3),
        }
    }
}

type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Send one `{method, params}` request and wait for the next `{result}`
/// frame, skipping interleaved `{"event": ...}` broadcasts.
async fn ws_call(ws: &mut WsStream, method: &str, params: Value) -> Result<Value> {
    ws.send(Message::Text(
        json!({"method": method, "params": params}).to_string(),
    ))
    .await
    .context("ws send")?;
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(30), ws.next())
            .await
            .map_err(|_| anyhow!("ws call {method}: response timed out"))?
            .ok_or_else(|| anyhow!("ws closed"))??;
        let Message::Text(text) = msg else { continue };
        let value: Value = serde_json::from_str(&text).context("ws frame parse")?;
        if value.get("event").is_some() {
            continue; // broadcast noise
        }
        if let Some(result) = value.get("result") {
            return Ok(result.clone());
        }
    }
}

/// One poll round on an open socket: fetch commands, execute, report.
/// Returns the number of commands processed.
pub async fn poll_round(
    ws: &mut WsStream,
    agent_id: &str,
    executor: &dyn CommandExecutor,
) -> Result<usize> {
    let polled = ws_call(ws, "browser/poll", json!({"agent_id": agent_id})).await?;
    let commands: Vec<BrowserCommand> = serde_json::from_value(
        polled.get("commands").cloned().unwrap_or(json!([])),
    )
    .context("parse polled commands")?;

    let count = commands.len();
    for command in commands {
        let started = std::time::Instant::now();
        let (success, result, error) = match executor.execute(&command).await {
            Ok(data) => (true, data, Value::Null),
            Err(err) => (false, Value::Null, json!(err.to_string())),
        };
        info!(
            command = %command.command_type,
            id = %command.id,
            success,
            ms = started.elapsed().as_millis() as u64,
            "executed browser command"
        );
        ws_call(
            ws,
            "browser/result",
            json!({
                "command_id": command.id,
                "agent_id": agent_id,
                "success": success,
                "result": result,
                "error": error,
            }),
        )
        .await?;
    }
    Ok(count)
}

/// Main loop: connect → poll rounds → reconnect on failure.
pub async fn run(config: PollerConfig, executor: &dyn CommandExecutor) -> Result<()> {
    let mut rounds = 0usize;
    'outer: loop {
        let ws = match tokio_tungstenite::connect_async(&config.server_ws).await {
            Ok((ws, _)) => ws,
            Err(err) => {
                warn!("connect {} failed: {err}; retrying", config.server_ws);
                tokio::time::sleep(config.reconnect_backoff).await;
                continue;
            }
        };
        info!("connected to {} as `{}`", config.server_ws, config.agent_id);
        let mut ws = ws;
        loop {
            match poll_round(&mut ws, &config.agent_id, executor).await {
                Ok(_) => {}
                Err(err) => {
                    warn!("poll round failed: {err}; reconnecting");
                    tokio::time::sleep(config.reconnect_backoff).await;
                    continue 'outer;
                }
            }
            rounds += 1;
            if let Some(max) = config.max_polls {
                if rounds >= max {
                    return Ok(());
                }
            }
            tokio::time::sleep(config.poll_interval).await;
        }
    }
}
