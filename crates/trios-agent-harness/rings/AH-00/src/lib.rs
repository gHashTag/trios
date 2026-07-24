//! AH-00 — Agent-harness core types.
//!
//! Pure data + serde. Ported 1:1 from the TS backend
//! (`lib/agents/agent-types.ts`, `lib/agents/types.ts`). No I/O, no async.
//! This is the bottom of the trios-agent-harness ring graph.

use serde::{Deserialize, Serialize};

/// Agent adapter kind (TS `AgentAdapter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentAdapter {
    Claude,
    Codex,
    Openclaw,
    Hermes,
}

impl AgentAdapter {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentAdapter::Claude => "claude",
            AgentAdapter::Codex => "codex",
            AgentAdapter::Openclaw => "openclaw",
            AgentAdapter::Hermes => "hermes",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "openclaw" => Some(Self::Openclaw),
            "hermes" => Some(Self::Hermes),
            _ => None,
        }
    }
}

/// Permission mode — currently only `approve-all` (TS `AgentPermissionMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PermissionMode {
    ApproveAll,
}

impl Default for PermissionMode {
    fn default() -> Self {
        PermissionMode::ApproveAll
    }
}

/// Agent definition (TS `AgentDefinition`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub adapter: AgentAdapter,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning_effort: Option<String>,
    pub permission_mode: PermissionMode,
    pub session_key: String,
    pub created_at: i64,
    pub updated_at: i64,
    /// Pinned agents float to the top of the rail. Defaulted on read.
    #[serde(default)]
    pub pinned: bool,
}

/// Agent runtime status (TS `AgentStatus`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub state: AgentState,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Ready,
    Unknown,
    Error,
}

/// A tool call inside a history entry (TS `AgentHistoryToolCall`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryToolCall {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
    pub tool_name: String,
    pub status: ToolCallStatus,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub output: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// One history entry (TS `AgentHistoryEntry`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub agent_id: String,
    pub role: HistoryRole,
    pub text: String,
    pub created_at: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<HistoryToolCall>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HistoryRole {
    User,
    Assistant,
}

/// Streamed event during a turn (TS `AgentStreamEvent`), tagged by `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    TextDelta { text: String, stream: TextStream },
    ToolCall { text: String, title: String, #[serde(default)] status: Option<String> },
    Status { text: String },
    Done { #[serde(default)] text: Option<String>, #[serde(default)] stop_reason: Option<String> },
    Error { message: String, #[serde(default)] code: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextStream {
    Output,
    Thought,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_roundtrip() {
        for a in [AgentAdapter::Claude, AgentAdapter::Codex, AgentAdapter::Openclaw, AgentAdapter::Hermes] {
            assert_eq!(AgentAdapter::parse(a.as_str()), Some(a));
        }
    }

    #[test]
    fn agent_definition_camelcase_wire() {
        let d = AgentDefinition {
            id: "a1".into(),
            name: "A".into(),
            adapter: AgentAdapter::Openclaw,
            model_id: Some("m".into()),
            reasoning_effort: None,
            permission_mode: PermissionMode::ApproveAll,
            session_key: "s".into(),
            created_at: 1,
            updated_at: 2,
            pinned: true,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert!(v.get("modelId").is_some());
        assert!(v.get("sessionKey").is_some());
        assert_eq!(v["permissionMode"], serde_json::json!("approve-all"));
        assert!(v.get("reasoningEffort").is_none()); // skipped when None
        let back: AgentDefinition = serde_json::from_value(v).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn stream_event_tagged() {
        let e = AgentStreamEvent::TextDelta { text: "hi".into(), stream: TextStream::Output };
        let v = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], serde_json::json!("text_delta"));
        assert_eq!(v["stream"], serde_json::json!("output"));
    }
}
