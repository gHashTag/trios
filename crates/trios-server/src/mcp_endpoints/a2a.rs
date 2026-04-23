//! A2A MCP endpoint — bridges trios-server WS routing to trios-a2a registry

use serde_json::{json, Value};
use crate::ws_handler::AppState;

/// a2a/list_agents — list all registered A2A agents
pub async fn list_agents(state: &AppState) -> Value {
    state.a2a.read().await.call("a2a_list_agents", json!({}))
}

/// a2a/register — register a new A2A agent
pub async fn register(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("");

    if id.is_empty() || name.is_empty() {
        return json!({"error": "id and name are required"});
    }

    // Parse capabilities from params
    let capabilities: Vec<String> = params
        .get("capabilities")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(String::from).collect())
        .unwrap_or_default();

    let router = state.a2a.read().await;
    let shared = router.registry();
    let mut reg = shared.lock().unwrap();
    let mut card = trios_a2a::AgentCard::new(id, name);
    card.description = description.to_string();
    // Map string capabilities to enum variants
    for c in &capabilities {
        let cap = match c.as_str() {
            "codegen" => trios_a2a::Capability::Codegen,
            "filesystem" => trios_a2a::Capability::FileSystem,
            "git" => trios_a2a::Capability::Git,
            "shell" => trios_a2a::Capability::Shell,
            "llm" => trios_a2a::Capability::LLM,
            "orchestrator" => trios_a2a::Capability::Orchestrator,
            other => trios_a2a::Capability::Custom(other.to_string()),
        };
        card = card.with_capability(cap);
    }
    reg.register_agent(card);
    json!({"ok": true, "id": id})
}

/// a2a/send — send direct message to agent
pub async fn send(state: &AppState, params: Option<Value>) -> Value {
    state.a2a.read().await.call("a2a_send", params.unwrap_or(json!({})))
}

/// a2a/broadcast — broadcast message to all agents
pub async fn broadcast(state: &AppState, params: Option<Value>) -> Value {
    state.a2a.read().await.call("a2a_broadcast", params.unwrap_or(json!({})))
}

/// a2a/assign_task — assign task to agent via A2A protocol
pub async fn assign_task(state: &AppState, params: Option<Value>) -> Value {
    state.a2a.read().await.call("a2a_assign_task", params.unwrap_or(json!({})))
}

/// a2a/task_status — get task status
pub async fn task_status(state: &AppState, params: Option<Value>) -> Value {
    state.a2a.read().await.call("a2a_task_status", params.unwrap_or(json!({})))
}

/// a2a/update_task — update task state
pub async fn update_task(state: &AppState, params: Option<Value>) -> Value {
    state.a2a.read().await.call("a2a_update_task", params.unwrap_or(json!({})))
}
