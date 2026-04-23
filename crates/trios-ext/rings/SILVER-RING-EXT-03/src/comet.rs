//! Comet Bridge — WebSocket connection to trios-server
//!
//! Handles EnvelopeT message processing and Chrome runtime messaging.

use crate::types::{Envelope, Payload};
use js_sys::{Array, Function};
use serde_json::json;
use trios_ext_ring_ex01 as dom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};

const TRIOS_WS_URL: &str = "ws://localhost:9005/ws";

thread_local! {
    static BRIDGE: std::cell::RefCell<CometBridge> = std::cell::RefCell::new(CometBridge::new());
}

pub struct CometBridge {
    ws: Option<WebSocket>,
}

impl Default for CometBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CometBridge {
    pub fn new() -> Self {
        Self { ws: None }
    }

    pub fn connect(&mut self) -> Result<(), JsValue> {
        let ws = WebSocket::new(TRIOS_WS_URL)?;

        let onopen: Closure<dyn Fn()> = Closure::new(|| {
            dom::set_status("Connected to trios-server via Comet").ok();
        });

        let onerror: Closure<dyn Fn()> = Closure::new(|| {
            dom::set_status("WebSocket error — reconnecting...").ok();
        });

        let onclose: Closure<dyn Fn()> = Closure::new(|| {
            dom::set_status("Disconnected from trios-server").ok();
        });

        let onmessage: Closure<dyn Fn(MessageEvent)> = Closure::new(|ev: MessageEvent| {
            if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                let text: String = txt.into();
                if let Ok(envelope) = Envelope::from_json(&text) {
                    match &envelope.payload {
                        Payload::ChatMessage { content, role, .. } => {
                            dom::append_message(role, content);
                        }
                        Payload::AgentHeartbeat {
                            agent_id, status, ..
                        } => {
                            dom::set_agent_list(&format!("{}: {}", agent_id, status));
                        }
                        Payload::AgentConnected { agent_id } => {
                            dom::set_agent_list(&format!("{}: connected", agent_id));
                        }
                        Payload::AgentDisconnected { agent_id } => {
                            dom::set_agent_list(&format!("{}: disconnected", agent_id));
                        }
                        Payload::McpToolCall { tool, params } => {
                            let text = format!(
                                "Tool: {}\n{}",
                                tool,
                                serde_json::to_string(params).unwrap_or_default()
                            );
                            dom::set_tool_list(&text);
                        }
                    }
                    notify_chrome(&envelope.event_type, &json!(envelope.payload));
                }
            }
        });

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

    pub fn send_chat(&self, content: &str, role: &str) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("not connected"))?;
        let envelope = Envelope::chat_message("bridge", content, role);
        let json = envelope
            .to_json()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        ws.send_with_str(&json)?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        matches!(&self.ws, Some(ws) if ws.ready_state() == WebSocket::OPEN)
    }
}

fn notify_chrome(event_type: &str, payload: &serde_json::Value) {
    let global = js_sys::global();
    let Ok(chrome) = js_sys::Reflect::get(&global, &JsValue::from_str("chrome")) else {
        return;
    };
    let Ok(runtime) = js_sys::Reflect::get(&chrome, &JsValue::from_str("runtime")) else {
        return;
    };

    if let Ok(connect) = js_sys::Reflect::get(&runtime, &JsValue::from_str("connect")) {
        let args = Array::new();
        if let Ok(port) = Function::from(connect).apply(&runtime, &args) {
            let msg = js_sys::Object::new();
            let _ = js_sys::Reflect::set(
                &msg,
                &JsValue::from_str("type"),
                &JsValue::from_str(event_type),
            );
            let _ = js_sys::Reflect::set(
                &msg,
                &JsValue::from_str("data"),
                &JsValue::from_str(&serde_json::to_string(payload).unwrap_or_default()),
            );
            if let Ok(post_msg) = js_sys::Reflect::get(&port, &JsValue::from_str("postMessage")) {
                let args = Array::new();
                args.push(&msg);
                let _ = Function::from(post_msg).apply(&port, &args);
            }
        }
    }
}

#[wasm_bindgen]
pub fn comet_bridge_init() -> Result<(), JsValue> {
    BRIDGE.with(|b| b.borrow_mut().connect())
}

#[wasm_bindgen]
pub fn comet_bridge_connect() -> Result<(), JsValue> {
    BRIDGE.with(|b| b.borrow_mut().connect())
}

#[wasm_bindgen]
pub fn comet_bridge_disconnect() {
    BRIDGE.with(|b| {
        let mut bridge = b.borrow_mut();
        if let Some(ws) = bridge.ws.take() {
            let _ = ws.close();
        }
    });
}

#[wasm_bindgen]
pub fn comet_send_chat(message: &str) -> Result<(), JsValue> {
    BRIDGE.with(|b| b.borrow().send_chat(message, "user"))
}

#[wasm_bindgen]
pub fn comet_is_connected() -> bool {
    BRIDGE.with(|b| b.borrow().is_connected())
}

#[cfg(test)]
mod tests {
    #[test]
    fn comet_module_compiles() {}
}
