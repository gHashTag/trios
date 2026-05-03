//! UR-00 — State atoms (Jotai-style Dioxus Signals)
//!
//! Provides global reactive atoms for the entire TRIOS UI.
//! Each atom is a `Signal<T>` created via `Signal::new()` and
//! accessed through convenience hooks (`use_agents_atom()`, etc.).
//!
//! ## Atoms
//!
//! | Atom | Type | Purpose |
//! |------|------|---------|
//! | `AgentsAtom` | `Vec<Agent>` | Agent list & status |
//! | `ChatAtom` | `ChatState` | Messages, input, current chat |
//! | `McpAtom` | `McpState` | MCP tools & connection status |
//! | `SettingsAtom` | `Settings` | Theme, API key, preferences |

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

// ─── Agent types ──────────────────────────────────────────────

/// Agent data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Agent {
    /// Agent ID.
    pub id: String,
    /// Agent display name.
    pub name: String,
    /// Agent description.
    pub description: String,
    /// Agent type (reasoner, coder, etc.).
    pub agent_type: String,
    /// Current status.
    pub status: AgentStatus,
}

/// Agent status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AgentStatus {
    /// Agent is offline (default).
    #[default]
    Offline,
    /// Agent is idle and available.
    Idle,
    /// Agent is working on a task.
    Busy,
    /// Agent encountered an error.
    Error(String),
}

// ─── Chat types ──────────────────────────────────────────────

/// Chat state atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatState {
    /// Chat messages.
    pub messages: Vec<ChatMessage>,
    /// Current input text.
    pub input: String,
    /// Whether we're waiting for a response.
    pub is_loading: bool,
    /// Active agent ID for the chat.
    pub active_agent_id: Option<String>,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    /// Unique message ID.
    pub id: String,
    /// Sender role.
    pub role: MessageRole,
    /// Message content.
    pub content: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Message sender role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    /// User message.
    User,
    /// Agent/assistant message.
    Assistant,
    /// System message.
    System,
}

// ─── MCP types ──────────────────────────────────────────────

/// MCP state atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpState {
    /// Available MCP tools.
    pub tools: Vec<McpTool>,
    /// Connection status.
    pub connected: bool,
    /// Server URL.
    pub server_url: String,
}

impl Default for McpState {
    fn default() -> Self {
        Self {
            tools: Vec::new(),
            connected: false,
            server_url: "http://localhost:9005".to_string(),
        }
    }
}

/// MCP tool descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpTool {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON schema for tool parameters.
    pub parameters: Option<String>,
}

// ─── Settings types ──────────────────────────────────────────

/// Settings atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    /// Active theme.
    pub theme: Theme,
    /// API key for z.ai direct chat.
    pub api_key: String,
    /// MCP server URL override.
    pub mcp_url: String,
    /// Sidebar collapsed state.
    pub sidebar_collapsed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            api_key: String::new(),
            mcp_url: "http://localhost:9005".to_string(),
            sidebar_collapsed: false,
        }
    }
}

/// Theme variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    /// Dark theme (default).
    Dark,
    /// Light theme.
    Light,
}

// ─── A2A Social types (UR-09) ─────────────────────────────────

/// A2A Social state atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2AState {
    /// A2A bus messages.
    pub messages: Vec<A2AMessage>,
    /// Agent presence map (name → entry).
    pub presence: std::collections::HashMap<String, A2APresenceEntry>,
    /// Whether bus is connected.
    pub connected: bool,
    /// Whether interrupt is active.
    pub interrupt_active: bool,
    /// Conversation ID.
    pub conversation_id: String,
}

impl Default for A2AState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            presence: std::collections::HashMap::new(),
            connected: false,
            interrupt_active: false,
            conversation_id: "trinity-ops-2026-05-03".to_string(),
        }
    }
}

impl A2AState {
    /// Check if an agent is online (seen within 120s).
    pub fn is_agent_online(&self, name: &str) -> bool {
        self.presence.get(name).map_or(false, |e| {
            let now = js_sys::Date::now() as u64;
            now.saturating_sub(e.last_seen) < 120_000
        })
    }
}

/// A single A2A bus message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2AMessage {
    /// Unique message ID.
    pub id: String,
    /// Message type (chat, interrupt, abort, interrupted, presence).
    #[serde(rename = "type")]
    pub msg_type: String,
    /// Sender role (human, agent).
    pub role: String,
    /// Sender agent name.
    #[serde(rename = "agentName")]
    pub agent_name: String,
    /// Message content.
    pub content: String,
    /// Conversation ID.
    #[serde(rename = "conversationId")]
    pub conversation_id: String,
    /// Timestamp (epoch ms).
    pub timestamp: u64,
}

/// A2A presence entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct A2APresenceEntry {
    /// Agent role.
    pub role: String,
    /// Last seen timestamp (epoch ms).
    #[serde(rename = "lastSeen")]
    pub last_seen: u64,
    /// Status (join, heartbeat, leave).
    pub status: String,
}

/// Agent profile for social display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProfile {
    /// Agent name (matches A2AMessage.agent_name).
    pub name: String,
    /// Display emoji.
    pub emoji: String,
    /// Display label.
    pub label: String,
    /// Agent color (CSS hex).
    pub color: String,
    /// Description.
    pub desc: String,
}

impl AgentProfile {
    pub fn human() -> Self {
        Self { name: "HumanOverlord".into(), emoji: "👑".into(), label: "You".into(), color: "#D4AF37".into(), desc: "Human-in-the-Loop — veto power".into() }
    }
    pub fn browser_os() -> Self {
        Self { name: "BrowserOS-Agent".into(), emoji: "🤖".into(), label: "BOS".into(), color: "#4fc3f7".into(), desc: "Local browser agent".into() }
    }
    pub fn scarabs() -> Self {
        Self { name: "PerplexityScarabs".into(), emoji: "🕷️".into(), label: "Scarabs".into(), color: "#ff6b9d".into(), desc: "Cloud code agent".into() }
    }
    pub fn phi_t27() -> Self {
        Self { name: "phi-t27".into(), emoji: "φ".into(), label: "t27".into(), color: "#FF6B6B".into(), desc: "Trinity compute agent".into() }
    }
    pub fn from_name(name: &str) -> Self {
        match name {
            "HumanOverlord" => Self::human(),
            "BrowserOS-Agent" => Self::browser_os(),
            "PerplexityScarabs" => Self::scarabs(),
            "phi-t27" => Self::phi_t27(),
            _ => Self { name: name.into(), emoji: "❓".into(), label: name.into(), color: "#666".into(), desc: String::new() },
        }
    }
}

// ─── Global Signal atoms (Jotai-style) ──────────────────────

/// Global agents atom. Use `use_agents_atom()` to access.
static AGENTS_ATOM: GlobalSignal<Vec<Agent>> = GlobalSignal::new(Vec::new);

/// Global chat state atom. Use `use_chat_atom()` to access.
static CHAT_ATOM: GlobalSignal<ChatState> = GlobalSignal::new(ChatState::default);

/// Global MCP state atom. Use `use_mcp_atom()` to access.
static MCP_ATOM: GlobalSignal<McpState> = GlobalSignal::new(McpState::default);

/// Global settings atom. Use `use_settings_atom()` to access.
static SETTINGS_ATOM: GlobalSignal<Settings> = GlobalSignal::new(Settings::default);

/// Global A2A social state atom. Use `use_a2a_atom()` to access.
pub static A2A_ATOM: GlobalSignal<A2AState> = GlobalSignal::new(A2AState::default);

// ─── Atom accessors (Jotai-style hooks) ─────────────────────

/// Access the global agents atom.
///
/// # Example
/// ```rust,ignore
/// fn MyComponent() -> Element {
///     let agents = use_agents_atom();
///     rsx! { {agents.len()} agents loaded }
/// }
/// ```
pub fn use_agents_atom() -> Signal<Vec<Agent>> {
    AGENTS_ATOM.signal()
}

/// Access the global chat state atom.
pub fn use_chat_atom() -> Signal<ChatState> {
    CHAT_ATOM.signal()
}

/// Access the global MCP state atom.
pub fn use_mcp_atom() -> Signal<McpState> {
    MCP_ATOM.signal()
}

/// Access the global settings atom.
pub fn use_settings_atom() -> Signal<Settings> {
    SETTINGS_ATOM.signal()
}

/// Access the global A2A social state atom.
pub fn use_a2a_atom() -> Signal<A2AState> {
    A2A_ATOM.signal()
}
