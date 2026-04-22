//! SR-01 — A2A Message Protocol
//!
//! Defines the message format for agent-to-agent communication.
//! All A2A interactions use A2AMessage as the envelope.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use trios_a2a_sr00::AgentId;
use uuid::Uuid;

/// A2A message — the envelope for all agent communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub msg_type: A2AMessageType,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Message types in the A2A protocol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum A2AMessageType {
    /// Direct message to another agent
    Direct,
    /// Broadcast to all agents
    Broadcast,
    /// Task assignment
    TaskAssign,
    /// Task status update
    TaskUpdate,
    /// Task result
    TaskResult,
    /// Heartbeat / ping
    Heartbeat,
    /// Error notification
    Error,
}

impl A2AMessage {
    /// Create a new direct message.
    pub fn direct(from: AgentId, to: AgentId, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to,
            msg_type: A2AMessageType::Direct,
            payload,
            timestamp: Utc::now().to_rfc3339(),
        }
    }

    /// Create a broadcast message.
    pub fn broadcast(from: AgentId, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from,
            to: AgentId::new("broadcast"),
            msg_type: A2AMessageType::Broadcast,
            payload,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Task in the A2A system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub assigned_to: Option<AgentId>,
    pub created_by: AgentId,
    pub state: TaskState,
    pub priority: TaskPriority,
    pub created_at: String,
    pub updated_at: String,
}

/// Task lifecycle states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    /// Task created, not yet assigned
    Pending,
    /// Task assigned to an agent
    Assigned,
    /// Agent is working on it
    InProgress,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task cancelled
    Cancelled,
}

/// Task priority levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Task {
    /// Create a new task.
    pub fn new(title: impl Into<String>, created_by: AgentId) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            assigned_to: None,
            created_by,
            state: TaskState::Pending,
            priority: TaskPriority::Medium,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Assign task to an agent.
    pub fn assign_to(mut self, agent_id: AgentId) -> Self {
        self.assigned_to = Some(agent_id);
        self.state = TaskState::Assigned;
        self.updated_at = Utc::now().to_rfc3339();
        self
    }

    /// Check if task is terminal (completed/failed/cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            TaskState::Completed | TaskState::Failed | TaskState::Cancelled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_message() {
        let msg = A2AMessage::direct(
            AgentId::new("alpha"),
            AgentId::new("beta"),
            serde_json::json!({"text": "hello"}),
        );
        assert_eq!(msg.msg_type, A2AMessageType::Direct);
        assert_eq!(msg.from.as_str(), "alpha");
        assert_eq!(msg.to.as_str(), "beta");
    }

    #[test]
    fn test_broadcast_message() {
        let msg = A2AMessage::broadcast(AgentId::new("alpha"), serde_json::json!({"event": "ping"}));
        assert_eq!(msg.msg_type, A2AMessageType::Broadcast);
        assert_eq!(msg.to.as_str(), "broadcast");
    }

    #[test]
    fn test_task_lifecycle() {
        let task = Task::new("Fix bug #42", AgentId::new("lead"));
        assert_eq!(task.state, TaskState::Pending);
        assert!(task.assigned_to.is_none());

        let task = task.assign_to(AgentId::new("alpha"));
        assert_eq!(task.state, TaskState::Assigned);
        assert_eq!(task.assigned_to.as_ref().unwrap().as_str(), "alpha");
        assert!(!task.is_terminal());
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new("Test task", AgentId::new("lead"));
        let json = serde_json::to_string(&task).unwrap();
        let parsed: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.title, "Test task");
    }
}
