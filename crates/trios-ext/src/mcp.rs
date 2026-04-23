use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

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
    ws: Option<WebSocket>,
    next_id: u64,
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
            let _ = crate::dom::set_status("Connected to trios-server");
        });

        let onerror_closure: Closure<dyn Fn()> = Closure::new(|| {
            let _ = crate::dom::set_status("Connection failed — is trios-server running?");
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

fn handle_mcp_response(val: &serde_json::Value) {
    let method = val.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "agents/chat" | "chat" => {
            if let Some(result) = val.get("result") {
                if let Some(response) = result.get("response").and_then(|r| r.as_str()) {
                    crate::dom::append_message("agent", response);
                } else if let Some(response) = result.get("content").and_then(|r| r.as_str()) {
                    crate::dom::append_message("agent", response);
                } else {
                    crate::dom::append_message(
                        "agent",
                        &serde_json::to_string(result).unwrap_or_default(),
                    );
                }
            } else if let Some(error) = val.get("error") {
                let msg = error
                    .get("message")
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown error");
                crate::dom::append_message("error", msg);
            }
        }
        "agents/list" => {
            if let Some(result) = val.get("result") {
                let text = if result.is_array() && result.as_array().unwrap().is_empty() {
                    "No agents connected.".to_string()
                } else {
                    serde_json::to_string_pretty(result).unwrap_or_default()
                };
                crate::dom::set_agent_list(&text);
            }
        }
        "tools/list" => {
            if let Some(result) = val.get("result") {
                let text = serde_json::to_string_pretty(result).unwrap_or_default();
                crate::dom::set_tool_list(&text);
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
