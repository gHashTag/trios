//! Bridge message types for Comet protocol.
//!
//! EnvelopeT<AgentPayload> equivalent - unified message format for
//! trios-server ↔ Chrome extension communication.

use serde::{Deserialize, Serialize};

/// Envelope wrapper for all bridge messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub event_type: String,
    pub payload: Payload,
}

impl Envelope {
    pub fn new(event_type: impl Into<String>, payload: Payload) -> Self {
        Self {
            event_type: event_type.into(),
            payload,
        }
    }

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

    pub fn agent_connected(agent_id: impl Into<String>) -> Self {
        Self::new(
            "agent:connected",
            Payload::AgentConnected {
                agent_id: agent_id.into(),
            },
        )
    }

    pub fn agent_disconnected(agent_id: impl Into<String>) -> Self {
        Self::new(
            "agent:disconnected",
            Payload::AgentDisconnected {
                agent_id: agent_id.into(),
            },
        )
    }

    pub fn mcp_tool_call(tool: impl Into<String>, params: serde_json::Value) -> Self {
        Self::new(
            "mcp:tool_call",
            Payload::McpToolCall {
                tool: tool.into(),
                params,
            },
        )
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Typed payload variants for bridge messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum Payload {
    AgentHeartbeat {
        agent_id: String,
        status: String,
        timestamp: u64,
    },
    ChatMessage {
        agent_id: String,
        content: String,
        role: String,
    },
    AgentConnected { agent_id: String },
    AgentDisconnected { agent_id: String },
    McpToolCall {
        tool: String,
        params: serde_json::Value,
    },
}

impl Payload {
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
    use serde_json::json;

    // ── Базовая сериализация ─────────────────────────────────────────────────

    #[test]
    fn envelope_serialization_heartbeat() {
        let env = Envelope::agent_heartbeat("ALFA-01", "active");
        let json = env.to_json().unwrap();
        assert!(json.contains("agent:heartbeat"));
        assert!(json.contains("ALFA-01"));
        assert!(json.contains("active"));
    }

    #[test]
    fn envelope_deserialization_chat() {
        let raw = r#"{"event_type":"chat:message","payload":{"type":"ChatMessage","data":{"agent_id":"BRAVO-02","content":"Hello","role":"assistant"}}}"#;
        let env = Envelope::from_json(raw).unwrap();
        assert_eq!(env.event_type, "chat:message");
        match env.payload {
            Payload::ChatMessage { agent_id, content, role } => {
                assert_eq!(agent_id, "BRAVO-02");
                assert_eq!(content, "Hello");
                assert_eq!(role, "assistant");
            }
            _ => panic!("wrong payload variant"),
        }
    }

    // ── Roundtrip для всех 5 вариантов Payload ────────────────────────────────

    #[test]
    fn roundtrip_agent_heartbeat() {
        let env = Envelope::agent_heartbeat("ALPHA", "idle");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        assert_eq!(rt.event_type, "agent:heartbeat");
        match rt.payload {
            Payload::AgentHeartbeat { agent_id, status, .. } => {
                assert_eq!(agent_id, "ALPHA");
                assert_eq!(status, "idle");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_chat_message() {
        let env = Envelope::chat_message("BETA", "test content", "user");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        assert_eq!(rt.event_type, "chat:message");
        match rt.payload {
            Payload::ChatMessage { agent_id, content, role } => {
                assert_eq!(agent_id, "BETA");
                assert_eq!(content, "test content");
                assert_eq!(role, "user");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_agent_connected() {
        let env = Envelope::agent_connected("GAMMA");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        assert_eq!(rt.event_type, "agent:connected");
        match rt.payload {
            Payload::AgentConnected { agent_id } => assert_eq!(agent_id, "GAMMA"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_agent_disconnected() {
        let env = Envelope::agent_disconnected("DELTA");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        assert_eq!(rt.event_type, "agent:disconnected");
        match rt.payload {
            Payload::AgentDisconnected { agent_id } => assert_eq!(agent_id, "DELTA"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn roundtrip_mcp_tool_call() {
        let params = json!({ "key": "val", "n": 42, "nested": { "x": true } });
        let env = Envelope::mcp_tool_call("my_tool", params.clone());
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        assert_eq!(rt.event_type, "mcp:tool_call");
        match rt.payload {
            Payload::McpToolCall { tool, params: p } => {
                assert_eq!(tool, "my_tool");
                assert_eq!(p, params);
            }
            _ => panic!("wrong variant"),
        }
    }

    // ── Инвариант: payload.event_type() == envelope.event_type ──────────────

    #[test]
    fn event_type_consistency_all_variants() {
        let cases = [
            Envelope::agent_heartbeat("A", "ok"),
            Envelope::chat_message("A", "msg", "user"),
            Envelope::agent_connected("A"),
            Envelope::agent_disconnected("A"),
            Envelope::mcp_tool_call("tool", json!({})),
        ];
        for env in &cases {
            assert_eq!(
                env.payload.event_type(),
                env.event_type,
                "event_type mismatch for: {}",
                env.event_type
            );
        }
    }

    // ── Обработка ошибок ─────────────────────────────────────────────────────

    #[test]
    fn from_json_invalid_string_returns_error() {
        assert!(Envelope::from_json("not json at all").is_err());
    }

    #[test]
    fn from_json_empty_object_returns_error() {
        assert!(Envelope::from_json("{}").is_err()); // нет event_type и payload
    }

    #[test]
    fn from_json_missing_payload_returns_error() {
        let json = r#"{"event_type": "chat:message"}"#;
        assert!(Envelope::from_json(json).is_err());
    }

    #[test]
    fn from_json_missing_event_type_returns_error() {
        let json = r#"{"payload": {"type": "AgentConnected", "data": {"agent_id": "X"}}}"#;
        assert!(Envelope::from_json(json).is_err());
    }

    #[test]
    fn from_json_unknown_payload_type_returns_error() {
        let json = r#"{"event_type":"x","payload":{"type":"UnknownVariant","data":{}}}"#;
        assert!(Envelope::from_json(json).is_err());
    }

    // ── Payload::event_type() для всех вариантов ─────────────────────────────

    #[test]
    fn payload_event_type_all_variants() {
        assert_eq!(
            Payload::AgentHeartbeat { agent_id: "x".into(), status: "ok".into(), timestamp: 0 }
                .event_type(),
            "agent:heartbeat"
        );
        assert_eq!(
            Payload::ChatMessage { agent_id: "x".into(), content: "c".into(), role: "user".into() }
                .event_type(),
            "chat:message"
        );
        assert_eq!(
            Payload::AgentConnected { agent_id: "x".into() }.event_type(),
            "agent:connected"
        );
        assert_eq!(
            Payload::AgentDisconnected { agent_id: "x".into() }.event_type(),
            "agent:disconnected"
        );
        assert_eq!(
            Payload::McpToolCall { tool: "t".into(), params: json!({}) }.event_type(),
            "mcp:tool_call"
        );
    }

    // ── Конструкторы и поля ──────────────────────────────────────────────────

    #[test]
    fn chat_message_constructor_fields() {
        let env = Envelope::chat_message("ZETA", "hello world", "assistant");
        assert_eq!(env.event_type, "chat:message");
        match env.payload {
            Payload::ChatMessage { agent_id, content, role } => {
                assert_eq!(agent_id, "ZETA");
                assert_eq!(content, "hello world");
                assert_eq!(role, "assistant");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn mcp_tool_call_preserves_complex_params() {
        let params = json!({
            "array": [1, 2, 3],
            "nested": { "deep": { "value": true } },
            "null_field": null
        });
        let env = Envelope::mcp_tool_call("complex_tool", params.clone());
        match env.payload {
            Payload::McpToolCall { params: p, .. } => assert_eq!(p, params),
            _ => panic!(),
        }
    }

    #[test]
    fn agent_heartbeat_timestamp_is_nonzero_on_native() {
        let env = Envelope::agent_heartbeat("TEST", "active");
        match env.payload {
            Payload::AgentHeartbeat { timestamp, .. } => {
                assert!(timestamp > 0, "timestamp should be > 0 on native");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn envelope_clone_is_independent() {
        let env1 = Envelope::agent_connected("X");
        let env2 = env1.clone();
        assert_eq!(env1.event_type, env2.event_type);
    }

    // ── Граничные случаи ─────────────────────────────────────────────────────

    #[test]
    fn empty_string_fields_are_valid() {
        let env = Envelope::chat_message("", "", "");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        match rt.payload {
            Payload::ChatMessage { agent_id, content, role } => {
                assert_eq!(agent_id, "");
                assert_eq!(content, "");
                assert_eq!(role, "");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn unicode_content_roundtrip() {
        let env = Envelope::chat_message("А", "Привет мир 🌍", "user");
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        match rt.payload {
            Payload::ChatMessage { content, .. } => assert_eq!(content, "Привет мир 🌍"),
            _ => panic!(),
        }
    }

    #[test]
    fn tool_call_empty_params_roundtrip() {
        let env = Envelope::mcp_tool_call("noop", json!({}));
        let rt = Envelope::from_json(&env.to_json().unwrap()).unwrap();
        match rt.payload {
            Payload::McpToolCall { params, .. } => assert_eq!(params, json!({})),
            _ => panic!(),
        }
    }
}
