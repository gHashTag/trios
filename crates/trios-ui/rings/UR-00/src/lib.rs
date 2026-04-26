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
use std::sync::RwLock;

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    /// Agent is idle and available.
    Idle,
    /// Agent is working on a task.
    Busy,
    /// Agent encountered an error.
    Error(String),
    /// Agent is offline.
    Offline,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Offline
    }
}

// ─── Chat types ──────────────────────────────────────────────

/// Chat state atom.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

impl Default for ChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            input: String::new(),
            is_loading: false,
            active_agent_id: None,
        }
    }
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

// ─── Global Signal atoms (Jotai-style) ──────────────────────

/// Global agents atom. Use `use_agents_atom()` to access.
static AGENTS_ATOM: RwLock<Vec<Agent>>> = RwLock::new(Vec::new());

/// Global chat state atom. Use `use_chat_atom()` to access.
static CHAT_ATOM: RwLock<ChatState> = RwLock::new(ChatState::default());

/// Global MCP state atom. Use `use_mcp_atom()` to access.
static MCP_ATOM: RwLock<McpState> = RwLock::new(McpState::default());

/// Global settings atom. Use `use_settings_atom()` to access.
static SETTINGS_ATOM: RwLock<Settings> = RwLock::new(Settings::default());

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
    AGENTS_ATOM.get()
}

/// Access the global chat state atom.
pub fn use_chat_atom() -> ChatState {
    CHAT_ATOM.read().clone()
}

/// Set the global chat state atom.
pub fn set_chat(state: ChatState) {
    *CHAT_ATOM.write().unwrap() = state;
}

/// Access the global MCP state atom.
pub fn use_mcp_atom() -> McpState {
    MCP_ATOM.read().clone()
}

/// Set the global MCP state atom.
pub fn set_mcp(state: McpState) {
    *MCP_ATOM.write().unwrap() = state;
}

/// Access the global settings atom.
pub fn use_settings_atom() -> Settings {
    SETTINGS_ATOM.read().clone()
}

/// Set the global settings atom.
pub fn set_settings(settings: Settings) {
    *SETTINGS_ATOM.write().unwrap() = settings;
}
