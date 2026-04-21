//! Bridge message types for Comet protocol.
//!
//! EnvelopeT<AgentPayload> equivalent - unified message format for
//! trios-server ↔ Chrome extension communication.

use serde::{Deserialize, Serialize};

/// Envelope wrapper for all bridge messages.
///
/// Equivalent to EnvelopeT<AgentPayload> from trios-proto (not yet implemented).
/// Contains event type metadata and typed payload data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Event type identifier for message routing
    pub event_type: String,

    /// Typed payload data
    pub payload: Payload,
}

impl Envelope {
    /// Create a new envelope with event type and payload.
    pub fn new(event_type: impl Into<String>, payload: Payload) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
        }
    }

    /// Create an agent heartbeat envelope.
    pub fn agent_heartbeat(agent_id: impl Into<String>, status: impl Into<String>) -> Self {
        Self::new(
            "agent:heartbeat",
            Payload::AgentHeartbeat {
                agent_id: agent_id.into(),
                status: status.into(),
                timestamp: timestamp_ms(),
            },
        )
    }

    /// Create a chat message envelope.
    pub fn chat_message(
        agent_id: impl Into<String>,
        content: impl Into<String>,
        role: impl Into<String>,
    ) -> Self {
        Self::new(
            "chat:message",
            Payload::ChatMessage {
                agent_id: agent_id.into(),
                content: content.into(),
                role: role.into(),
            },
        )
    }

    /// Create an agent connected envelope.
    pub fn agent_connected(agent_id: impl Into<String>) -> Self {
        Self::new(
            "agent:connected",
            Payload::AgentConnected {
                agent_id: agent_id.into(),
            },
        )
    }

    /// Create an agent disconnected envelope.
    pub fn agent_disconnected(agent_id: impl Into<String>) -> Self {
        Self::new(
            "agent:disconnected",
            Payload::AgentDisconnected {
                agent_id: agent_id.into(),
            },
        )
    }

    /// Create an MCP tool call envelope.
    pub fn mcp_tool_call(tool: impl Into<String>, params: serde_json::Value) -> Self {
        Self::new(
            "mcp:tool_call",
            Payload::McpToolCall {
                tool: tool.into(),
                params,
            },
        )
    }

    /// Serialize envelope to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize envelope from JSON string.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Typed payload variants for bridge messages.
///
/// Uses internally-tagged representation for clean JSON serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Payload {
    /// Agent heartbeat signal with status and timestamp.
    AgentHeartbeat {
        agent_id: String,
        status: String,
        timestamp: u64,
    },

    /// Chat message from user or agent.
    ChatMessage {
        agent_id: String,
        content: String,
        role: String,
    },

    /// Agent connected to the system.
    AgentConnected { agent_id: String },

    /// Agent disconnected from the system.
    AgentDisconnected { agent_id: String },

    /// MCP tool invocation request/response.
    McpToolCall {
        tool: String,
        params: serde_json::Value,
    },
}

impl Payload {
    /// Get the event type string for this payload variant.
    pub fn event_type(&self) -> &'static str {
        match self {
            Payload::AgentHeartbeat { .. } => "agent:heartbeat",
            Payload::ChatMessage { .. } => "chat:message",
            Payload::AgentConnected { .. } => "agent:connected",
            Payload::AgentDisconnected { .. } => "agent:disconnected",
            Payload::McpToolCall { .. } => "mcp:tool_call",
        }
    }
}

/// Get current Unix timestamp in milliseconds.
fn timestamp_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::Date;
        Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_serialization() {
        let envelope = Envelope::agent_heartbeat("ALFA-01", "active");
        let json = envelope.to_json().unwrap();

        assert!(json.contains("agent:heartbeat"));
        assert!(json.contains("ALFA-01"));
        assert!(json.contains("active"));
    }

    #[test]
    fn envelope_deserialization() {
        let json = r#"{
            "event_type": "chat:message",
            "payload": {
                "type": "ChatMessage",
                "data": {
                    "agent_id": "BRAVO-02",
                    "content": "Hello",
                    "role": "assistant"
                }
            }
        }"#;

        let envelope = Envelope::from_json(json).unwrap();
        assert_eq!(envelope.event_type, "chat:message");

        match envelope.payload {
            Payload::ChatMessage {
                agent_id,
                content,
                role,
            } => {
                assert_eq!(agent_id, "BRAVO-02");
                assert_eq!(content, "Hello");
                assert_eq!(role, "assistant");
            }
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn payload_event_type() {
        assert_eq!(
            Payload::AgentHeartbeat {
                agent_id: "test".into(),
                status: "ok".into(),
                timestamp: 0
            }
            .event_type(),
            "agent:heartbeat"
        );
        assert_eq!(
            Payload::ChatMessage {
                agent_id: "test".into(),
                content: "hi".into(),
                role: "user".into()
            }
            .event_type(),
            "chat:message"
        );
        assert_eq!(
            Payload::AgentConnected {
                agent_id: "test".into()
            }
            .event_type(),
            "agent:connected"
        );
        assert_eq!(
            Payload::AgentDisconnected {
                agent_id: "test".into()
            }
            .event_type(),
            "agent:disconnected"
        );
        assert_eq!(
            Payload::McpToolCall {
                tool: "test_tool".into(),
                params: serde_json::json!({})
            }
            .event_type(),
            "mcp:tool_call"
        );
    }
}
