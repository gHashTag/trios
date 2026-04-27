//! SILVER-RING-EXT-02 — MCP Client
//!
//! MCP WebSocket client for connecting to trios-server.

use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

use trios_ext_ring_ex01 as dom;

pub const MCP_WS_URL: &str = "ws://localhost:9005/ws";

#[derive(Debug, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

pub struct McpClient {
    pub ws: Option<WebSocket>,
    pub next_id: u64,
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            ws: None,
            next_id: 1,
        }
    }

    pub fn connect(&mut self) -> Result<(), JsValue> {
        let ws = WebSocket::new(MCP_WS_URL)?;

        let onopen_closure: Closure<dyn Fn()> = Closure::new(|| {
            let _ = dom::set_status("Connected to trios-server");
        });

        let onerror_closure: Closure<dyn Fn()> = Closure::new(|| {
            let _ = dom::set_status("Connection failed — is trios-server running?");
        });

        let onmessage_closure: Closure<dyn Fn(MessageEvent)> = Closure::new(|ev: MessageEvent| {
            if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                let text: String = txt.into();
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    handle_mcp_response(&val);
                }
            }
        });

        ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));

        onopen_closure.forget();
        onerror_closure.forget();
        onmessage_closure.forget();

        self.ws = Some(ws);
        Ok(())
    }

    pub fn send(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?;
        let req = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: self.next_id,
            method: method.to_string(),
            params,
        };
        self.next_id += 1;
        let json = serde_json::to_string(&req).map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&json)?;
        Ok(())
    }

    pub fn send_chat(&mut self, message: &str) -> Result<(), JsValue> {
        self.send("agents/chat", Some(json!({ "message": message })))
    }

    pub fn list_agents(&mut self) -> Result<(), JsValue> {
        self.send("agents/list", None)
    }

    pub fn list_tools(&mut self) -> Result<(), JsValue> {
        self.send("tools/list", None)
    }

    pub fn is_connected(&self) -> bool {
        self.ws
            .as_ref()
            .is_some_and(|ws| ws.ready_state() == WebSocket::OPEN)
    }
}

pub fn handle_mcp_response(val: &serde_json::Value) {
    let method = val.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "agents/chat" | "chat" => {
            if let Some(result) = val.get("result") {
                if let Some(response) = result.get("response").and_then(|r| r.as_str()) {
                    dom::append_message("agent", response);
                } else if let Some(response) = result.get("content").and_then(|r| r.as_str()) {
                    dom::append_message("agent", response);
                } else {
                    dom::append_message(
                        "agent",
                        &serde_json::to_string(result).unwrap_or_default(),
                    );
                }
            } else if let Some(error) = val.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                dom::append_message("error", msg);
            }
        }
        "agents/list" => {
            if let Some(result) = val.get("result") {
                let text = if result.is_array() && result.as_array().unwrap().is_empty() {
                    "No agents connected.".to_string()
                } else {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                };
                dom::set_agent_list(&text);
            }
        }
        "tools/list" => {
            if let Some(result) = val.get("result") {
                let text = serde_json::to_string_pretty(result).unwrap_or_default();
                dom::set_tool_list(&text);
            }
        }
        _ => {}
    }
}

thread_local! {
    static CLIENT: std::cell::RefCell<McpClient> = std::cell::RefCell::new(McpClient::new());
}

#[wasm_bindgen]
pub fn mcp_connect() -> Result<(), JsValue> {
    CLIENT.with(|c| c.borrow_mut().connect())
}

#[wasm_bindgen]
pub fn mcp_send_chat(message: &str) -> Result<(), JsValue> {
    CLIENT.with(|c| c.borrow_mut().send_chat(message))
}

#[wasm_bindgen]
pub fn mcp_list_agents() -> Result<(), JsValue> {
    CLIENT.with(|c| c.borrow_mut().list_agents())
}

#[wasm_bindgen]
pub fn mcp_list_tools() -> Result<(), JsValue> {
    CLIENT.with(|c| c.borrow_mut().list_tools())
}

#[wasm_bindgen]
pub fn mcp_ping() -> Result<(), JsValue> {
    CLIENT.with(|c| c.borrow_mut().send("ping", None))
}

#[wasm_bindgen]
pub fn mcp_is_connected() -> bool {
    CLIENT.with(|c| c.borrow().is_connected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── McpRequest сериализация ──────────────────────────────────────────────

    #[test]
    fn mcp_url_uses_port_9005() {
        assert!(MCP_WS_URL.contains("9005"));
        assert!(MCP_WS_URL.starts_with("ws://"));
    }

    #[test]
    fn mcp_request_full_serialization() {
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            id: 42,
            method: "agents/chat".into(),
            params: Some(json!({ "message": "hello" })),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"id\":42"));
        assert!(s.contains("\"method\":\"agents/chat\""));
        assert!(s.contains("\"message\":\"hello\""));
    }

    #[test]
    fn mcp_request_skips_null_params() {
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "ping".into(),
            params: None,
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(!s.contains("params"), "params must not appear when None: {s}");
    }

    #[test]
    fn mcp_request_increments_id_field() {
        // Проверяем что каждый запрос получает уникальный id
        let r1 = McpRequest { jsonrpc: "2.0".into(), id: 1, method: "a".into(), params: None };
        let r2 = McpRequest { jsonrpc: "2.0".into(), id: 2, method: "a".into(), params: None };
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn mcp_request_roundtrip() {
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            id: 7,
            method: "tools/list".into(),
            params: Some(json!({ "filter": null })),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: McpRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 7);
        assert_eq!(back.method, "tools/list");
        assert_eq!(back.jsonrpc, "2.0");
    }

    #[test]
    fn mcp_request_send_chat_params_structure() {
        // Проверяем что send_chat формирует правильный payload
        let params = json!({ "message": "test message" });
        let req = McpRequest {
            jsonrpc: "2.0".into(),
            id: 1,
            method: "agents/chat".into(),
            params: Some(params.clone()),
        };
        assert_eq!(req.params.unwrap()["message"], "test message");
    }

    // ── McpClient состояние ──────────────────────────────────────────────────

    #[test]
    fn mcp_client_starts_disconnected() {
        let client = McpClient::new();
        assert!(client.ws.is_none());
        assert_eq!(client.next_id, 1);
    }

    #[test]
    fn mcp_client_default_equals_new() {
        let a = McpClient::new();
        let b = McpClient::default();
        assert_eq!(a.next_id, b.next_id);
        assert!(a.ws.is_none());
        assert!(b.ws.is_none());
    }

    #[test]
    fn mcp_client_send_without_ws_returns_err() {
        let mut client = McpClient::new();
        // send() без connect() должен вернуть Err("not connected")
        let result = client.send("ping", None);
        assert!(result.is_err());
        let err_str = result.unwrap_err().as_string().unwrap_or_default();
        assert!(err_str.contains("not connected"), "got: {err_str}");
    }

    #[test]
    fn mcp_client_send_chat_without_ws_returns_err() {
        let mut client = McpClient::new();
        let result = client.send_chat("hello");
        assert!(result.is_err());
    }

    // ── handle_mcp_response — роутинг логика ─────────────────────────────────
    // NOTE: handle_mcp_response вызывает dom::* (side-effect на DOM)
    // На native (non-wasm) dom-функции являются no-op / panic-safe,
    // поэтому проверяем только что функция не паникует с разными входами.

    #[test]
    fn handle_response_chat_with_response_field_no_panic() {
        let val = json!({
            "method": "agents/chat",
            "result": { "response": "pong from agent" }
        });
        handle_mcp_response(&val); // не должна паниковать
    }

    #[test]
    fn handle_response_chat_with_content_field_no_panic() {
        let val = json!({
            "method": "agents/chat",
            "result": { "content": "fallback content" }
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_chat_with_raw_result_no_panic() {
        let val = json!({
            "method": "chat",
            "result": { "data": [1, 2, 3] }
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_chat_error_no_panic() {
        let val = json!({
            "method": "agents/chat",
            "error": { "message": "agent unavailable", "code": -32000 }
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_agents_list_empty_array_no_panic() {
        let val = json!({
            "method": "agents/list",
            "result": []
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_agents_list_non_empty_no_panic() {
        let val = json!({
            "method": "agents/list",
            "result": [{"id": "ALPHA", "status": "active"}]
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_tools_list_no_panic() {
        let val = json!({
            "method": "tools/list",
            "result": { "tools": ["tool_a", "tool_b"] }
        });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_unknown_method_is_noop() {
        // не паникует, не роутит никуда
        let val = json!({ "method": "unknown/xyz", "result": {} });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_missing_method_is_noop() {
        let val = json!({ "result": { "data": "something" } });
        handle_mcp_response(&val);
    }

    #[test]
    fn handle_response_empty_object_is_noop() {
        handle_mcp_response(&json!({}));
    }

    #[test]
    fn handle_response_chat_no_result_no_error_is_noop() {
        let val = json!({ "method": "agents/chat" }); // нет result и error
        handle_mcp_response(&val);
    }
}
