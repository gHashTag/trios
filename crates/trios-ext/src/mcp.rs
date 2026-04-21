use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

pub const MCP_HTTP_URL: &str = "http://127.0.0.1:9105/mcp";
pub const CHAT_HTTP_URL: &str = "http://127.0.0.1:9105/chat";

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
            match http_post(&url, &body).await {
                Ok(text) if !text.is_empty() => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) { handle_mcp_response(&val); }
                }
                Err(e) => { let _ = crate::dom::set_status(&format!("HTTP error: {:?}", e)); }
                _ => {}
            }
        });
        Ok(())
    }
    pub fn send_chat(&mut self, msg: &str) -> Result<(), JsValue> {
        let body = serde_json::to_string(&json!({
            "conversationId": "00000000-0000-0000-0000-000000000001",
            "message": msg,
            "model": "gpt-4o",
            "provider": "openai",
            "mode": "chat",
            "origin": "sidepanel"
        })).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let url = CHAT_HTTP_URL.to_string();
        spawn_local(async move {
            match http_post(&url, &body).await {
                Ok(text) if !text.is_empty() => {
                    // Chat endpoint returns streaming text or JSON
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(err) = val.get("error") {
                            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("Error");
                            crate::dom::append_message("error", msg);
                        } else {
                            let fallback = serde_json::to_string_pretty(&val).unwrap_or_default();
                            crate::dom::append_message("agent", &fallback);
                        }
                    } else {
                        // Plain text response (streaming)
                        crate::dom::append_message("agent", &text);
                    }
                }
                Err(e) => { crate::dom::append_message("error", &format!("Chat error: {:?}", e)); }
                _ => { crate::dom::append_message("error", "Empty response"); }
            }
        });
        Ok(())
    }
    pub fn list_agents(&mut self) -> Result<(), JsValue> { self.send_request("agents/list", None) }
    pub fn list_tools(&mut self) -> Result<(), JsValue> { self.send_request("tools/list", None) }
    pub fn ping(&mut self) -> Result<(), JsValue> { self.send_request("ping", None) }
}

async fn http_post(url: &str, body: &str) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(body));
    let request = web_sys::Request::new_with_str_and_init(url, &opts)?;
    let h = request.headers();
    h.set("Content-Type", "application/json")?;
    h.set("Accept", "application/json, text/event-stream")?;
    let resp_val = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request)).await?;
    let response: web_sys::Response = resp_val.dyn_into()?;
    let text_val = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    text_val.as_string().ok_or_else(|| JsValue::from_str("not a string"))
}

fn handle_mcp_response(val: &serde_json::Value) {
    let id = val.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
    let method = METHOD_MAP.with(|m| m.borrow_mut().remove(&id)).unwrap_or_default();
    match method.as_str() {
        "initialize" => { let _ = crate::dom::set_status("Connected to trios-server (HTTP)"); }
        "agents/chat" | "chat" => {
            if let Some(r) = val.get("result") {
                let txt = r.get("response").and_then(|v| v.as_str())
                    .or_else(|| r.get("content").and_then(|v| v.as_str()));
                let fallback = serde_json::to_string(r).unwrap_or_default();
                crate::dom::append_message("agent", txt.unwrap_or(&fallback));
            } else if let Some(e) = val.get("error") {
                crate::dom::append_message("error", e.get("message").and_then(|m| m.as_str()).unwrap_or("Error"));
            }
        }
        "agents/list" => { if let Some(r) = val.get("result") { crate::dom::set_agent_list(&serde_json::to_string_pretty(r).unwrap_or_default()); } }
        "tools/list" => { if let Some(r) = val.get("result") { crate::dom::set_tool_list(&serde_json::to_string_pretty(r).unwrap_or_default()); } }
        "ping" => { let _ = crate::dom::set_status("pong"); }
        _ => {}
    }
}

thread_local! {
    static CLIENT: std::cell::RefCell<McpClient> = std::cell::RefCell::new(McpClient::new());
    static METHOD_MAP: std::cell::RefCell<std::collections::HashMap<u64, String>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

#[wasm_bindgen] pub fn mcp_connect() -> Result<(), JsValue> {
    let _ = crate::dom::set_status("Connecting...");
    CLIENT.with(|c| c.borrow_mut().send_request("initialize", Some(json!({"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"trios-ext","version":"0.3.0"}}))))
}
#[wasm_bindgen] pub fn mcp_send_chat(m: &str) -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().send_chat(m)) }
#[wasm_bindgen] pub fn mcp_list_agents() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().list_agents()) }
#[wasm_bindgen] pub fn mcp_list_tools() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().list_tools()) }
#[wasm_bindgen] pub fn mcp_ping() -> Result<(), JsValue> { CLIENT.with(|c| c.borrow_mut().ping()) }
#[wasm_bindgen] pub fn mcp_is_connected() -> bool { true }
