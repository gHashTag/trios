//! `trios-doctor-loop` — autonomous Queen↔Doctor loop client (Doctor side).
//!
//! Connects to `trios-server` via WebSocket (`/ws`), subscribes to broadcast
//! events, filters `QueenOrder` events targeting `target_agent == "doctor"`,
//! executes the corresponding action through the existing `Doctor` API, and
//! publishes a `DoctorReport` back onto the bus via `doctor/report` method.
//!
//! Constitutional anchors:
//! - L1  — pure Rust, no shell scripts.
//! - L8  — push-first: this binary is committed before being run remotely.
//! - L21 — append-only: events are broadcast, never mutated.
//! - L24 — agent-to-agent traffic via the canonical bus, not sibling sockets.
//! - AGENTS.md AGENT T 6-phase cycle: this binary lives in phase RUN as the
//!   Doctor's executor, and emits VERDICT input via DoctorReport.
//!
//! Anchor: φ² + φ⁻² = 3.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream, WebSocketStream,
};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, warn};
use uuid::Uuid;

use trios_doctor::{CheckStatus, Doctor};

type WsClient = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// CLI configuration.
#[derive(Debug, Clone)]
struct LoopConfig {
    /// WebSocket URL (e.g. `ws://127.0.0.1:9005/ws`).
    ws_url: String,
    /// Workspace root that Doctor diagnoses.
    workspace_root: PathBuf,
    /// Agent identifier published in DoctorReport.
    agent_id: String,
    /// Reconnect delay on transport error.
    reconnect_delay: Duration,
}

impl LoopConfig {
    fn from_env_and_args() -> Result<Self> {
        let ws_url = std::env::var("TRIOS_DOCTOR_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:9005/ws".to_string());

        let workspace_root = std::env::var("TRIOS_WORKSPACE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .context("cannot determine workspace root (set TRIOS_WORKSPACE)")?;

        let agent_id =
            std::env::var("TRIOS_DOCTOR_AGENT_ID").unwrap_or_else(|_| "doctor".to_string());

        let reconnect_secs: u64 = std::env::var("TRIOS_DOCTOR_RECONNECT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        Ok(Self {
            ws_url,
            workspace_root,
            agent_id,
            reconnect_delay: Duration::from_secs(reconnect_secs),
        })
    }
}

/// Subset of the bus event we care about. Only `QueenOrder` is interesting
/// to the Doctor loop. Any other variant is ignored.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
#[allow(clippy::large_enum_variant)]
enum InboundEvent {
    QueenOrder {
        order_id: String,
        action: String,
        target_agent: String,
        params: Value,
        ts: i64,
    },
    /// Anything else — we tolerate it without erroring out.
    #[serde(other)]
    Ignored,
}

#[derive(Debug, Serialize)]
struct WsRequest {
    id: String,
    method: String,
    params: Value,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trios_doctor_loop=info,info")
        .init();

    let cfg = LoopConfig::from_env_and_args()?;
    info!("trios-doctor-loop starting");
    info!("  ws_url        = {}", cfg.ws_url);
    info!("  workspace     = {}", cfg.workspace_root.display());
    info!("  agent_id      = {}", cfg.agent_id);
    info!("  reconnect_in  = {:?}", cfg.reconnect_delay);
    info!("  φ² + φ⁻² = 3   (constitutional anchor)");

    loop {
        match run_session(&cfg).await {
            Ok(()) => {
                warn!("WS session ended cleanly; reconnecting in {:?}", cfg.reconnect_delay);
            }
            Err(e) => {
                error!("WS session failed: {:#}; reconnecting in {:?}", e, cfg.reconnect_delay);
            }
        }
        tokio::time::sleep(cfg.reconnect_delay).await;
    }
}

async fn run_session(cfg: &LoopConfig) -> Result<()> {
    let (ws_stream, response) = connect_async(&cfg.ws_url)
        .await
        .with_context(|| format!("connecting to {}", cfg.ws_url))?;
    info!(
        "WS connected: {} (HTTP {})",
        cfg.ws_url,
        response.status().as_u16()
    );

    let mut ws: WsClient = ws_stream;

    while let Some(msg) = ws.next().await {
        let msg = msg.context("WS recv")?;
        match msg {
            Message::Text(text) => {
                if let Err(e) = handle_text_frame(&text, &mut ws, cfg).await {
                    warn!("frame handling error: {:#}", e);
                }
            }
            Message::Close(_) => {
                info!("WS closed by peer");
                return Ok(());
            }
            Message::Ping(p) => {
                ws.send(Message::Pong(p)).await.ok();
            }
            _ => {}
        }
    }

    Ok(())
}

/// Decide whether a text frame is a `BusEvent` we should react to.
/// Server frames have shape `{"result": ...}` for RPC responses or are the raw
/// `BusEvent` JSON for broadcasts. We only react to broadcasts targeted at us.
async fn handle_text_frame(text: &str, ws: &mut WsClient, cfg: &LoopConfig) -> Result<()> {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return Ok(()), // not JSON → silently ignore
    };

    // Skip RPC responses
    if value.get("result").is_some() && value.get("type").is_none() {
        return Ok(());
    }

    let event: InboundEvent = match serde_json::from_value(value) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    match event {
        InboundEvent::QueenOrder {
            order_id,
            action,
            target_agent,
            params,
            ts,
        } => {
            if target_agent != cfg.agent_id && target_agent != "doctor" {
                return Ok(());
            }
            info!(
                "QueenOrder received: order_id={} action={:?} ts={}",
                order_id, action, ts
            );
            let report = execute_order(&order_id, &action, &params, cfg)?;
            send_report(ws, report).await?;
        }
        InboundEvent::Ignored => {}
    }
    Ok(())
}

/// Honest execution: dispatch by action, run real Doctor checks where applicable.
fn execute_order(
    order_id: &str,
    action: &str,
    _params: &Value,
    cfg: &LoopConfig,
) -> Result<DoctorReportPayload> {
    let doctor = Doctor::new(&cfg.workspace_root);
    let normalized = action.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "doctor scan" | "doctor quick" | "doctor heal" => {
            // All three currently map onto the same workspace diagnosis: the
            // Doctor binary itself only differentiates by exit code, not by
            // separate routines, so we honestly run the same set of checks
            // and label the report with the requested action.
            let diag = doctor.run_all();
            let any_red = diag.checks.iter().any(|c| c.status == CheckStatus::Red);
            let any_yellow = diag.checks.iter().any(|c| c.status == CheckStatus::Yellow);
            let status = if any_red {
                "red"
            } else if any_yellow {
                "yellow"
            } else {
                "green"
            }
            .to_string();
            let summary = format!(
                "{}: {} crates · {} checks · {}",
                action,
                diag.crate_count,
                diag.checks.len(),
                status
            );
            let diagnosis = serde_json::to_value(&diag)
                .unwrap_or_else(|_| json!({"error": "serialize diagnosis failed"}));
            Ok(DoctorReportPayload {
                order_id: order_id.to_string(),
                agent_id: cfg.agent_id.clone(),
                status,
                summary,
                diagnosis,
            })
        }
        other => {
            // Unknown action → honest "noop" report so Queen knows we saw it.
            Ok(DoctorReportPayload {
                order_id: order_id.to_string(),
                agent_id: cfg.agent_id.clone(),
                status: "noop".into(),
                summary: format!("doctor-loop: unsupported action {:?}", other),
                diagnosis: json!({"action": other, "supported": [
                    "doctor scan", "doctor quick", "doctor heal"
                ]}),
            })
        }
    }
}

#[derive(Debug, Serialize)]
struct DoctorReportPayload {
    order_id: String,
    agent_id: String,
    status: String,
    summary: String,
    diagnosis: Value,
}

async fn send_report(ws: &mut WsClient, report: DoctorReportPayload) -> Result<()> {
    let req = WsRequest {
        id: Uuid::new_v4().to_string(),
        method: "doctor/report".into(),
        params: serde_json::to_value(&report)
            .map_err(|e| anyhow!("serialize DoctorReport: {}", e))?,
    };
    let payload = serde_json::to_string(&req)?;
    info!(
        "sending DoctorReport order_id={} status={}",
        report.order_id, report.status
    );
    ws.send(Message::Text(payload)).await?;
    Ok(())
}
