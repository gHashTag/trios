use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use crate::ws_handler::AppState;
use trios_a2a::{BrowserCommand, BrowserCommandQueue, BrowserResult};

pub struct BrowserState {
    pub queue: Mutex<BrowserCommandQueue>,
    /// Last time each host agent polled the queue — lets `/chat` auto-wire
    /// browser tools to whichever agent is actually connected (Wave 16).
    pollers: Mutex<std::collections::HashMap<String, std::time::Instant>>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(BrowserCommandQueue::new()),
            pollers: Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// The most recently polling agent within `window`, if any.
    pub async fn active_agent(&self, window: std::time::Duration) -> Option<String> {
        let now = std::time::Instant::now();
        let pollers = self.pollers.lock().await;
        pollers
            .iter()
            .filter(|(_, at)| now.duration_since(**at) <= window)
            .max_by_key(|(_, at)| **at)
            .map(|(id, _)| id.clone())
    }

    async fn record_poll(&self, agent_id: &str) {
        let mut pollers = self.pollers.lock().await;
        pollers.insert(agent_id.to_string(), std::time::Instant::now());
        // Keep the map tiny — drop entries stale for over an hour.
        pollers.retain(|_, at| at.elapsed() < std::time::Duration::from_secs(3600));
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ReportBody {
    pub command_id: String,
    pub agent_id: String,
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

pub async fn browser_commands(state: &AppState, agent_id: &str) -> Value {
    if !agent_id.is_empty() {
        state.browser.record_poll(agent_id).await;
    }
    let mut queue = state.browser.queue.lock().await;
    let commands = queue.poll(agent_id);
    json!({"commands": commands})
}

pub async fn browser_result(state: &AppState, body: Value) -> Value {
    let command_id = body.get("command_id").and_then(|v| v.as_str()).unwrap_or("");
    let ok = body.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let data = body.get("result").cloned().unwrap_or(json!(null));
    let error = body.get("error").and_then(|v| v.as_str()).map(String::from);

    if command_id.is_empty() {
        return json!({"error": "command_id is required"});
    }

    let result = BrowserResult {
        command_id: command_id.to_string(),
        ok,
        data,
        error,
        reported_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut queue = state.browser.queue.lock().await;
    queue.record_result(result);

    json!({"ok": true, "command_id": command_id})
}

pub async fn enqueue_command(state: &AppState, params: Value) -> Value {
    let tool_name = params.get("tool").and_then(|v| v.as_str()).unwrap_or("");
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    let tool_params = params.get("params").cloned().unwrap_or(json!({}));

    if agent_id.is_empty() {
        return json!({"error": "agent_id is required"});
    }

    let cmd = match BrowserCommand::from_tool_name(tool_name, agent_id, tool_params) {
        Some(c) => c,
        None => return json!({"error": format!("unknown browser tool: {}", tool_name)}),
    };

    let mut queue = state.browser.queue.lock().await;
    match queue.try_enqueue(cmd) {
        Ok(command_id) => json!({"queued": true, "command_id": command_id}),
        Err(full) => json!({
            "error": full.to_string(),
            "queue_full": true,
            "depth": full.depth,
            "capacity": full.capacity,
        }),
    }
}

/// Observability snapshot of the browser command queue (for `/metrics`).
pub async fn queue_stats(state: &AppState) -> trios_a2a::QueueStats {
    state.browser.queue.lock().await.stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_command_flow() {
        let state = AppState::new();

        let tool_result = enqueue_command(
            &state,
            json!({
                "tool": "browser_navigate",
                "agent_id": "browser-agent-42",
                "params": {"agent_id": "browser-agent-42", "url": "https://github.com"}
            }),
        )
        .await;
        assert_eq!(tool_result["queued"], true);
        let command_id = tool_result["command_id"].as_str().unwrap().to_string();

        let polled = browser_commands(&state, "browser-agent-42").await;
        let commands = polled["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0]["id"], command_id);

        let result = browser_result(
            &state,
            json!({
                "command_id": command_id,
                "success": true,
                "result": {"final_url": "https://github.com"}
            }),
        )
        .await;
        assert_eq!(result["ok"], true);

        let polled_again = browser_commands(&state, "browser-agent-42").await;
        assert_eq!(polled_again["commands"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_browser_tool_missing_agent_id() {
        let state = AppState::new();
        let result = enqueue_command(
            &state,
            json!({"tool": "browser_navigate", "agent_id": "", "params": {}}),
        )
        .await;
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_browser_result_missing_command_id() {
        let state = AppState::new();
        let result = browser_result(&state, json!({"success": true})).await;
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_browser_unknown_tool() {
        let state = AppState::new();
        let result = enqueue_command(
            &state,
            json!({"tool": "browser_nonexistent", "agent_id": "a1", "params": {}}),
        )
        .await;
        assert!(result.get("error").is_some());
    }

    #[tokio::test]
    async fn test_browser_poll_empty_for_other_agent() {
        let state = AppState::new();
        enqueue_command(
            &state,
            json!({"tool": "browser_navigate", "agent_id": "agent-A", "params": {"url": "https://a.com"}}),
        )
        .await;

        let polled = browser_commands(&state, "agent-B").await;
        assert_eq!(polled["commands"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_browser_result_recording_failure() {
        let state = AppState::new();
        let tool_result = enqueue_command(
            &state,
            json!({"tool": "browser_get_url", "agent_id": "agent-X", "params": {}}),
        )
        .await;
        let command_id = tool_result["command_id"].as_str().unwrap().to_string();

        let result = browser_result(
            &state,
            json!({
                "command_id": command_id,
                "success": false,
                "error": "tab not found"
            }),
        )
        .await;
        assert_eq!(result["ok"], true);

        let queue = state.browser.queue.lock().await;
        let r = queue.get_result(&command_id).unwrap();
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("tab not found"));
    }
}
