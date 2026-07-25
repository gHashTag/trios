//! REST + SSE A2A surface — wire-compatible with the Swift
//! `A2ARegistryClient` (rings/SR-02 of the macOS client).
//!
//! This is the final piece of TS-retirement item 5: the exact HTTP contract
//! previously served by the Hono `apps/server` A2A routes in browseros, now
//! consolidated into the single Rust `trios-server`.
//!
//! Wire-contract notes (verified against the Swift client source):
//! - `GET /a2a/agents` returns a **bare JSON array** of agent cards. The TS
//!   server wrapped it in `{"agents": [...]}` which the Swift decoder
//!   (`decode([AgentCard].self)`) could never parse — a latent TS bug fixed
//!   here, not reproduced.
//! - `A2AMessage.payload` on the Swift side is `Data`, which Codable
//!   (de)serializes as a **base64 string**. String payloads pass through
//!   verbatim (Swift→Swift is already base64); object payloads are
//!   base64-encoded JSON. The TS server emitted plain JSON strings, which
//!   Swift silently dropped in `parseSSELine` — also fixed here.
//! - Task enum values are Swift raw values (`inProgress`), not snake_case;
//!   task JSON keys are camelCase (`createdAt`).

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use futures::stream::Stream;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use trios_a2a::{A2ARegistry, AgentId, Task, TaskPriority, TaskState, HEARTBEAT_TTL_MS};
use uuid::Uuid;

use crate::ws_handler::AppState;

/// Live SSE subscribers: agent id → sender of wire-message JSON values.
///
/// Delivery-side state only (I/O layer). The registry (SR-02) remains the
/// single source of truth for agents, liveness, tasks, and offline queues.
#[derive(Default)]
pub struct A2aHub {
    subscribers: Mutex<HashMap<String, mpsc::UnboundedSender<Value>>>,
}

impl A2aHub {
    /// Open a live channel for an agent, replacing any previous one.
    pub fn subscribe(&self, agent_id: &str) -> mpsc::UnboundedReceiver<Value> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.subscribers
            .lock()
            .unwrap()
            .insert(agent_id.to_string(), tx);
        rx
    }

    /// Drop an agent's live channel.
    pub fn unsubscribe(&self, agent_id: &str) {
        self.subscribers.lock().unwrap().remove(agent_id);
    }

    /// Try to deliver to a live subscriber. False when the agent has no
    /// (working) stream; dead channels are pruned on the way.
    pub fn deliver(&self, recipient: &str, msg: &Value) -> bool {
        let mut subs = self.subscribers.lock().unwrap();
        if let Some(tx) = subs.get(recipient) {
            if tx.send(msg.clone()).is_ok() {
                return true;
            }
            subs.remove(recipient);
        }
        false
    }

    /// Fan out to all live subscribers except the sender (TS broadcast
    /// parity: live-only, никогда не в очередь). Returns delivered count.
    pub fn broadcast(&self, sender: &str, msg: &Value) -> usize {
        let mut subs = self.subscribers.lock().unwrap();
        let mut dead = Vec::new();
        let mut delivered = 0;
        for (id, tx) in subs.iter() {
            if id == sender {
                continue;
            }
            if tx.send(msg.clone()).is_ok() {
                delivered += 1;
            } else {
                dead.push(id.clone());
            }
        }
        for id in dead {
            subs.remove(&id);
        }
        delivered
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Swift `Data` payloads are base64 on the wire. Strings pass through
/// verbatim; everything else becomes base64-encoded JSON.
fn normalize_payload_for_swift(payload: Option<&Value>) -> Value {
    match payload {
        None | Some(Value::Null) => Value::String(String::new()),
        Some(Value::String(s)) => Value::String(s.clone()),
        Some(other) => {
            let bytes = serde_json::to_vec(other).unwrap_or_default();
            Value::String(B64.encode(bytes))
        }
    }
}

/// Swift task-state raw values ↔ SR-01 `TaskState`.
fn task_state_from_wire(s: &str) -> Option<TaskState> {
    match s {
        "pending" => Some(TaskState::Pending),
        "assigned" => Some(TaskState::Assigned),
        "inProgress" => Some(TaskState::InProgress),
        "completed" => Some(TaskState::Completed),
        "failed" => Some(TaskState::Failed),
        "cancelled" => Some(TaskState::Cancelled),
        _ => None,
    }
}

fn task_state_to_wire(s: &TaskState) -> &'static str {
    match s {
        TaskState::Pending => "pending",
        TaskState::Assigned => "assigned",
        TaskState::InProgress => "inProgress",
        TaskState::Completed => "completed",
        TaskState::Failed => "failed",
        TaskState::Cancelled => "cancelled",
    }
}

fn priority_from_wire(n: i64) -> TaskPriority {
    match n {
        0 => TaskPriority::Low,
        2 => TaskPriority::High,
        3 => TaskPriority::Critical,
        _ => TaskPriority::Medium,
    }
}

fn priority_to_wire(p: &TaskPriority) -> i64 {
    match p {
        TaskPriority::Low => 0,
        TaskPriority::Medium => 1,
        TaskPriority::High => 2,
        TaskPriority::Critical => 3,
    }
}

/// SR-01 `Task` → Swift `AgentTask` wire shape (camelCase keys, raw-value
/// enums, integer priority).
fn task_to_wire(task: &Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "description": task.description,
        "state": task_state_to_wire(&task.state),
        "priority": priority_to_wire(&task.priority),
        "assignee": task.assigned_to.as_ref().map(|a| a.as_str()).unwrap_or(""),
        "createdAt": task.created_at,
        "updatedAt": task.updated_at,
    })
}

/// Build a system wire message (taskAssign / taskUpdate) with a
/// Swift-decodable base64 payload.
fn system_message(recipient: &str, msg_type: &str, payload: &Value) -> Value {
    json!({
        "id": Uuid::new_v4().to_string(),
        "sender": "system",
        "recipient": recipient,
        "type": msg_type,
        "payload": normalize_payload_for_swift(Some(payload)),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })
}

/// The `/a2a/*` REST + SSE router (merged into the main app router).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/a2a/register", post(register))
        .route("/a2a/unregister", post(unregister))
        .route("/a2a/heartbeat", post(heartbeat))
        .route("/a2a/agents", get(agents))
        .route("/a2a/matrix", get(matrix))
        .route("/a2a/message", post(message))
        .route("/a2a/task/assign", post(task_assign))
        .route("/a2a/task/update", post(task_update))
        .route("/a2a/stream", get(stream))
}

type ApiError = (StatusCode, Json<Value>);

fn bad_request(msg: &str) -> ApiError {
    (StatusCode::BAD_REQUEST, Json(json!({"error": msg})))
}

async fn with_registry<R>(state: &AppState, f: impl FnOnce(&mut A2ARegistry) -> R) -> R {
    let router = state.a2a.read().await;
    let shared = router.registry();
    let mut reg = shared.lock().unwrap();
    f(&mut reg)
}

async fn register(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let res = with_registry(&state, |reg| reg.register_wire(body, now_ms())).await;
    if res["ok"] == json!(true) {
        Ok(Json(json!({"success": true})))
    } else {
        Err(bad_request(
            res["error"].as_str().unwrap_or("Missing agent id or name"),
        ))
    }
}

async fn unregister(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agentId")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("Missing agentId"))?
        .to_string();
    let removed = with_registry(&state, |reg| reg.unregister_agent(&agent_id)).await;
    state.a2a_hub.unsubscribe(&agent_id);
    Ok(Json(json!({"success": removed})))
}

async fn heartbeat(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let agent_id = body
        .get("agentId")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("Missing agentId"))?
        .to_string();
    let ok = with_registry(&state, |reg| reg.heartbeat(&agent_id, now_ms())).await;
    Ok(Json(json!({"success": ok})))
}

/// Bare array — the shape the Swift client actually decodes.
async fn agents(State(state): State<AppState>) -> Json<Value> {
    let cards =
        with_registry(&state, |reg| reg.live_wire_cards(now_ms(), HEARTBEAT_TTL_MS)).await;
    Json(Value::Array(cards))
}

async fn matrix(State(state): State<AppState>) -> Json<Value> {
    // Memory-mode parity with the TS route: static canon + empty matrix.
    let _ = &state;
    Json(json!({
        "matrix": [],
        "canon": "IGLA-SHORT-WAVE-MATRIX-2026",
        "generated_at": chrono::Utc::now().to_rfc3339(),
    }))
}

async fn message(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let id = body.get("id").and_then(Value::as_str).unwrap_or_default();
    let sender = body.get("sender").and_then(Value::as_str).unwrap_or_default();
    let msg_type = body.get("type").and_then(Value::as_str).unwrap_or_default();
    if id.is_empty() || sender.is_empty() || msg_type.is_empty() {
        return Err(bad_request("Invalid message"));
    }
    let recipient = body
        .get("recipient")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let mut wire = body.clone();
    wire["payload"] = normalize_payload_for_swift(body.get("payload"));

    if recipient.is_empty() {
        // Broadcast: live subscribers only (TS parity).
        state.a2a_hub.broadcast(sender, &wire);
        return Ok(Json(json!({"success": true})));
    }

    if state.a2a_hub.deliver(&recipient, &wire) {
        return Ok(Json(json!({"success": true})));
    }
    // Offline: queue for the registered recipient, reject unknown ones (P2).
    let queued = with_registry(&state, |reg| reg.queue_pending(&recipient, wire)).await;
    if queued {
        Ok(Json(json!({"success": true})))
    } else {
        Err(bad_request(&format!(
            "recipient '{recipient}' is not a registered agent"
        )))
    }
}

async fn task_assign(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_json = body
        .get("task")
        .filter(|t| t.is_object())
        .ok_or_else(|| bad_request("Missing task or agentId"))?
        .clone();
    let agent_id = body
        .get("agentId")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("Missing task or agentId"))?
        .to_string();

    let now = chrono::Utc::now().to_rfc3339();
    let task = Task {
        id: task_json
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        title: task_json
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        description: task_json
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        assigned_to: Some(AgentId::new(&agent_id)),
        created_by: AgentId::new("system"),
        // TS parity: assignment forces the pending state.
        state: TaskState::Pending,
        priority: priority_from_wire(
            task_json.get("priority").and_then(Value::as_i64).unwrap_or(1),
        ),
        created_at: task_json
            .get("createdAt")
            .and_then(Value::as_str)
            .unwrap_or(&now)
            .to_string(),
        updated_at: now.clone(),
    };

    let res = with_registry(&state, |reg| reg.upsert_task(task)).await;
    if res["ok"] != json!(true) {
        return Err(bad_request(
            res["error"].as_str().unwrap_or("unknown assignee"),
        ));
    }

    // Notify the assignee: deliver live or queue (payload = original wire
    // task, so the Swift decoder sees exactly what the sender built).
    let msg = system_message(&agent_id, "taskAssign", &task_json);
    if !state.a2a_hub.deliver(&agent_id, &msg) {
        with_registry(&state, |reg| reg.queue_pending(&agent_id, msg)).await;
    }
    Ok(Json(json!({"success": true})))
}

async fn task_update(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let task_id = body
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("Missing id or state"))?
        .to_string();
    let state_raw = body
        .get("state")
        .and_then(Value::as_str)
        .ok_or_else(|| bad_request("Missing id or state"))?;
    let new_state = task_state_from_wire(state_raw)
        .ok_or_else(|| bad_request(&format!("unknown task state '{state_raw}'")))?;

    let updated = with_registry(&state, |reg| {
        let res = reg.update_task(&task_id, new_state);
        if res["ok"] == json!(true) {
            reg.get_task(&task_id).map(task_to_wire)
        } else {
            None
        }
    })
    .await;

    let Some(wire_task) = updated else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Task not found"})),
        ));
    };

    // TS parity: taskUpdate is delivered live-only (no offline queue).
    let assignee = wire_task["assignee"].as_str().unwrap_or_default().to_string();
    if !assignee.is_empty() {
        let msg = system_message(&assignee, "taskUpdate", &wire_task);
        state.a2a_hub.deliver(&assignee, &msg);
    }
    Ok(Json(json!({"success": true})))
}

/// SSE stream that unsubscribes its agent from the hub on disconnect.
struct SubscriberStream {
    rx: mpsc::UnboundedReceiver<Value>,
    /// Messages queued while the agent was offline, flushed first.
    backlog: std::vec::IntoIter<Value>,
    hub: Arc<A2aHub>,
    agent_id: String,
}

impl Stream for SubscriberStream {
    type Item = Result<Event, Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(msg) = self.backlog.next() {
            return Poll::Ready(Some(Ok(Event::default().data(msg.to_string()))));
        }
        match Pin::new(&mut self.rx).poll_recv(cx) {
            Poll::Ready(Some(msg)) => {
                Poll::Ready(Some(Ok(Event::default().data(msg.to_string()))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for SubscriberStream {
    fn drop(&mut self) {
        self.hub.unsubscribe(&self.agent_id);
    }
}

async fn stream(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Sse<SubscriberStream>, ApiError> {
    let agent_id = params
        .get("agentId")
        .filter(|v| !v.is_empty())
        .ok_or_else(|| bad_request("Missing agentId query parameter"))?
        .clone();

    let rx = state.a2a_hub.subscribe(&agent_id);
    let backlog = with_registry(&state, |reg| reg.drain_pending(&agent_id)).await;

    let stream = SubscriberStream {
        rx,
        backlog: backlog.into_iter(),
        hub: Arc::clone(&state.a2a_hub),
        agent_id,
    };
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    ))
}

/// Background watchdog: prune stale agents and their live channels every
/// minute (TS heartbeat-watchdog parity).
pub fn spawn_prune_loop(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await; // first tick is immediate — skip it
        loop {
            ticker.tick().await;
            let stale = {
                let router = state.a2a.read().await;
                let shared = router.registry();
                let mut reg = shared.lock().unwrap();
                reg.prune_stale(now_ms(), HEARTBEAT_TTL_MS)
            };
            for id in stale {
                tracing::warn!("A2A agent '{id}' pruned after missed heartbeats");
                state.a2a_hub.unsubscribe(&id);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn app() -> (Router, AppState) {
        let state = AppState::new();
        let router = router().with_state(state.clone());
        (router, state)
    }

    async fn post_json(router: &Router, path: &str, body: Value) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
    }

    async fn get_json(router: &Router, path: &str) -> (StatusCode, Value) {
        let res = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
    }

    fn card(id: &str) -> Value {
        json!({
            "id": id,
            "name": format!("Agent {id}"),
            "description": "t",
            "capabilities": ["chat"],
            "version": "1.0.0"
        })
    }

    #[tokio::test]
    async fn register_then_agents_returns_bare_array() {
        let (router, _) = app();
        let (st, body) = post_json(&router, "/a2a/register", card("swift-1")).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["success"], true);

        let (st, body) = get_json(&router, "/a2a/agents").await;
        assert_eq!(st, StatusCode::OK);
        // Swift decodes [AgentCard].self — the body must be a bare array.
        let arr = body.as_array().expect("bare array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "swift-1");
    }

    #[tokio::test]
    async fn register_rejects_incomplete_card() {
        let (router, _) = app();
        let (st, _) = post_json(&router, "/a2a/register", json!({"name": "x"})).await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn heartbeat_unknown_agent_reports_failure() {
        let (router, _) = app();
        let (st, body) =
            post_json(&router, "/a2a/heartbeat", json!({"agentId": "ghost"})).await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["success"], false);
    }

    #[tokio::test]
    async fn unregister_removes_agent() {
        let (router, _) = app();
        post_json(&router, "/a2a/register", card("a")).await;
        let (_, body) = post_json(&router, "/a2a/unregister", json!({"agentId": "a"})).await;
        assert_eq!(body["success"], true);
        let (_, body) = get_json(&router, "/a2a/agents").await;
        assert!(body.as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn message_to_unregistered_recipient_is_rejected() {
        let (router, _) = app();
        let (st, body) = post_json(
            &router,
            "/a2a/message",
            json!({"id": "m1", "sender": "a", "type": "direct", "recipient": "ghost", "payload": "aGk=", "timestamp": "t"}),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("ghost"));
    }

    #[tokio::test]
    async fn offline_message_is_queued_and_drained_by_stream_backlog() {
        let (router, state) = app();
        post_json(&router, "/a2a/register", card("rcpt")).await;
        let (st, _) = post_json(
            &router,
            "/a2a/message",
            json!({"id": "m1", "sender": "a", "type": "direct", "recipient": "rcpt", "payload": "aGk=", "timestamp": "t"}),
        )
        .await;
        assert_eq!(st, StatusCode::OK);

        let backlog = {
            let router_guard = state.a2a.read().await;
            let shared = router_guard.registry();
            let mut reg = shared.lock().unwrap();
            reg.drain_pending("rcpt")
        };
        assert_eq!(backlog.len(), 1);
        assert_eq!(backlog[0]["payload"], "aGk=");
    }

    #[tokio::test]
    async fn live_subscriber_gets_message_and_broadcast() {
        let (router, state) = app();
        post_json(&router, "/a2a/register", card("live")).await;
        let mut rx = state.a2a_hub.subscribe("live");

        // direct
        post_json(
            &router,
            "/a2a/message",
            json!({"id": "m1", "sender": "x", "type": "direct", "recipient": "live", "payload": {"k": 1}, "timestamp": "t"}),
        )
        .await;
        let got = rx.recv().await.unwrap();
        // object payload → base64 JSON for the Swift Data decoder
        let decoded = B64.decode(got["payload"].as_str().unwrap()).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&decoded).unwrap(), json!({"k": 1}));

        // broadcast (no recipient) reaches the subscriber too
        post_json(
            &router,
            "/a2a/message",
            json!({"id": "m2", "sender": "x", "type": "broadcast", "payload": "aGk=", "timestamp": "t"}),
        )
        .await;
        assert_eq!(rx.recv().await.unwrap()["type"], "broadcast");
    }

    #[tokio::test]
    async fn task_assign_queues_swift_decodable_task() {
        let (router, state) = app();
        post_json(&router, "/a2a/register", card("worker")).await;

        let task = json!({
            "id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
            "title": "wire",
            "description": "d",
            "state": "pending",
            "priority": 2,
            "assignee": "worker",
            "createdAt": "2026-07-25T00:00:00Z",
            "updatedAt": "2026-07-25T00:00:00Z"
        });
        let (st, _) = post_json(
            &router,
            "/a2a/task/assign",
            json!({"task": task, "agentId": "worker"}),
        )
        .await;
        assert_eq!(st, StatusCode::OK);

        let backlog = {
            let router_guard = state.a2a.read().await;
            let shared = router_guard.registry();
            let mut reg = shared.lock().unwrap();
            reg.drain_pending("worker")
        };
        assert_eq!(backlog.len(), 1);
        assert_eq!(backlog[0]["type"], "taskAssign");
        // payload is base64 of the ORIGINAL wire task
        let decoded = B64
            .decode(backlog[0]["payload"].as_str().unwrap())
            .unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&decoded).unwrap(), task);
    }

    #[tokio::test]
    async fn task_assign_to_unknown_agent_is_rejected() {
        let (router, _) = app();
        let (st, _) = post_json(
            &router,
            "/a2a/task/assign",
            json!({"task": {"id": "t1", "title": "x"}, "agentId": "ghost"}),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn task_update_maps_swift_state_and_notifies_live_assignee() {
        let (router, state) = app();
        post_json(&router, "/a2a/register", card("worker")).await;
        post_json(
            &router,
            "/a2a/task/assign",
            json!({"task": {"id": "t-1", "title": "x", "priority": 1}, "agentId": "worker"}),
        )
        .await;
        // drop the queued taskAssign, then go live
        {
            let router_guard = state.a2a.read().await;
            let shared = router_guard.registry();
            shared.lock().unwrap().drain_pending("worker");
        }
        let mut rx = state.a2a_hub.subscribe("worker");

        let (st, _) = post_json(
            &router,
            "/a2a/task/update",
            json!({"id": "t-1", "state": "inProgress"}),
        )
        .await;
        assert_eq!(st, StatusCode::OK);

        let msg = rx.recv().await.unwrap();
        assert_eq!(msg["type"], "taskUpdate");
        let decoded = B64.decode(msg["payload"].as_str().unwrap()).unwrap();
        let task: Value = serde_json::from_slice(&decoded).unwrap();
        // Swift raw value, camelCase keys, integer priority
        assert_eq!(task["state"], "inProgress");
        assert_eq!(task["priority"], 1);
        assert_eq!(task["assignee"], "worker");
        assert!(task.get("createdAt").is_some());
    }

    #[tokio::test]
    async fn task_update_unknown_task_is_404() {
        let (router, _) = app();
        let (st, _) = post_json(
            &router,
            "/a2a/task/update",
            json!({"id": "nope", "state": "completed"}),
        )
        .await;
        assert_eq!(st, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn stream_requires_agent_id() {
        let (router, _) = app();
        let (st, _) = get_json(&router, "/a2a/stream").await;
        assert_eq!(st, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn matrix_has_canon_shape() {
        let (router, _) = app();
        let (st, body) = get_json(&router, "/a2a/matrix").await;
        assert_eq!(st, StatusCode::OK);
        assert_eq!(body["canon"], "IGLA-SHORT-WAVE-MATRIX-2026");
        assert!(body["matrix"].as_array().unwrap().is_empty());
    }

    #[test]
    fn payload_normalization_matrix() {
        assert_eq!(normalize_payload_for_swift(None), json!(""));
        assert_eq!(normalize_payload_for_swift(Some(&Value::Null)), json!(""));
        assert_eq!(
            normalize_payload_for_swift(Some(&json!("aGk="))),
            json!("aGk=")
        );
        let b64 = normalize_payload_for_swift(Some(&json!({"a": 1})));
        let decoded = B64.decode(b64.as_str().unwrap()).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&decoded).unwrap(), json!({"a": 1}));
    }

    #[test]
    fn state_and_priority_roundtrip() {
        for (wire, variant) in [
            ("pending", TaskState::Pending),
            ("assigned", TaskState::Assigned),
            ("inProgress", TaskState::InProgress),
            ("completed", TaskState::Completed),
            ("failed", TaskState::Failed),
            ("cancelled", TaskState::Cancelled),
        ] {
            assert_eq!(task_state_from_wire(wire), Some(variant.clone()));
            assert_eq!(task_state_to_wire(&variant), wire);
        }
        assert_eq!(task_state_from_wire("in_progress"), None);
        for n in 0..4 {
            assert_eq!(priority_to_wire(&priority_from_wire(n)), n);
        }
    }
}
