//! A2A MCP endpoint — bridges trios-server WS routing to trios-a2a registry

use serde_json::{json, Value};
use trios_a2a::{
    AgentCard, AgentId, Capability,
    A2AMessage, A2AMessageType,
    TaskPriority,
    A2ARouter,
};
use crate::ws_handler::AppState;

/// a2a/list_agents — list all registered A2A agents
pub async fn list_agents(state: &AppState) -> Value {
    let registry = state.a2a.read().await;
    let agents = registry.list_agents();
    let result: Vec<Value> = agents.iter().map(|card| {
        json!({
            "id": card.id.0,
            "name": card.name,
            "description": card.description,
            "capabilities": card.capabilities.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            "status": format!("{:?}", card.status),
        })
    }).collect();
    json!(result)
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

    let capabilities: Vec<Capability> = params
        .get("capabilities")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).map(|s| Capability {
            name: s.to_string(),
            description: None,
        }).collect())
        .unwrap_or_default();

    let card = AgentCard {
        id: AgentId(id.to_string()),
        name: name.to_string(),
        description: description.to_string(),
        capabilities,
        status: trios_a2a::AgentStatus::Idle,
        endpoint: params.get("endpoint").and_then(|v| v.as_str()).map(String::from),
    };

    let mut registry = state.a2a.write().await;
    registry.register(card);
    json!({"ok": true, "id": id})
}

/// a2a/send — send direct message to agent
pub async fn send(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("system");
    let to = params.get("to").and_then(|v| v.as_str()).unwrap_or("");
    let content = params.get("content").cloned().unwrap_or(json!({}));

    if to.is_empty() {
        return json!({"error": "to is required"});
    }

    let msg = A2AMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from: AgentId(from.to_string()),
        to: Some(AgentId(to.to_string())),
        message_type: A2AMessageType::Direct,
        content,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let registry = state.a2a.read().await;
    match registry.send_message(msg) {
        Ok(msg_id) => json!({"ok": true, "message_id": msg_id}),
        Err(e) => json!({"error": e}),
    }
}

/// a2a/broadcast — broadcast message to all agents
pub async fn broadcast(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("system");
    let content = params.get("content").cloned().unwrap_or(json!({}));

    let msg = A2AMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from: AgentId(from.to_string()),
        to: None,
        message_type: A2AMessageType::Broadcast,
        content,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
    };

    let registry = state.a2a.read().await;
    let count = registry.broadcast(msg);
    json!({"ok": true, "delivered_to": count})
}

/// a2a/assign_task — assign task to agent via A2A protocol
pub async fn assign_task(state: &AppState, params: Option<Value>) -> Value {
    let params = params.unwrap_or(json!({}));
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let description = params.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let priority_str = params.get("priority").and_then(|v| v.as_str()).unwrap_or("medium");

    if agent_id.is_empty() || description.is_empty() {
        return json!({"error": "agent_id and description are required"});
    }

    let priority = match priority_str {
        "low" => TaskPriority::Low,
        "high" => TaskPriority::High,
        "critical" => TaskPriority::Critical,
        _ => TaskPriority::Medium,
    };

    let task = trios_a2a::Task {
        id: uuid::Uuid::new_v4().to_string(),
        agent_id: AgentId(agent_id.to_string()),
        description: description.to_string(),
        priority,
        state: trios_a2a::TaskState::Pending,
        context: params.get("context").cloned(),
        created_at: chrono::Utc::now().timestamp_millis() as u64,
        updated_at: chrono::Utc::now().timestamp_millis() as u64,
    };

    let task_id = task.id.clone();
    let mut registry = state.a2a.write().await;
    registry.assign_task(task);
    json!({"ok": true, "task_id": task_id, "agent_id": agent_id, "priority": priority_str})
}
