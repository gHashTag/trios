//! UR-07 — WebSocket API Client

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

pub struct ApiClient {
    ws: Option<WebSocket>,
}

impl Default for ApiClient {
    fn default() -> Self { Self::new() }
}

impl ApiClient {
    pub fn new() -> Self {
        Self { ws: None }
    }

    pub fn connect_with_callback<M, O, E>(
        &mut self,
        on_message: M,
        on_open: O,
        mut on_error: E,
    ) -> Result<(), JsValue>
    where
        M: FnMut(String) + 'static,
        O: FnOnce() + 'static,
        E: FnMut() + 'static,
    {
        log::info!("[UR-07] WebSocket::new({})", SERVER_WS_URL);
        let ws = match WebSocket::new(SERVER_WS_URL) {
            Ok(w) => { log::info!("[UR-07] WS created ok, readyState={}", w.ready_state()); w }
            Err(e) => { log::error!("[UR-07] WS create FAILED: {:?}", e); return Err(e); }
        };
        let ws_clone = ws.clone();
        let on_open = std::cell::RefCell::new(Some(on_open));

        let onopen = Closure::<dyn Fn()>::new(move || {
            log::info!("[UR-07] onopen fired — WS is OPEN");

            let agents_payload = json!({"jsonrpc": "2.0", "id": 2, "method": "agents/list"});
            match serde_json::to_string(&agents_payload) {
                Ok(json) => {
                    match ws_clone.send_with_str(&json) {
                        Ok(_) => log::info!("[UR-07] sent agents/list ({} bytes)", json.len()),
                        Err(e) => log::error!("[UR-07] send agents/list FAILED: {:?}", e),
                    }
                }
                Err(e) => log::error!("[UR-07] serialize agents/list FAILED: {}", e),
            }

            let tools_payload = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/list"});
            match serde_json::to_string(&tools_payload) {
                Ok(json) => {
                    match ws_clone.send_with_str(&json) {
                        Ok(_) => log::info!("[UR-07] sent tools/list ({} bytes)", json.len()),
                        Err(e) => log::error!("[UR-07] send tools/list FAILED: {:?}", e),
                    }
                }
                Err(e) => log::error!("[UR-07] serialize tools/list FAILED: {}", e),
            }

            if let Some(cb) = on_open.borrow_mut().take() {
                log::info!("[UR-07] calling on_open callback");
                cb();
            } else {
                log::warn!("[UR-07] on_open callback already consumed!");
            }
        });

        let onerror = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: web_sys::Event| {
            log::error!("[UR-07] onerror fired: {:?}", e.type_());
            on_error();
        }));

        let onclose = Closure::<dyn Fn(_)>::new(move |e: web_sys::CloseEvent| {
            log::warn!("[UR-07] onclose fired: code={} reason='{}' wasClean={}",
                e.code(), e.reason(), e.was_clean());
        });

        let mut on_message = on_message;
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(
            move |ev: MessageEvent| {
                if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    log::debug!("[UR-07] onmessage {} bytes", text.len());
                    on_message(text);
                } else {
                    log::warn!("[UR-07] onmessage got non-string data");
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

        log::info!("[UR-07] callbacks registered, readyState={}", ws.ready_state());
        self.ws = Some(ws);
        Ok(())
    }

    pub fn send_chat(&self, message: &str) -> Result<(), JsValue> {
        let ws = self.ws.as_ref().ok_or_else(|| JsValue::from_str("not connected"))?;
        log::info!("[UR-07] send_chat readyState={} msg='{}'", ws.ready_state(), message);
        let payload = json!({
            "jsonrpc": "2.0", "id": 1,
            "method": "agents/chat",
            "params": { "message": message }
        });
        let json = serde_json::to_string(&payload).map_err(|e| JsValue::from_str(&e.to_string()))?;
        match ws.send_with_str(&json) {
            Ok(_) => { log::info!("[UR-07] send_chat OK ({} bytes)", json.len()); Ok(()) }
            Err(e) => { log::error!("[UR-07] send_chat FAILED: {:?}", e); Err(e) }
        }
    }

    pub fn is_connected(&self) -> bool {
        matches!(&self.ws, Some(ws) if ws.ready_state() == WebSocket::OPEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_url_uses_port_9005() { assert!(SERVER_WS_URL.contains("9005")); }
    #[test]
    fn client_starts_disconnected() { assert!(!ApiClient::new().is_connected()); }
}
