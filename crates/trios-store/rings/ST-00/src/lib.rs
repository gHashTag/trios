//! ST-00 — Store schema types
//!
//! Pure data + serde. Mirrors the drizzle SQLite schema from
//! `browseros-agent/apps/server/src/lib/db/schema/*` 1:1 so the Rust
//! backend reads/writes the SAME database file during migration.
//!
//! Three tables: `agent_definitions`, `oauth_tokens`, `produced_files`.
//! No I/O, no async, no SQL — this is the bottom of the ring graph.

use serde::{Deserialize, Serialize};

/// Adapter kind for an agent definition (drizzle enum).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Adapter {
    Claude,
    Codex,
    Openclaw,
    Hermes,
}

impl Adapter {
    pub fn as_str(&self) -> &'static str {
        match self {
            Adapter::Claude => "claude",
            Adapter::Codex => "codex",
            Adapter::Openclaw => "openclaw",
            Adapter::Hermes => "hermes",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "claude" => Some(Adapter::Claude),
            "codex" => Some(Adapter::Codex),
            "openclaw" => Some(Adapter::Openclaw),
            "hermes" => Some(Adapter::Hermes),
            _ => None,
        }
    }
}

/// How a produced file was detected (drizzle enum, default `diff`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectedBy {
    Diff,
    Tool,
}

impl Default for DetectedBy {
    fn default() -> Self {
        DetectedBy::Diff
    }
}

impl DetectedBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            DetectedBy::Diff => "diff",
            DetectedBy::Tool => "tool",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "tool" {
            DetectedBy::Tool
        } else {
            DetectedBy::Diff
        }
    }
}

/// Row of `agent_definitions`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDefinitionRow {
    pub id: String,
    pub name: String,
    pub adapter: Adapter,
    pub model_id: String,
    pub reasoning_effort: String,
    /// drizzle enum `permission_mode` — only `approve-all`, default `approve-all`.
    pub permission_mode: String,
    pub session_key: String,
    pub pinned: bool,
    pub adapter_config_json: Option<String>,
    /// epoch millis
    pub created_at: i64,
    pub updated_at: i64,
}

/// Row of `oauth_tokens` (composite PK: browseros_id + provider).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuthTokenRow {
    pub browseros_id: String,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub updated_at: i64,
}

/// Row of `produced_files`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProducedFileRow {
    pub id: String,
    pub agent_definition_id: String,
    pub session_key: String,
    pub turn_id: String,
    pub turn_prompt: String,
    pub path: String,
    pub size: i64,
    pub mtime_ms: i64,
    pub created_at: i64,
    pub detected_by: DetectedBy,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_roundtrip() {
        for a in [Adapter::Claude, Adapter::Codex, Adapter::Openclaw, Adapter::Hermes] {
            assert_eq!(Adapter::parse(a.as_str()), Some(a.clone()));
        }
        assert_eq!(Adapter::parse("nope"), None);
    }

    #[test]
    fn detected_by_default_is_diff() {
        assert_eq!(DetectedBy::default(), DetectedBy::Diff);
        assert_eq!(DetectedBy::parse("weird"), DetectedBy::Diff);
        assert_eq!(DetectedBy::parse("tool"), DetectedBy::Tool);
    }

    #[test]
    fn agent_row_serde() {
        let row = AgentDefinitionRow {
            id: "a1".into(),
            name: "Agent".into(),
            adapter: Adapter::Openclaw,
            model_id: "m".into(),
            reasoning_effort: "high".into(),
            permission_mode: "approve-all".into(),
            session_key: "s1".into(),
            pinned: true,
            adapter_config_json: None,
            created_at: 1,
            updated_at: 2,
        };
        let j = serde_json::to_string(&row).unwrap();
        let back: AgentDefinitionRow = serde_json::from_str(&j).unwrap();
        assert_eq!(row, back);
    }
}
