//! SR-01 — A2A Message Protocol
//!
//! Defines the message format for agent-to-agent communication.
//! All A2A interactions use A2AMessage as the envelope.
//!
//! ## Canonical wire contract (Wave 1 — schema parity)
//!
//! The on-wire JSON is the single source of truth shared with the Swift
//! client (`trios/rings/SR-01/A2AMessage.swift`) and the legacy Hono TS
//! server. Wire field names are `sender` / `recipient` / `type`, and the
//! `type` values are camelCase (`taskAssign`, `taskUpdate`, `taskResult`,
//! `addToolCall`, ...). Rust keeps ergonomic field names (`from` / `to` /
//! `msg_type`) internally and maps them to the wire via `#[serde(rename)]`,
//! so a message serialized by Rust round-trips through Swift/Hono and back
//! without a translation shim. This closes the P1 schema-divergence finding
//! from the A2A e2e audit.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use trios_a2a_sr00::AgentId;
use uuid::Uuid;

/// A2A message — the envelope for all agent communication.
///
/// Wire format matches the Swift/Hono clients exactly:
/// `{ id, sender, recipient, type, payload, timestamp }`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub id: String,
    /// Sender agent. Wire name: `sender`.
    #[serde(rename = "sender")]
    pub from: AgentId,
    /// Recipient agent. Wire name: `recipient` (optional on the wire —
    /// absent for broadcasts; deserializes to the `broadcast` sentinel).
    #[serde(rename = "recipient", default = "AgentId::broadcast_sentinel")]
    pub to: AgentId,
    /// Message type. Wire name: `type`.
    #[serde(rename = "type")]
    pub msg_type: A2AMessageType,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

/// Message types in the A2A protocol.
///
/// Wire values are camelCase to match the Swift `A2AMessageType` enum and
/// the Hono server (`direct`, `broadcast`, `taskAssign`, `taskUpdate`,
/// `taskResult`, `addToolCall`, `heartbeat`, `error`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
    /// Tool-call addition (Swift parity)
    AddToolCall,
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

    // ---- Wave 1: wire-contract parity with Swift / Hono -------------------

    #[test]
    fn wire_field_names_match_swift_hono() {
        let msg = A2AMessage::direct(
            AgentId::new("alpha"),
            AgentId::new("beta"),
            serde_json::json!({"text": "hi"}),
        );
        let v: serde_json::Value = serde_json::to_value(&msg).unwrap();
        let obj = v.as_object().unwrap();
        // Canonical wire fields (NOT from/to/msg_type).
        assert!(obj.contains_key("sender"), "wire must use `sender`");
        assert!(obj.contains_key("recipient"), "wire must use `recipient`");
        assert!(obj.contains_key("type"), "wire must use `type`");
        assert!(!obj.contains_key("from"));
        assert!(!obj.contains_key("to"));
        assert!(!obj.contains_key("msg_type"));
        assert_eq!(obj["sender"], serde_json::json!("alpha"));
        assert_eq!(obj["recipient"], serde_json::json!("beta"));
        assert_eq!(obj["type"], serde_json::json!("direct"));
    }

    #[test]
    fn message_type_values_are_camelcase() {
        for (variant, expected) in [
            (A2AMessageType::TaskAssign, "taskAssign"),
            (A2AMessageType::TaskUpdate, "taskUpdate"),
            (A2AMessageType::TaskResult, "taskResult"),
            (A2AMessageType::AddToolCall, "addToolCall"),
            (A2AMessageType::Heartbeat, "heartbeat"),
            (A2AMessageType::Error, "error"),
        ] {
            let s = serde_json::to_string(&variant).unwrap();
            assert_eq!(s, format!("\"{expected}\""));
        }
    }

    #[test]
    fn deserializes_swift_style_envelope() {
        // Exactly what the Swift client / Hono server put on the wire.
        let wire = r#"{
            "id": "m1",
            "sender": "alpha",
            "recipient": "beta",
            "type": "taskAssign",
            "payload": {"task": "build"},
            "timestamp": "2026-07-24T00:00:00Z"
        }"#;
        let msg: A2AMessage = serde_json::from_str(wire).unwrap();
        assert_eq!(msg.id, "m1");
        assert_eq!(msg.from.as_str(), "alpha");
        assert_eq!(msg.to.as_str(), "beta");
        assert_eq!(msg.msg_type, A2AMessageType::TaskAssign);
    }

    #[test]
    fn broadcast_without_recipient_uses_sentinel() {
        // Swift sends `recipient: null` (or omits it) for broadcasts.
        let wire = r#"{
            "id": "m2",
            "sender": "alpha",
            "type": "broadcast",
            "payload": {},
            "timestamp": "2026-07-24T00:00:00Z"
        }"#;
        let msg: A2AMessage = serde_json::from_str(wire).unwrap();
        assert!(msg.to.is_broadcast());
        assert_eq!(msg.msg_type, A2AMessageType::Broadcast);
    }

    #[test]
    fn roundtrip_rust_to_wire_to_rust() {
        let msg = A2AMessage::direct(
            AgentId::new("a"),
            AgentId::new("b"),
            serde_json::json!({"k": 1}),
        );
        let wire = serde_json::to_string(&msg).unwrap();
        let back: A2AMessage = serde_json::from_str(&wire).unwrap();
        assert_eq!(back.from.as_str(), "a");
        assert_eq!(back.to.as_str(), "b");
        assert_eq!(back.msg_type, A2AMessageType::Direct);
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
