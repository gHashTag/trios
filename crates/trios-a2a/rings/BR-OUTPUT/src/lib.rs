//! BR-OUTPUT — A2A Router + Registry Assembly
//!
//! Assembles all rings into a unified A2A service.
//! Provides the `A2ARouter` that dispatches MCP tool calls to the registry.

use trios_a2a_sr00::AgentCard;
use trios_a2a_sr01::TaskState;
use trios_a2a_sr02::{A2ARegistry, SharedRegistry, shared_registry};
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// A2A Router — dispatches MCP tool calls to the registry.
pub struct A2ARouter {
    registry: SharedRegistry,
}

impl A2ARouter {
    /// Create a new router with an empty registry.
    pub fn new() -> Self {
        Self {
            registry: shared_registry(),
        }
    }

    /// Get a clone of the shared registry for use in other contexts.
    pub fn registry(&self) -> SharedRegistry {
        Arc::clone(&self.registry)
    }

    /// Dispatch an MCP tool call by name.
    pub fn call(&self, tool: &str, params: Value) -> Value {
        let mut reg = self.registry.lock().unwrap();
        match tool {
            "a2a_list_agents" => reg.list_agents(),
            "a2a_send" => {
                let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let to = params.get("to").and_then(|v| v.as_str()).unwrap_or("");
                let payload = params.get("payload").cloned().unwrap_or(Value::Null);
                reg.send_message(from, to, payload)
            }
            "a2a_broadcast" => {
                let from = params.get("from").and_then(|v| v.as_str()).unwrap_or("");
                let payload = params.get("payload").cloned().unwrap_or(Value::Null);
                reg.broadcast(from, payload)
            }
            "a2a_assign_task" => {
                let title = params.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let created_by = params.get("created_by").and_then(|v| v.as_str()).unwrap_or("");
                let assign_to = params.get("assign_to").and_then(|v| v.as_str()).unwrap_or("");
                reg.assign_task(title, created_by, assign_to)
            }
            "a2a_task_status" => {
                let task_id = params.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                reg.task_status(task_id)
            }
            "a2a_update_task" => {
                let task_id = params.get("task_id").and_then(|v| v.as_str()).unwrap_or("");
                let state_str = params.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let state = match state_str {
                    "pending" => TaskState::Pending,
                    "assigned" => TaskState::Assigned,
                    "in_progress" => TaskState::InProgress,
                    "completed" => TaskState::Completed,
                    "failed" => TaskState::Failed,
                    "cancelled" => TaskState::Cancelled,
                    _ => return serde_json::json!({"error": format!("unknown state: {}", state_str)}),
                };
                reg.update_task(task_id, state)
            }
            _ => serde_json::json!({"error": format!("unknown tool: {}", tool)}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_router_register_and_list() {
        let router = A2ARouter::new();
        // Register via direct registry access
        {
            let mut reg = router.registry.lock().unwrap();
            reg.register_agent(AgentCard::new("alpha", "Alpha"));
        }
        let result = router.call("a2a_list_agents", json!({}));
        let agents = result.as_array().unwrap();
        assert_eq!(agents.len(), 1);
    }

    #[test]
    fn test_router_assign_and_status() {
        let router = A2ARouter::new();
        let result = router.call("a2a_assign_task", json!({
            "title": "Test task",
            "created_by": "lead",
            "assign_to": "alpha"
        }));
        assert_eq!(result["ok"], true);
        let task_id = result["task_id"].as_str().unwrap();

        let status = router.call("a2a_task_status", json!({"task_id": task_id}));
        assert_eq!(status["state"], "assigned");

        let update = router.call("a2a_update_task", json!({
            "task_id": task_id,
            "state": "completed"
        }));
        assert_eq!(update["ok"], true);
    }

    #[test]
    fn test_router_unknown_tool() {
        let router = A2ARouter::new();
        let result = router.call("unknown_tool", json!({}));
        assert!(result.get("error").is_some());
    }
}
