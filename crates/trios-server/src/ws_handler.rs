use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::mcp::McpService;
use crate::mcp_endpoints;

#[derive(Clone)]
pub struct AppState {
    pub mcp: McpService,
    pub agents: Arc<Mutex<Vec<AgentState>>>,
    pub tasks: Arc<Mutex<Vec<TaskEntry>>>,
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
        Self {
            mcp: McpService::new(),
            agents: Arc::new(Mutex::new(Vec::new())),
            tasks: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WsRequest {
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct WsResponse {
    result: Value,
}

pub async fn ws_handler(ws: WebSocketUpgrade, axum::extract::State(state): axum::extract::State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    info!("WebSocket client connected");

    while let Some(msg) = socket.next().await {
        match msg {
            Ok(Message::Text(text)) => {
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
            Ok(Message::Close(_)) => {
                info!("WebSocket client disconnected");
                break;
            }
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

async fn handle_message(text: &str, state: &AppState) -> WsResponse {
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
        "agents/list" => mcp_endpoints::agents::list(state).await,
        "agents/chat" => mcp_endpoints::agents::chat(state, req.params).await,
        "tasks/assign" => mcp_endpoints::tasks::assign(state, req.params).await,
        "tasks/status" => mcp_endpoints::tasks::status(state, req.params).await,
        "experience/read" => mcp_endpoints::experience::read(state, req.params).await,
        "tools/list" => tools_list(state).await,
        "tools/call" => tools_call(state, req.params).await,
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
    let arguments = params_val.get("arguments").cloned();

    use rust_mcp_schema::CallToolRequestParams;
    let call_params = CallToolRequestParams {
        name: tool_name.to_string(),
        arguments: arguments.and_then(|a| {
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
}
