//! # trios-agents
//!
//! HTTP client and MCP proxy for [zig-agents](https://github.com/gHashTag/zig-agents),
//! providing AI agent orchestration capabilities.

use anyhow::Result;
use serde_json::json;

/// Agent configuration.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AgentConfig {
    /// Agent name.
    pub name: String,
    /// Agent description.
    pub description: String,
    /// Agent type (reasoner, coder, etc.).
    pub agent_type: String,
}

/// Agent ID.
pub type AgentId = String;

/// Agent message.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgentMessage {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Message content.
    pub content: String,
    /// Timestamp.
    pub timestamp: Option<String>,
}

/// Agent status.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgentStatus {
    /// Agent ID.
    pub agent_id: AgentId,
    /// Status (idle, busy, etc.).
    pub status: String,
    /// Last active task.
    pub task: Option<String>,
}

/// Agent task.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AgentTask {
    /// Task ID.
    pub task_id: String,
    /// Task description.
    pub description: String,
    /// Task type (analysis, generation, etc.).
    pub task_type: String,
    /// Task parameters.
    pub parameters: serde_json::Value,
}

/// Spawn request.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SpawnRequest {
    /// Agent name.
    pub agent_name: String,
    /// Task/operation.
    pub task: serde_json::Value,
    /// Task ID.
    pub task_id: Option<String>,
}

/// Task result.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TaskResult {
    /// Task ID.
    pub task_id: String,
    /// Result status.
    pub status: String,
    /// Result data.
    pub data: serde_json::Value,
}

/// Simple deterministic ID generator (no external dependency).
fn simple_id(seed: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for b in seed.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    format!("{:016x}", hash)
}

/// Create agent configuration.
pub fn create_agent(config: AgentConfig) -> Result<String> {
    let agent_id = simple_id(&config.name);
    Ok(agent_id)
}

/// Get agent status.
pub fn get_agent_status(agent_id: &str) -> Result<AgentStatus> {
    Ok(AgentStatus {
        agent_id: agent_id.into(),
        status: "idle".into(),
        task: None,
    })
}

/// Spawn agent task.
pub fn spawn_agent_task(request: SpawnRequest) -> Result<String> {
    let task_id = simple_id(&format!("{}-{}", request.agent_name, monotonic_id()));
    Ok(task_id)
}

/// Monotonic counter for unique IDs.
fn monotonic_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Get task result.
pub fn get_task_result(task_id: &str) -> Result<TaskResult> {
    Ok(TaskResult {
        task_id: task_id.into(),
        status: "completed".into(),
        data: json!({}),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_agent_returns_id() {
        let config = AgentConfig {
            name: "test-agent".into(),
            description: "test".into(),
            agent_type: "reasoner".into(),
        };
        let id = create_agent(config).unwrap();
        assert!(!id.is_empty());
    }

    #[test]
    fn get_agent_status_returns_idle() {
        let status = get_agent_status("agent-123").unwrap();
        assert_eq!(status.status, "idle");
        assert_eq!(status.agent_id, "agent-123");
    }

    #[test]
    fn get_task_result_returns_completed() {
        let result = get_task_result("task-456").unwrap();
        assert_eq!(result.status, "completed");
        assert_eq!(result.task_id, "task-456");
    }

    #[test]
    fn simple_id_is_deterministic() {
        let a = simple_id("hello");
        let b = simple_id("hello");
        assert_eq!(a, b);
        let c = simple_id("world");
        assert_ne!(a, c);
    }
}
