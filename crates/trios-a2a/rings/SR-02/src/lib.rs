//! SR-02 — A2A MCP Tools
//!
//! MCP-compatible tool definitions for A2A operations.
//! These tools can be registered with trios-server's MCP service.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use trios_a2a_sr00::{AgentCard, AgentId};
use trios_a2a_sr01::{A2AMessage, Task, TaskState};

/// A2A registry — holds agents and tasks.
#[derive(Debug, Clone)]
pub struct A2ARegistry {
    pub agents: HashMap<String, AgentCard>,
    pub tasks: HashMap<String, Task>,
    pub messages: Vec<A2AMessage>,
}

impl A2ARegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            tasks: HashMap::new(),
            messages: Vec::new(),
        }
    }

    /// Register an agent.
    pub fn register_agent(&mut self, card: AgentCard) -> Value {
        let id = card.id.to_string();
        self.agents.insert(id.clone(), card);
        json!({"ok": true, "agent_id": id})
    }

    /// List all registered agents.
    pub fn list_agents(&self) -> Value {
        let agents: Vec<&AgentCard> = self.agents.values().collect();
        serde_json::to_value(agents).unwrap_or(json!([]))
    }

    /// Send a direct message from one agent to another.
    pub fn send_message(&mut self, from: &str, to: &str, payload: Value) -> Value {
        let msg = A2AMessage::direct(AgentId::new(from), AgentId::new(to), payload);
        let result = serde_json::to_value(&msg).unwrap_or(json!({}));
        self.messages.push(msg);
        json!({"ok": true, "message_id": result["id"]})
    }

    /// Broadcast a message to all agents.
    pub fn broadcast(&mut self, from: &str, payload: Value) -> Value {
        let msg = A2AMessage::broadcast(AgentId::new(from), payload);
        let result = serde_json::to_value(&msg).unwrap_or(json!({}));
        self.messages.push(msg);
        json!({"ok": true, "message_id": result["id"], "recipients": self.agents.len()})
    }

    /// Assign a task to an agent.
    pub fn assign_task(&mut self, title: &str, created_by: &str, assign_to: &str) -> Value {
        let task = Task::new(title, AgentId::new(created_by))
            .assign_to(AgentId::new(assign_to));
        let task_id = task.id.clone();
        self.tasks.insert(task_id.clone(), task);
        json!({"ok": true, "task_id": task_id})
    }

    /// Get task status.
    pub fn task_status(&self, task_id: &str) -> Value {
        match self.tasks.get(task_id) {
            Some(task) => serde_json::to_value(task).unwrap_or(json!({"error": "serialize failed"})),
            None => json!({"error": format!("task {} not found", task_id)}),
        }
    }

    /// Update task state.
    pub fn update_task(&mut self, task_id: &str, new_state: TaskState) -> Value {
        match self.tasks.get_mut(task_id) {
            Some(task) => {
                task.state = new_state;
                task.updated_at = chrono::Utc::now().to_rfc3339();
                json!({"ok": true, "task_id": task_id, "state": serde_json::to_value(&task.state).unwrap()})
            }
            None => json!({"error": format!("task {} not found", task_id)}),
        }
    }
}

/// Thread-safe wrapper for A2A registry.
pub type SharedRegistry = Arc<Mutex<A2ARegistry>>;

/// Create a new shared registry.
pub fn shared_registry() -> SharedRegistry {
    Arc::new(Mutex::new(A2ARegistry::new()))
}

/// MCP tool definitions for A2A.
pub fn mcp_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "a2a_list_agents",
            "description": "List all registered A2A agents",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "a2a_send",
            "description": "Send a direct A2A message to another agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string"},
                    "to": {"type": "string"},
                    "payload": {"type": "object"}
                },
                "required": ["from", "to", "payload"]
            }
        }),
        json!({
            "name": "a2a_broadcast",
            "description": "Broadcast a message to all A2A agents",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": {"type": "string"},
                    "payload": {"type": "object"}
                },
                "required": ["from", "payload"]
            }
        }),
        json!({
            "name": "a2a_assign_task",
            "description": "Assign a task to an A2A agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string"},
                    "created_by": {"type": "string"},
                    "assign_to": {"type": "string"}
                },
                "required": ["title", "created_by", "assign_to"]
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_agent() {
        let mut reg = A2ARegistry::new();
        let card = AgentCard::new("alpha-1", "Alpha");
        let result = reg.register_agent(card);
        assert_eq!(result["ok"], true);
        assert_eq!(reg.agents.len(), 1);
    }

    #[test]
    fn test_send_message() {
        let mut reg = A2ARegistry::new();
        reg.register_agent(AgentCard::new("alpha", "Alpha"));
        reg.register_agent(AgentCard::new("beta", "Beta"));
        let result = reg.send_message("alpha", "beta", json!({"text": "hello"}));
        assert_eq!(result["ok"], true);
        assert_eq!(reg.messages.len(), 1);
    }

    #[test]
    fn test_assign_task() {
        let mut reg = A2ARegistry::new();
        let result = reg.assign_task("Fix bug", "lead", "alpha");
        assert_eq!(result["ok"], true);
        assert_eq!(reg.tasks.len(), 1);
    }

    #[test]
    fn test_task_lifecycle() {
        let mut reg = A2ARegistry::new();
        let result = reg.assign_task("Test task", "lead", "alpha");
        let task_id = result["task_id"].as_str().unwrap();
        
        let status = reg.task_status(task_id);
        assert_eq!(status["state"], "assigned");
        
        let update = reg.update_task(task_id, TaskState::Completed);
        assert_eq!(update["ok"], true);
        
        let status = reg.task_status(task_id);
        assert_eq!(status["state"], "completed");
    }

    #[test]
    fn test_mcp_tool_definitions() {
        let tools = mcp_tool_definitions();
        assert_eq!(tools.len(), 4);
        assert_eq!(tools[0]["name"], "a2a_list_agents");
    }
}
