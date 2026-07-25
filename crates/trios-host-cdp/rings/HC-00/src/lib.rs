//! HC-00 — minimal Chrome DevTools Protocol client over a raw WebSocket.
//!
//! Design goals (Ring Isolation):
//! - no chromiumoxide/heavy CDP stacks — one WS connection, id-correlated
//!   request/response, protocol events are skipped;
//! - target discovery through the DevTools HTTP endpoint (`/json/list`);
//! - everything typed as `serde_json::Value` — HC-01 owns the semantics.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

/// Discover the WebSocket debugger URL of the first `page` target.
///
/// `http_base` is the DevTools HTTP endpoint, e.g. `http://127.0.0.1:9102`.
pub async fn discover_page_ws(http_base: &str) -> Result<String> {
    let base = http_base.trim_end_matches('/');
    let url = format!("{base}/json/list");
    let targets: Vec<Value> = reqwest::get(&url)
        .await
        .with_context(|| format!("GET {url}"))?
        .json()
        .await
        .context("parse /json/list")?;
    targets
        .iter()
        .find(|t| t["type"] == "page")
        .or_else(|| targets.first())
        .and_then(|t| t["webSocketDebuggerUrl"].as_str())
        .map(String::from)
        .ok_or_else(|| anyhow!("no debuggable targets at {base} (is the browser running with --remote-debugging-port?)"))
}

type Pending = Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>;

/// Id-correlated CDP client. Cheap to clone; the WS connection lives in a
/// background task that routes responses back by `id` and drops events.
#[derive(Clone)]
pub struct CdpClient {
    tx: mpsc::UnboundedSender<Message>,
    pending: Pending,
    next_id: Arc<AtomicU64>,
}

impl CdpClient {
    /// Connect to a target's `webSocketDebuggerUrl`.
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (stream, _) = tokio_tungstenite::connect_async(ws_url)
            .await
            .with_context(|| format!("connect CDP ws {ws_url}"))?;
        let (mut sink, mut source) = stream.split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
        let pending: Pending = Arc::new(Mutex::new(HashMap::new()));

        // Writer task.
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Reader task: route responses by id, skip protocol events.
        let pending_reader = pending.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = source.next().await {
                let Message::Text(text) = msg else { continue };
                let Ok(value) = serde_json::from_str::<Value>(&text) else {
                    continue;
                };
                let Some(id) = value["id"].as_u64() else {
                    continue; // CDP event (method + params, no id) — not ours.
                };
                if let Some(waiter) = pending_reader.lock().await.remove(&id) {
                    let _ = waiter.send(value);
                }
            }
            // Connection gone: wake every waiter with an error marker.
            let mut map = pending_reader.lock().await;
            for (_, waiter) in map.drain() {
                let _ = waiter.send(json!({"error": {"message": "CDP connection closed"}}));
            }
        });

        Ok(Self {
            tx,
            pending,
            next_id: Arc::new(AtomicU64::new(1)),
        })
    }

    /// Issue a CDP method call and wait for its response `result`.
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx_done, rx_done) = oneshot::channel();
        self.pending.lock().await.insert(id, tx_done);

        let payload = json!({"id": id, "method": method, "params": params});
        self.tx
            .send(Message::Text(payload.to_string()))
            .map_err(|_| anyhow!("CDP writer task is gone"))?;

        let response = tokio::time::timeout(std::time::Duration::from_secs(20), rx_done)
            .await
            .map_err(|_| anyhow!("CDP call {method} timed out"))?
            .map_err(|_| anyhow!("CDP call {method}: response channel dropped"))?;

        if let Some(err) = response.get("error") {
            bail!(
                "CDP {method} failed: {}",
                err["message"].as_str().unwrap_or("unknown error")
            );
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}
