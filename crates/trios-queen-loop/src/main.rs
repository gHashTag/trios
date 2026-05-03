//! `trios-queen-loop` — autonomous Queen↔Doctor loop client (Queen side).
//!
//! Sister-binary to `trios-doctor-loop`. The Queen daemon:
//!   1. Reads `.trinity/queen/{policy,actions,senses,supervisor_state}.json`
//!      from the workspace root.
//!   2. Picks one action permitted by `policy.max_auto_level` (default 3).
//!   3. Publishes a `queen/order` request onto the bus via `/operator` WS.
//!   4. Awaits matching `DoctorReport` events broadcast back on the same bus.
//!   5. Sleeps `tick_secs` and repeats.
//!
//! No fake scripts: every order is a real WS frame, every report is parsed
//! straight from the bus, every action id comes from on-disk JSON.
//!
//! Constitutional anchors:
//! - L1  — pure Rust, no shell scripts.
//! - L8  — push-first.
//! - L11 — soul-name on every order (default: SCARABS).
//! - L21 — append-only events.
//! - L24 — bus-mediated agent traffic.
//! - AGENTS.md AGENT T 6-phase cycle: this binary owns phases PLAN/ASSIGN.
//!
//! Anchor: φ² + φ⁻² = 3.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{error, info, warn};
use uuid::Uuid;

/// Queen-side loop configuration.
#[derive(Debug, Clone)]
struct QueenConfig {
    ws_url: String,
    operator_token: Option<String>,
    workspace_root: PathBuf,
    soul_name: String,
    tick: Duration,
    reconnect_delay: Duration,
    target_agent: String,
    /// Hard ceiling on action.level we are allowed to dispatch.
    /// Read from `.trinity/queen/policy.json` `max_auto_level` field.
    max_auto_level_override: Option<u8>,
}

impl QueenConfig {
    fn from_env() -> Result<Self> {
        let ws_url = std::env::var("TRIOS_QUEEN_WS_URL")
            .unwrap_or_else(|_| "ws://127.0.0.1:9005/operator".to_string());
        let operator_token = std::env::var("TRIOS_OPERATOR_TOKEN").ok().filter(|s| !s.is_empty());
        let workspace_root = std::env::var("TRIOS_WORKSPACE")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .context("cannot determine workspace root (set TRIOS_WORKSPACE)")?;
        let soul_name = std::env::var("TRIOS_QUEEN_SOUL").unwrap_or_else(|_| "SCARABS".to_string());
        let tick_secs: u64 = std::env::var("TRIOS_QUEEN_TICK_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60);
        let reconnect_secs: u64 = std::env::var("TRIOS_QUEEN_RECONNECT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);
        let target_agent =
            std::env::var("TRIOS_QUEEN_TARGET").unwrap_or_else(|_| "doctor".to_string());
        let max_auto_level_override = std::env::var("TRIOS_QUEEN_MAX_LEVEL")
            .ok()
            .and_then(|v| v.parse().ok());
        Ok(Self {
            ws_url,
            operator_token,
            workspace_root,
            soul_name,
            tick: Duration::from_secs(tick_secs),
            reconnect_delay: Duration::from_secs(reconnect_secs),
            target_agent,
            max_auto_level_override,
        })
    }

    fn full_ws_url(&self) -> String {
        match &self.operator_token {
            Some(t) => format!("{}?token={}", self.ws_url, t),
            None => self.ws_url.clone(),
        }
    }
}

/// Snapshot of `.trinity/queen/policy.json`.
#[derive(Debug, Clone, Deserialize)]
struct Policy {
    #[serde(default)]
    god_mode: bool,
    #[serde(default = "default_max_level")]
    max_auto_level: u8,
}

fn default_max_level() -> u8 {
    3
}

/// One row from `.trinity/queen/actions.json`.
#[derive(Debug, Clone, Deserialize)]
struct ActionDef {
    id: String,
    level: u8,
    #[allow(dead_code)]
    #[serde(default)]
    cooldown_sec: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ActionsFile {
    actions: Vec<ActionDef>,
}

#[derive(Debug, Serialize)]
struct WsRequest {
    id: String,
    method: String,
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
#[allow(clippy::large_enum_variant)]
enum InboundEvent {
    DoctorReport {
        order_id: String,
        agent_id: String,
        status: String,
        summary: String,
        diagnosis: Value,
        ts: i64,
    },
    #[serde(other)]
    Ignored,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trios_queen_loop=info,info")
        .init();

    let cfg = QueenConfig::from_env()?;
    info!("trios-queen-loop starting");
    info!("  ws_url        = {}", cfg.ws_url);
    info!("  operator_token= {}", if cfg.operator_token.is_some() { "<set>" } else { "<empty>" });
    info!("  workspace     = {}", cfg.workspace_root.display());
    info!("  soul_name     = {}", cfg.soul_name);
    info!("  tick          = {:?}", cfg.tick);
    info!("  target_agent  = {}", cfg.target_agent);
    info!("  φ² + φ⁻² = 3");

    loop {
        if let Err(e) = run_session(&cfg).await {
            error!("session failed: {:#}; reconnecting in {:?}", e, cfg.reconnect_delay);
        }
        tokio::time::sleep(cfg.reconnect_delay).await;
    }
}

async fn run_session(cfg: &QueenConfig) -> Result<()> {
    let url = cfg.full_ws_url();
    let (ws_stream, response) = connect_async(&url)
        .await
        .with_context(|| format!("connecting to {}", url))?;
    info!("WS connected: {} (HTTP {})", cfg.ws_url, response.status().as_u16());

    let (mut sink, mut stream) = ws_stream.split();

    // Channel for outbound frames (used by the ticker task).
    let (out_tx, mut out_rx) = mpsc::channel::<Message>(32);

    // Ticker task: every `cfg.tick`, picks an action and sends queen/order.
    let cfg_tick = cfg.clone();
    let out_tx_tick = out_tx.clone();
    let _ticker = tokio::spawn(async move {
        // One initial send on startup so the first DoctorReport doesn't wait
        // a full tick.
        if let Err(e) = send_one_order(&cfg_tick, &out_tx_tick).await {
            warn!("initial order send failed: {:#}", e);
        }
        let mut ticker = tokio::time::interval(cfg_tick.tick);
        // first tick fires immediately — skip
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = send_one_order(&cfg_tick, &out_tx_tick).await {
                warn!("order send failed: {:#}", e);
            }
        }
    });

    // Pending orders we are still awaiting reports for (bounded ring buffer).
    let mut pending: VecDeque<String> = VecDeque::with_capacity(64);

    loop {
        tokio::select! {
            Some(out_msg) = out_rx.recv() => {
                // ticker asked us to send a queen/order — extract order_id for tracking
                if let Message::Text(ref txt) = out_msg {
                    if let Some(order_id) = peek_order_id(txt) {
                        if pending.len() >= 64 { pending.pop_front(); }
                        pending.push_back(order_id);
                    }
                }
                sink.send(out_msg).await.context("WS send")?;
            }
            inbound = stream.next() => {
                let msg = match inbound {
                    Some(m) => m.context("WS recv")?,
                    None => return Ok(()),
                };
                match msg {
                    Message::Text(text) => {
                        handle_inbound(&text, &mut pending);
                    }
                    Message::Close(_) => return Ok(()),
                    Message::Ping(p) => { sink.send(Message::Pong(p)).await.ok(); }
                    _ => {}
                }
            }
        }
    }
}

fn peek_order_id(frame: &str) -> Option<String> {
    let v: Value = serde_json::from_str(frame).ok()?;
    v.get("params")
        .and_then(|p| p.get("__order_id"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
}

fn handle_inbound(text: &str, pending: &mut VecDeque<String>) {
    let value: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return,
    };

    // RPC reply (e.g. {"result": {"ok": true, "order_id": "..."}}).
    if let Some(result) = value.get("result") {
        if let Some(oid) = result.get("order_id").and_then(|v| v.as_str()) {
            info!("server ack for order_id={}", oid);
        }
        return;
    }

    let event: InboundEvent = match serde_json::from_value(value) {
        Ok(e) => e,
        Err(_) => return,
    };
    if let InboundEvent::DoctorReport { order_id, agent_id, status, summary, diagnosis, ts } = event {
        let known = pending.iter().any(|p| *p == order_id);
        if known {
            pending.retain(|p| *p != order_id);
        }
        info!(
            "DoctorReport · order_id={} · agent={} · status={} · ts={} · known={} · {}",
            order_id, agent_id, status, ts, known, summary
        );
        // Dump diagnosis at debug level so operators can inspect via env_filter.
        tracing::debug!("diagnosis: {}", diagnosis);
    }
}

async fn send_one_order(cfg: &QueenConfig, out_tx: &mpsc::Sender<Message>) -> Result<()> {
    let policy = read_policy(&cfg.workspace_root).unwrap_or(Policy {
        god_mode: false,
        max_auto_level: default_max_level(),
    });
    let actions = read_actions(&cfg.workspace_root).unwrap_or_default();
    if actions.is_empty() {
        warn!("no actions found in .trinity/queen/actions.json — skipping tick");
        return Ok(());
    }

    let cap = cfg.max_auto_level_override.unwrap_or(policy.max_auto_level);
    let candidates: Vec<&ActionDef> = actions
        .iter()
        .filter(|a| a.id.starts_with("doctor "))
        .filter(|a| a.level <= cap)
        .collect();
    if candidates.is_empty() {
        warn!(
            "no doctor-* actions at level <= {} (god_mode={}); skipping",
            cap, policy.god_mode
        );
        return Ok(());
    }

    // Deterministic, low-noise selection: rotate by epoch_secs / tick.
    let pick_idx = (epoch_secs() as usize / cfg.tick.as_secs().max(1) as usize) % candidates.len();
    let picked = candidates[pick_idx];

    let order_id = Uuid::new_v4().to_string();
    let req = WsRequest {
        id: Uuid::new_v4().to_string(),
        method: "queen/order".into(),
        params: json!({
            "action": picked.id,
            "target_agent": cfg.target_agent,
            "soul": cfg.soul_name,
            "params": {},
            "__order_id": order_id,
        }),
    };
    info!("queen/order → action={:?} (level {}) order_id={}", picked.id, picked.level, order_id);
    let body = serde_json::to_string(&req).map_err(|e| anyhow!("serialize: {}", e))?;
    out_tx.send(Message::Text(body)).await.map_err(|e| anyhow!("send: {}", e))?;
    Ok(())
}

fn read_policy(root: &Path) -> Result<Policy> {
    let p = root.join(".trinity/queen/policy.json");
    let data = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
    let policy: Policy = serde_json::from_slice(&data)?;
    Ok(policy)
}

fn read_actions(root: &Path) -> Result<Vec<ActionDef>> {
    let p = root.join(".trinity/queen/actions.json");
    let data = std::fs::read(&p).with_context(|| format!("read {}", p.display()))?;
    let actions: ActionsFile = serde_json::from_slice(&data)?;
    Ok(actions.actions)
}

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
