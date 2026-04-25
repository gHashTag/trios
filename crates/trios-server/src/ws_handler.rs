use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::{broadcast, Mutex, RwLock};
use tracing::{error, info};

use crate::mcp::McpService;
use crate::mcp_endpoints;
use trios_a2a::A2ARouter;

/// Event types broadcasted to all connected clients
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum BusEvent {
    TaskAssigned { task_id: String, agent_id: String },
    TaskUpdated { task_id: String, status: String },
    AgentConnected { agent_id: String },
    AgentDisconnected { agent_id: String },
    A2AMessage { from: String, to: Option<String>, content: Value },
}

#[derive(Clone)]
pub struct AppState {
    pub mcp: McpService,
    pub agents: Arc<Mutex<Vec<AgentState>>>,
    pub tasks: Arc<Mutex<Vec<TaskEntry>>>,
    /// Broadcast channel for event streaming to all connected clients
    pub event_tx: broadcast::Sender<BusEvent>,
    /// A2A router for agent-to-agent communication
    pub a2a: Arc<RwLock<A2ARouter>>,
    /// z.ai API endpoint (Anthropic-compatible)
    pub zai_api: String,
    /// z.ai API keys (rotated round-robin)
    pub zai_keys: Vec<String>,
    /// HTTP client for outbound requests
    pub http_client: reqwest::Client,
    /// Round-robin counter for key rotation
    pub zai_key_idx: Arc<AtomicUsize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: String,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub id: String,
    pub agent_id: String,
    pub task: String,
    pub status: String,
}

impl AppState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);

        // Load z.ai credentials from environment
        let zai_api = std::env::var("ZAI_API").unwrap_or_default();
        let mut zai_keys = Vec::new();
        for i in 1..=6 {
            if let Ok(key) = std::env::var(format!("ZAI_KEY_{}", i)) {
                if !key.is_empty() {
                    zai_keys.push(key);
                }
            }
        }

        info!(".env loaded: zai_api={} keys={}",
            if zai_api.is_empty() { "(empty)" } else { &zai_api },
            zai_keys.len());

        Self {
            mcp: McpService::new(),
            agents: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(Vec::new())),
            event_tx: tx,
            a2a: Arc::new(RwLock::new(A2ARouter::new())),
            zai_api,
            zai_keys,
            http_client: reqwest::Client::new(),
            zai_key_idx: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Pick next key via round-robin
    pub fn next_zai_key(&self) -> Option<&str> {
        if self.zai_keys.is_empty() { return None; }
        let idx = self.zai_key_idx.fetch_add(1, Ordering::Relaxed) % self.zai_keys.len();
        Some(&self.zai_keys[idx])
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast_event(&self, event: BusEvent) {
        let _ = self.event_tx.send(event);
    }
}

#[derive(Debug, Deserialize)]
struct WsRequest {
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct WsResponse {
    pub result: Value,
}

pub async fn ws_handler(ws: WebSocketUpgrade, axum::extract::State(state): axum::extract::State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    info!("WebSocket client connected");
    let mut event_rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let response = handle_message(&text, &state).await;
                        let response_json = serde_json::to_string(&response).unwrap_or_else(|e| {
                            serde_json::to_string(&WsResponse {
                                result: json!({"error": format!("serialize error: {}", e)}),
                            }).unwrap_or_default()
                        });
                        if socket.send(Message::Text(response_json)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None | Some(Ok(_)) => {}
                }
            }
            event_result = event_rx.recv() => {
                match event_result {
                    Ok(event) => {
                        let event_json = serde_json::to_string(&json!({"event": event})).unwrap_or_default();
                        if socket.send(Message::Text(event_json)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        info!("Event channel lagged, skipped {} events", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("Event channel closed");
                        break;
                    }
                }
            }
        }
    }
}

pub async fn handle_message(text: &str, state: &AppState) -> WsResponse {
    let req: WsRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => {
            return WsResponse {
                result: json!({"error": format!("invalid request: {}", e)}),
            }
        }
    };

    info!("WS request: method={}", req.method);

    let result = match req.method.as_str() {
        "initialize" => json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": false }
            },
            "serverInfo": {
                "name": "trios-server",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        "notifications/initialized" => json!({}),
        "ping" => json!({"status": "ok"}),
        "agents/list"         => mcp_endpoints::agents::list(state).await,
        "agents/chat"         => mcp_endpoints::agents::chat(state, req.params).await,
        "tasks/assign"        => mcp_endpoints::tasks::assign(state, req.params).await,
        "tasks/status"        => mcp_endpoints::tasks::status(state, req.params).await,
        "tasks/update_status" => mcp_endpoints::tasks::update_status(state, req.params).await,
        "experience/read"     => mcp_endpoints::experience::read(state, req.params).await,
        "tools/list"          => tools_list(state).await,
        "tools/call"          => tools_call(state, req.params).await,
        "a2a/list_agents"     => mcp_endpoints::a2a::list_agents(state).await,
        "a2a/register"        => mcp_endpoints::a2a::register(state, req.params).await,
        "a2a/send"            => mcp_endpoints::a2a::send(state, req.params).await,
        "a2a/broadcast"       => mcp_endpoints::a2a::broadcast(state, req.params).await,
        "a2a/assign_task"     => mcp_endpoints::a2a::assign_task(state, req.params).await,
        "a2a/task_status"     => mcp_endpoints::a2a::task_status(state, req.params).await,
        "a2a/update_task"     => mcp_endpoints::a2a::update_task(state, req.params).await,
        _ => json!({"error": format!("unknown method: {}", req.method)}),
    };

    WsResponse { result }
}

async fn tools_list(state: &AppState) -> Value {
    let list = state.mcp.list_tools();
    serde_json::to_value(list.tools).unwrap_or(json!({"error": "serialize failed"}))
}

async fn tools_call(state: &AppState, params: Option<Value>) -> Value {
    let params_val = params.unwrap_or(json!({}));
    let tool_name = params_val.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params_val.get("arguments").cloned().unwrap_or(json!({}));

    let a2a_result = match tool_name {
        "a2a_register" => Some(mcp_endpoints::a2a::register(state, Some(arguments)).await),
        "a2a_list_agents" => Some(mcp_endpoints::a2a::list_agents(state).await),
        "a2a_send" => Some(mcp_endpoints::a2a::send(state, Some(arguments)).await),
        "a2a_broadcast" => Some(mcp_endpoints::a2a::broadcast(state, Some(arguments)).await),
        "a2a_assign_task" => Some(mcp_endpoints::a2a::assign_task(state, Some(arguments)).await),
        "a2a_task_status" => Some(mcp_endpoints::a2a::task_status(state, Some(arguments)).await),
        "a2a_update_task" => Some(mcp_endpoints::a2a::update_task(state, Some(arguments)).await),
        _ => None,
    };

    if let Some(result) = a2a_result {
        return json!({
            "content": [{"type": "text", "text": serde_json::to_string(&result).unwrap_or_default()}]
        });
    }

    let arguments_obj = params_val.get("arguments").cloned();
    use rust_mcp_schema::CallToolRequestParams;
    let call_params = CallToolRequestParams {
        name: tool_name.to_string(),
        arguments: arguments_obj.and_then(|a| {
            if a.is_object() {
                Some(a.as_object().cloned().unwrap_or_default())
            } else {
                None
            }
        }),
        meta: None,
        task: None,
    };

    let result = state.mcp.call_tool(call_params).await;
    serde_json::to_value(result).unwrap_or(json!({"error": "serialize failed"}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agents_list_empty() {
        let state = AppState::new();
        let result = mcp_endpoints::agents::list(&state).await;
        assert_eq!(result, json!([]));
    }

    #[tokio::test]
    async fn test_tasks_assign() {
        let state = AppState::new();
        let params = json!({"agent_id": "agent-1", "task": "test task", "context": "test"});
        let result = mcp_endpoints::tasks::assign(&state, Some(params)).await;
        assert!(result.get("task_id").is_some());
    }

    #[tokio::test]
    async fn test_tasks_status_not_found() {
        let state = AppState::new();
        let params = json!({"task_id": "nonexistent"});
        let result = mcp_endpoints::tasks::status(&state, Some(params)).await;
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_experience_read() {
        let state = AppState::new();
        let result = mcp_endpoints::experience::read(&state, None).await;
        assert!(result.is_array());
    }

    #[tokio::test]
    async fn test_tools_list_returns_array() {
        let state = AppState::new();
        let result = tools_list(&state).await;
        assert!(result.is_array());
    }

    #[tokio::test]
    async fn test_handle_unknown_method() {
        let state = AppState::new();
        let response = handle_message(r#"{"method":"unknown","params":{}}"#, &state).await;
        assert!(response.result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_handle_invalid_json() {
        let state = AppState::new();
        let response = handle_message("not json", &state).await;
        assert!(response.result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_broadcast_event() {
        let state = AppState::new();
        let mut rx = state.event_tx.subscribe();
        let event = BusEvent::AgentConnected { agent_id: "test-agent".to_string() };
        state.broadcast_event(event.clone());
        let received = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_a2a_register_and_list() {
        let state = AppState::new();
        let params = json!({
            "id": "agent-trinity",
            "name": "Trinity",
            "description": "Core orchestrator",
            "capabilities": ["code", "review"]
        });
        let result = mcp_endpoints::a2a::register(&state, Some(params)).await;
        assert_eq!(result["ok"], true);

        let agents = mcp_endpoints::a2a::list_agents(&state).await;
        assert_eq!(agents.as_array().unwrap().len(), 1);
        assert_eq!(agents[0]["id"], "agent-trinity");
    }

    #[tokio::test]
    async fn test_a2a_assign_task() {
        let state = AppState::new();
        let reg_params = json!({"id": "worker-1", "name": "Worker"});
        mcp_endpoints::a2a::register(&state, Some(reg_params)).await;

        let params = json!({
            "agent_id": "worker-1",
            "description": "Implement SR-02 ring",
            "priority": "high"
        });
        let result = mcp_endpoints::a2a::assign_task(&state, Some(params)).await;
        assert_eq!(result["ok"], true);
        assert!(result.get("task_id").is_some());
    }

    #[tokio::test]
    async fn test_a2a_broadcast() {
        let state = AppState::new();
        for i in 0..2 {
            let p = json!({"id": format!("agent-{}", i), "name": format!("Agent {}", i)});
            mcp_endpoints::a2a::register(&state, Some(p)).await;
        }
        let params = json!({"from": "system", "payload": {"msg": "hello all"}});
        let result = mcp_endpoints::a2a::broadcast(&state, Some(params)).await;
        assert_eq!(result["ok"], true);
        assert_eq!(result["recipients"], 2);
    }

    #[tokio::test]
    async fn test_round_robin_keys() {
        let state = AppState::new();
        if state.zai_keys.len() >= 2 {
            let k0 = state.next_zai_key().map(|s| s.to_string());
            let k1 = state.next_zai_key().map(|s| s.to_string());
            assert_ne!(k0, k1);
        }
    }
}
