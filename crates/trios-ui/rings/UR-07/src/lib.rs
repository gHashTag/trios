//! UR-07 — WebSocket API Client
//!
//! Connects to trios-server via WebSocket on port 9005.
//! Provides send/receive capabilities for the sidebar UI.

use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

pub const SERVER_WS_URL: &str = "ws://localhost:9005/ws";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub status: String,
}

/// WebSocket client for trios-server.
pub struct ApiClient {
    ws: Option<WebSocket>,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        Self { ws: None }
    }

    /// Connect with message and error callbacks (FnMut for Dioxus Signal compatibility).
    pub fn connect_with_callback<M, E>(
        &mut self,
        on_message: M,
        mut on_error: E,
    ) -> Result<(), JsValue>
    where
        M: FnMut(String) + 'static,
        E: FnMut() + 'static,
    {
        let ws = WebSocket::new(SERVER_WS_URL)?;

        let onopen = Closure::<dyn Fn()>::new(|| {
            log::info!("[trios-ui] Connected to trios-server");
        });

        let onerror = Closure::<dyn FnMut()>::wrap(Box::new(move || {
            log::error!("[trios-ui] WebSocket error");
            on_error();
        }));

        let onclose = Closure::<dyn Fn()>::new(|| {
            log::info!("[trios-ui] Disconnected from trios-server");
        });

        let mut on_message = on_message;
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(
            move |ev: MessageEvent| {
                if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    on_message(text);
                }
            },
        ));

        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

        onopen.forget();
        onerror.forget();
        onclose.forget();
        onmessage.forget();

        self.ws = Some(ws);
        Ok(())
    }

    pub fn send_chat(&self, message: &str) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?;

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "agents/chat",
            "params": { "message": message }
        });

        let json = serde_json::to_string(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&json)?;
        Ok(())
    }

    pub fn list_agents(&self) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?;

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "agents/list"
        });

        let json = serde_json::to_string(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&json)?;
        Ok(())
    }

    pub fn list_tools(&self) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?;

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        });

        let json = serde_json::to_string(&payload)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&json)?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        matches!(&self.ws, Some(ws) if ws.ready_state() == WebSocket::OPEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_url_uses_port_9005() {
        assert!(SERVER_WS_URL.contains("9005"));
    }

    #[test]
    fn client_starts_disconnected() {
        let client = ApiClient::new();
        assert!(!client.is_connected());
    }
}
