//! SR-03 — BrowserOS A2A Agent
//!
//! Server-side types for browser control via A2A/MCP protocol.
//! AI agents call MCP tools → commands queue → Chrome extension polls → executes → reports back.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use uuid::Uuid;

/// All browser operations available via MCP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserCommandType {
    GetUrl,
    GetTitle,
    Navigate,
    GetDom,
    QuerySelector,
    Click,
    Type,
    Scroll,
    Eval,
    Screenshot,
    OpenTab,
    CloseTab,
}

impl std::fmt::Display for BrowserCommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "browser_{}", s)
    }
}

/// A browser command issued by an AI agent via MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCommand {
    /// Unique command ID (UUID v4)
    pub id: String,
    /// Target browser agent ID (e.g. "browser-agent-42")
    pub target_agent: String,
    /// Operation to perform
    pub command_type: BrowserCommandType,
    /// Operation parameters (tool-specific)
    pub params: Value,
    /// ISO8601 — when command was created
    pub created_at: String,
    /// ISO8601 — command expires at (default: +30s)
    pub expires_at: String,
    /// Current status
    pub status: BrowserCommandStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserCommandStatus {
    /// Waiting for browser to poll
    Pending,
    /// Browser received and is executing
    Executing,
    /// Execution complete (see BrowserResult)
    Done,
    /// Timed out or error
    Failed,
}

impl BrowserCommand {
    pub fn new(target_agent: &str, command_type: BrowserCommandType, params: Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            target_agent: target_agent.to_string(),
            command_type,
            params,
            created_at: now.to_rfc3339(),
            expires_at: (now + chrono::Duration::seconds(30)).to_rfc3339(),
            status: BrowserCommandStatus::Pending,
        }
    }

    /// Parse command type from MCP tool name (e.g. "browser_navigate" → Navigate)
    pub fn from_tool_name(tool_name: &str, target_agent: &str, params: Value) -> Option<Self> {
        let cmd_type = match tool_name {
            "browser_get_url" => BrowserCommandType::GetUrl,
            "browser_get_title" => BrowserCommandType::GetTitle,
            "browser_navigate" => BrowserCommandType::Navigate,
            "browser_get_dom" => BrowserCommandType::GetDom,
            "browser_query_selector" => BrowserCommandType::QuerySelector,
            "browser_click" => BrowserCommandType::Click,
            "browser_type" => BrowserCommandType::Type,
            "browser_scroll" => BrowserCommandType::Scroll,
            "browser_eval" => BrowserCommandType::Eval,
            "browser_screenshot" => BrowserCommandType::Screenshot,
            "browser_open_tab" => BrowserCommandType::OpenTab,
            "browser_close_tab" => BrowserCommandType::CloseTab,
            _ => return None,
        };
        Some(Self::new(target_agent, cmd_type, params))
    }
}

/// Result reported by Chrome extension after executing a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    pub command_id: String,
    pub ok: bool,
    pub data: Value,
    pub error: Option<String>,
    pub reported_at: String,
}

impl BrowserResult {
    pub fn ok(command_id: &str, data: Value) -> Self {
        Self {
            command_id: command_id.to_string(),
            ok: true,
            data,
            error: None,
            reported_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn err(command_id: &str, msg: &str) -> Self {
        Self {
            command_id: command_id.to_string(),
            ok: false,
            data: Value::Null,
            error: Some(msg.to_string()),
            reported_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Server-side queue of pending browser commands.
/// Held in trios-server, polled by Chrome extension every 2s.
#[derive(Debug, Default)]
pub struct BrowserCommandQueue {
    pending: VecDeque<BrowserCommand>,
    results: Vec<BrowserResult>,
    /// Track seen command IDs to prevent duplicate execution
    seen_ids: std::collections::HashSet<String>,
    /// Monotonic counters for observability (`/metrics`).
    enqueued_total: u64,
    polled_total: u64,
    results_total: u64,
    rejected_total: u64,
}

/// Backpressure cap: maximum number of commands in `Pending` state.
/// `try_enqueue` refuses new commands once the cap is reached so a stuck
/// or absent browser agent cannot grow the queue without bound.
pub const MAX_PENDING_COMMANDS: usize = 256;

/// Point-in-time queue observability snapshot (`/metrics`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct QueueStats {
    /// Commands waiting to be polled (`Pending`).
    pub depth: usize,
    /// Commands handed to a browser agent, result not reported yet.
    pub executing: usize,
    /// Total commands accepted since start.
    pub enqueued_total: u64,
    /// Total commands handed out by `poll` since start.
    pub polled_total: u64,
    /// Total results recorded since start.
    pub results_total: u64,
    /// Total enqueues refused by the backpressure cap.
    pub rejected_total: u64,
}

/// Error returned by [`BrowserCommandQueue::try_enqueue`] when the
/// backpressure cap is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueFull {
    /// Current number of `Pending` commands (== the cap).
    pub depth: usize,
    /// The configured cap ([`MAX_PENDING_COMMANDS`]).
    pub capacity: usize,
}

impl std::fmt::Display for QueueFull {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "browser command queue is full ({}/{} pending)",
            self.depth, self.capacity
        )
    }
}

impl std::error::Error for QueueFull {}

impl BrowserCommandQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Enqueue a new command. Returns command id.
    ///
    /// Unbounded legacy path — prefer [`Self::try_enqueue`], which applies
    /// the [`MAX_PENDING_COMMANDS`] backpressure cap.
    pub fn enqueue(&mut self, cmd: BrowserCommand) -> String {
        let id = cmd.id.clone();
        if !self.seen_ids.contains(&id) {
            self.seen_ids.insert(id.clone());
            self.pending.push_back(cmd);
            self.enqueued_total += 1;
        }
        id
    }

    /// Enqueue with backpressure: refuses the command when the number of
    /// `Pending` commands has reached [`MAX_PENDING_COMMANDS`].
    /// Duplicate ids are acknowledged (idempotent) without re-queueing.
    pub fn try_enqueue(&mut self, cmd: BrowserCommand) -> Result<String, QueueFull> {
        let id = cmd.id.clone();
        if self.seen_ids.contains(&id) {
            return Ok(id); // idempotent duplicate — already queued once
        }
        let depth = self.pending_count();
        if depth >= MAX_PENDING_COMMANDS {
            self.rejected_total += 1;
            return Err(QueueFull {
                depth,
                capacity: MAX_PENDING_COMMANDS,
            });
        }
        self.seen_ids.insert(id.clone());
        self.pending.push_back(cmd);
        self.enqueued_total += 1;
        Ok(id)
    }

    /// Poll pending commands for a specific agent (called by extension).
    /// Marks them as Executing.
    pub fn poll(&mut self, agent_id: &str) -> Vec<BrowserCommand> {
        let mut out = Vec::new();
        for cmd in self.pending.iter_mut() {
            if cmd.target_agent == agent_id && cmd.status == BrowserCommandStatus::Pending {
                cmd.status = BrowserCommandStatus::Executing;
                out.push(cmd.clone());
            }
        }
        self.polled_total += out.len() as u64;
        out
    }

    /// Record result from extension, mark command Done/Failed.
    pub fn record_result(&mut self, result: BrowserResult) {
        for cmd in self.pending.iter_mut() {
            if cmd.id == result.command_id {
                cmd.status = if result.ok {
                    BrowserCommandStatus::Done
                } else {
                    BrowserCommandStatus::Failed
                };
                break;
            }
        }
        self.results.push(result);
        self.results_total += 1;
    }

    /// Get result for a specific command (for AI agent waiting on response).
    pub fn get_result(&self, command_id: &str) -> Option<&BrowserResult> {
        self.results.iter().find(|r| r.command_id == command_id)
    }

    pub fn pending_count(&self) -> usize {
        self.pending
            .iter()
            .filter(|c| c.status == BrowserCommandStatus::Pending)
            .count()
    }

    /// Commands currently marked `Executing` (polled, result pending).
    pub fn executing_count(&self) -> usize {
        self.pending
            .iter()
            .filter(|c| c.status == BrowserCommandStatus::Executing)
            .count()
    }

    /// Observability snapshot for `/metrics`.
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            depth: self.pending_count(),
            executing: self.executing_count(),
            enqueued_total: self.enqueued_total,
            polled_total: self.polled_total,
            results_total: self.results_total,
            rejected_total: self.rejected_total,
        }
    }
}

/// MCP tool definitions for browser control.
/// Register these in trios-server's MCP service alongside SR-02 tools.
pub fn mcp_browser_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "browser_get_url",
            "description": "Get current URL of the browser tab controlled by BrowserOS agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string", "description": "Target browser agent ID, e.g. browser-agent-42"}
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "browser_get_title",
            "description": "Get page title of the browser tab",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"}
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "browser_navigate",
            "description": "Navigate the browser tab to a URL",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "url": {"type": "string", "description": "URL to navigate to"}
                },
                "required": ["agent_id", "url"]
            }
        }),
        json!({
            "name": "browser_get_dom",
            "description": "Get full page HTML of the browser tab",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"}
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "browser_query_selector",
            "description": "Find a DOM element by CSS selector, returns outer HTML",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "selector": {"type": "string", "description": "CSS selector"}
                },
                "required": ["agent_id", "selector"]
            }
        }),
        json!({
            "name": "browser_click",
            "description": "Click a DOM element by CSS selector",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "selector": {"type": "string"}
                },
                "required": ["agent_id", "selector"]
            }
        }),
        json!({
            "name": "browser_type",
            "description": "Type text into an input or textarea element",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "selector": {"type": "string"},
                    "text": {"type": "string"}
                },
                "required": ["agent_id", "selector", "text"]
            }
        }),
        json!({
            "name": "browser_scroll",
            "description": "Scroll the browser tab to coordinates",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "x": {"type": "number", "default": 0},
                    "y": {"type": "number"}
                },
                "required": ["agent_id", "y"]
            }
        }),
        json!({
            "name": "browser_eval",
            "description": "Evaluate a JavaScript expression in the browser tab (sandboxed via new Function)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "js": {"type": "string", "description": "JS expression to evaluate"}
                },
                "required": ["agent_id", "js"]
            }
        }),
        json!({
            "name": "browser_screenshot",
            "description": "Capture a screenshot of the browser tab as base64 PNG",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"}
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "browser_open_tab",
            "description": "Open a new browser tab with given URL",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"},
                    "url": {"type": "string"}
                },
                "required": ["agent_id", "url"]
            }
        }),
        json!({
            "name": "browser_close_tab",
            "description": "Close the current browser tab",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": {"type": "string"}
                },
                "required": ["agent_id"]
            }
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_creation() {
        let cmd = BrowserCommand::new(
            "browser-agent-42",
            BrowserCommandType::Navigate,
            json!({"url": "https://github.com"}),
        );
        assert_eq!(cmd.target_agent, "browser-agent-42");
        assert_eq!(cmd.command_type, BrowserCommandType::Navigate);
        assert_eq!(cmd.status, BrowserCommandStatus::Pending);
    }

    #[test]
    fn test_from_tool_name() {
        let cmd = BrowserCommand::from_tool_name(
            "browser_navigate",
            "browser-agent-1",
            json!({"url": "https://example.com"}),
        );
        assert!(cmd.is_some());
        assert_eq!(cmd.unwrap().command_type, BrowserCommandType::Navigate);

        let unknown = BrowserCommand::from_tool_name("unknown_tool", "agent", json!({}));
        assert!(unknown.is_none());
    }

    #[test]
    fn test_queue_enqueue_poll() {
        let mut q = BrowserCommandQueue::new();
        let cmd = BrowserCommand::new("browser-agent-1", BrowserCommandType::GetUrl, json!({}));
        let id = q.enqueue(cmd);
        assert_eq!(q.pending_count(), 1);

        let polled = q.poll("browser-agent-1");
        assert_eq!(polled.len(), 1);
        assert_eq!(polled[0].id, id);
        assert_eq!(q.pending_count(), 0); // now Executing
    }

    #[test]
    fn test_queue_deduplication() {
        let mut q = BrowserCommandQueue::new();
        let cmd = BrowserCommand::new("browser-agent-1", BrowserCommandType::GetUrl, json!({}));
        let id = cmd.id.clone();
        q.enqueue(cmd.clone());
        q.enqueue(cmd); // duplicate
        assert_eq!(q.pending_count(), 1); // only 1
        drop(id);
    }

    #[test]
    fn test_result_recording() {
        let mut q = BrowserCommandQueue::new();
        let cmd = BrowserCommand::new("browser-agent-1", BrowserCommandType::GetTitle, json!({}));
        let id = q.enqueue(cmd);
        q.poll("browser-agent-1");

        let result = BrowserResult::ok(&id, json!({"title": "GitHub"}));
        q.record_result(result);

        let r = q.get_result(&id).unwrap();
        assert!(r.ok);
        assert_eq!(r.data["title"], "GitHub");
    }

    #[test]
    fn test_mcp_tool_definitions() {
        let tools = mcp_browser_tool_definitions();
        assert_eq!(tools.len(), 12);
        let names: Vec<&str> = tools.iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"browser_navigate"));
        assert!(names.contains(&"browser_eval"));
        assert!(names.contains(&"browser_screenshot"));
    }

    #[test]
    fn test_try_enqueue_rejects_at_capacity() {
        let mut q = BrowserCommandQueue::new();
        for _ in 0..MAX_PENDING_COMMANDS {
            let cmd = BrowserCommand::new("agent-x", BrowserCommandType::GetUrl, json!({}));
            q.try_enqueue(cmd).expect("below cap must be accepted");
        }
        assert_eq!(q.pending_count(), MAX_PENDING_COMMANDS);

        let overflow = BrowserCommand::new("agent-x", BrowserCommandType::GetUrl, json!({}));
        let err = q.try_enqueue(overflow).expect_err("cap reached must reject");
        assert_eq!(err.capacity, MAX_PENDING_COMMANDS);
        assert_eq!(err.depth, MAX_PENDING_COMMANDS);
        assert_eq!(q.stats().rejected_total, 1);
        // Queue did not grow past the cap.
        assert_eq!(q.pending_count(), MAX_PENDING_COMMANDS);
    }

    #[test]
    fn test_try_enqueue_duplicate_is_idempotent() {
        let mut q = BrowserCommandQueue::new();
        let cmd = BrowserCommand::new("agent-x", BrowserCommandType::GetUrl, json!({}));
        let dup = cmd.clone();
        let id = q.try_enqueue(cmd).unwrap();
        let id2 = q.try_enqueue(dup).unwrap();
        assert_eq!(id, id2);
        assert_eq!(q.pending_count(), 1);
        assert_eq!(q.stats().enqueued_total, 1);
    }

    #[test]
    fn test_stats_counters_track_lifecycle() {
        let mut q = BrowserCommandQueue::new();
        let cmd = BrowserCommand::new("agent-x", BrowserCommandType::GetTitle, json!({}));
        let id = q.try_enqueue(cmd).unwrap();
        assert_eq!(q.stats().depth, 1);
        assert_eq!(q.stats().executing, 0);

        let polled = q.poll("agent-x");
        assert_eq!(polled.len(), 1);
        let s = q.stats();
        assert_eq!(s.depth, 0);
        assert_eq!(s.executing, 1);
        assert_eq!(s.polled_total, 1);

        q.record_result(BrowserResult::ok(&id, json!({"title": "t"})));
        let s = q.stats();
        assert_eq!(s.executing, 0);
        assert_eq!(s.results_total, 1);
        assert_eq!(s.enqueued_total, 1);
        assert_eq!(s.rejected_total, 0);
    }
}
