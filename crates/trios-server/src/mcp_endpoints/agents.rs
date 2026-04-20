use crate::ws_handler::AppState;
use serde_json::{json, Value};

pub async fn list(state: &AppState) -> Value {
    let agents = state.agents.lock().await;
    serde_json::to_value(&*agents).unwrap_or(json!([]))
}

pub async fn chat(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");

    if agent_id.is_empty() {
        return json!({"error": "agent_id is required"});
    }

    let agents = state.agents.lock().await;
    let exists = agents.iter().any(|a| a.id == agent_id);
    drop(agents);

    if !exists {
        return json!({"error": format!("agent {} not found", agent_id)});
    }

    json!({"response": format!("echo: {}", message)})
}
