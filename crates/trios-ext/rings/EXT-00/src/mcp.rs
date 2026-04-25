//! EXT-00/mcp — MCP client (HTTP transport)
//!
//! JSON-RPC over HTTP to trios-server, plus direct z.ai API calls.

use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

pub const MCP_HTTP_URL: &str = "http://127.0.0.1:9105/mcp";
pub const CHAT_HTTP_URL: &str = "http://127.0.0.1:9105/chat";
/// z.ai Anthropic-compatible Messages API
pub const ZAI_CHAT_URL: &str = "https://api.z.ai/api/anthropic/v1/messages";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

pub struct McpClient { pub next_id: u64 }
impl Default for McpClient { fn default() -> Self { Self::new() } }
impl McpClient {
    pub fn new() -> Self { Self { next_id: 1 } }
    fn alloc_id(&mut self) -> u64 { let id = self.next_id; self.next_id += 1; id }
    pub fn send_request(&mut self, method: &str, params: Option<serde_json::Value>) -> Result<(), JsValue> {
        let id = self.alloc_id();
        let req = McpRequest { jsonrpc: "2.0".into(), id, method: method.into(), params };
        let body = serde_json::to_string(&req).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = MCP_HTTP_URL.to_string();
        let method_label = method.to_string();
        METHOD_MAP.with(|m| m.borrow_mut().insert(id, method_label.clone()));
        spawn_local(async move {
            match http_post(&url, &body, None).await {
                Ok(text) if !text.is_empty() => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) { handle_mcp_response(&val); }
                }
                Err(e) => { let _ = super::dom::set_status(&format!("HTTP error: {:?}", e)); }
                _ => {}
            }
        });
        Ok(())
    }
    pub fn send_chat(&mut self, msg: &str) -> Result<(), JsValue> {
        // If we have a z.ai API key, call z.ai directly (bypass BrowserOS)
        if let Some(key) = trios_ext_02::get_api_key() {
            return self.send_chat_direct(&key, msg);
        }
        // Fallback: route through BrowserOS server
        self.send_chat_via_server(msg)
    }
    /// Direct z.ai API call (Anthropic-compatible /api/anthropic/v1/messages)
    fn send_chat_direct(&self, api_key: &str, msg: &str) -> Result<(), JsValue> {
        let body = serde_json::to_string(&json!({
            "model": "glm-5.1",
            "messages": [{"role": "user", "content": msg}],
            "max_tokens": 4096
        })).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = ZAI_CHAT_URL.to_string();
        let key = api_key.to_string();
        let msg_display = msg.to_string();
        spawn_local(async move {
            super::dom::append_message("user", &msg_display);
            match http_post_zai(&url, &body, &key).await {
                Ok(text) if !text.is_empty() => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
                            let text_parts: Vec<&str> = content.iter()
                                .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
                                .collect();
                            if text_parts.is_empty() {
                                super::dom::append_message("agent", &text);
                            } else {
                                super::dom::append_message("agent", &text_parts.join("\n"));
                            }
                        } else if let Some(err) = val.get("error") {
                            let err_msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("API error");
                            super::dom::append_message("error", err_msg);
                        } else {
                            super::dom::append_message("agent", &text);
                        }
                    } else {
                        super::dom::append_message("agent", &text);
                    }
                }
                Err(e) => { super::dom::append_message("error", &format!("z.ai error: {:?}", e)); }
                _ => { super::dom::append_message("error", "Empty response from z.ai"); }
            }
        });
        Ok(())
    }
    /// Fallback: route chat through BrowserOS server
    fn send_chat_via_server(&self, msg: &str) -> Result<(), JsValue> {
        let body = serde_json::to_string(&json!({
            "conversationId": "00000000-0000-0000-0000-000000000001",
            "message": msg,
            "model": "glm-5.1",
            "provider": "zai",
            "mode": "chat",
            "origin": "sidepanel"
        })).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = CHAT_HTTP_URL.to_string();
        spawn_local(async move {
            match http_post(&url, &body, None).await {
                Ok(text) if !text.is_empty() => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(err) = val.get("error") {
                            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Error");
                            super::dom::append_message("error", msg);
                        } else {
                            let fallback = serde_json::to_string_pretty(&val).unwrap_or_default();
                            super::dom::append_message("agent", &fallback);
                        }
                    } else {
                        super::dom::append_message("agent", &text);
                    }
                }
                Err(e) => { super::dom::append_message("error", &format!("Chat error: {:?}", e)); }
                _ => { super::dom::append_message("error", "Empty response"); }
            }
        });
        Ok(())
    }
    pub fn list_agents(&mut self) -> Result<(), JsValue> { self.send_request("agents/list", None) }
    pub fn list_tools(&mut self) -> Result<(), JsValue> { self.send_request("tools/list", None) }
    pub fn list_issues(&mut self) -> Result<(), JsValue> { self.send_request("issues/list", None) }
    pub fn ping(&mut self) -> Result<(), JsValue> { self.send_request("ping", None) }
}

/// HTTP POST with optional Bearer token. Uses Promise.race for 10s timeout.
async fn http_post(url: &str, body: &str, bearer: Option<&str>) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(body));
    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let h = request.headers();
    h.set("Content-Type", "application/json")?;
    h.set("Accept", "application/json, text/event-stream")?;
    if let Some(token) = bearer {
        h.set("Authorization", &format!("Bearer {token}"))?;
    }
    let fetch_promise = window.fetch_with_request(&request);
    fetch_with_timeout(fetch_promise, 10000).await
}

/// HTTP POST to z.ai with x-api-key header (Anthropic-compatible).
async fn http_post_zai(url: &str, body: &str, api_key: &str) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(body));
    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let h = request.headers();
    h.set("Content-Type", "application/json")?;
    h.set("x-api-key", api_key)?;
    h.set("anthropic-version", "2023-06-01")?;
    let fetch_promise = window.fetch_with_request(&request);
    fetch_with_timeout(fetch_promise, 30000).await
}

/// Race a fetch Promise against a timeout.
async fn fetch_with_timeout(fetch_promise: js_sys::Promise, timeout_ms: i32) -> Result<String, JsValue> {
    let timeout_promise = js_sys::Promise::new(&mut |resolve, _| {
        let w = web_sys::window().unwrap();
        let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, timeout_ms);
    });
    let race = js_sys::Promise::race(&js_sys::Array::of2(&timeout_promise, &fetch_promise));
    let resp_val = wasm_bindgen_futures::JsFuture::from(race).await
        .map_err(|_| JsValue::from_str(&format!("Request timeout ({}s)", timeout_ms / 1000)))?;
    let response: web_sys::Response = resp_val.dyn_into()
        .map_err(|_| JsValue::from_str("Not a response (timeout)"))?;
    let text_val = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    text_val.as_string().ok_or_else(|| JsValue::from_str("not a string"))
}

fn handle_mcp_response(val: &serde_json::Value) {
    let id = val.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
    let method = METHOD_MAP.with(|m| m.borrow_mut().remove(&id)).unwrap_or_default();
    match method.as_str() {
        "initialize" => { let _ = super::dom::set_status("Connected to trios-server (HTTP)"); }
        "agents/chat" | "chat" => {
            if let Some(r) = val.get("result") {
                let txt = r.get("response").and_then(|v| v.as_str())
                    .or_else(|| r.get("content").and_then(|v| v.as_str()));
                let fallback = serde_json::to_string(r).unwrap_or_default();
                super::dom::append_message("agent", txt.unwrap_or(&fallback));
            } else if let Some(e) = val.get("error") {
                super::dom::append_message("error", e.get("message").and_then(|m| m.as_str()).unwrap_or("Error"));
            }
        }
        "agents/list" => {
            if let Some(r) = val.get("result") {
                if r.is_null() || !r.is_object() {
                    super::dom::set_agent_list("No agents available");
                } else {
                    super::dom::set_agent_list(&serde_json::to_string_pretty(r).unwrap_or_default());
                }
            } else if let Some(e) = val.get("error") {
                let msg = e.get("message").and_then(|m| m.as_str()).unwrap_or("Error");
                super::dom::set_agent_list(&format!("Error: {}", msg));
            } else {
                super::dom::set_agent_list("No data available");
            }
        }
        "tools/list" => {
            if let Some(r) = val.get("result") {
                if r.is_null() || !r.is_object() {
                    super::dom::set_tool_list("No tools available");
                } else {
                    super::dom::set_tool_list(&serde_json::to_string(r).unwrap_or_default());
                }
            } else if let Some(e) = val.get("error") {
                let msg = e.get("message").and_then(|m| m.as_str()).unwrap_or("Error");
                super::dom::set_tool_list(&format!("Error: {}", msg));
            } else {
                super::dom::set_tool_list("No data available");
            }
        }
        "issues/list" => {
            if let Some(r) = val.get("result") {
                if r.is_null() || !r.is_object() {
                    super::dom::set_issue_list("[]");
                } else if let Some(arr) = r.as_array() {
                    super::dom::set_issue_list(&serde_json::to_string(arr).unwrap_or_default());
                } else {
                    super::dom::set_issue_list(&serde_json::to_string_pretty(r).unwrap_or_default());
                }
            } else if let Some(e) = val.get("error") {
                let msg = e.get("message").and_then(|m| m.as_str()).unwrap_or("Error");
                super::dom::set_issue_list("[]");
                let _ = super::dom::set_status(&format!("Issues error: {}", msg));
            } else {
                super::dom::set_issue_list("[]");
            }
        }
        "ping" => { let _ = super::dom::set_status("pong"); }
        _ => {}
    }
}

thread_local! {
    static CLIENT: std::cell::RefCell<McpClient> = std::cell::RefCell::new(McpClient::new());
    static METHOD_MAP: std::cell::RefCell<std::collections::HashMap<u64, String>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

#[wasm_bindgen] pub fn mcp_connect() -> Result<(), JsValue> {
    let _ = super::dom::set_status("Connecting...");
    CLIENT.with(|c| c.borrow_mut().send_request("initialize", Some(json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"trios-ext","version":"0.3.0"}}))))
}
#[wasm_bindgen] pub fn mcp_send_chat(m: &str) -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().send_chat(m)) }
#[wasm_bindgen] pub fn mcp_list_agents() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().list_agents()) }
#[wasm_bindgen] pub fn mcp_list_tools() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().list_tools()) }
#[wasm_bindgen] pub fn mcp_list_issues() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().list_issues()) }
#[wasm_bindgen] pub fn mcp_ping() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().ping()) }
#[wasm_bindgen] pub fn mcp_is_connected() -> bool { true }
