//! Comet Bridge Implementation
//!
//! WebSocket client connecting to trios-server (port 9005) with
//! Chrome runtime messaging integration for background ↔ sidepanel communication.
//!
//! Architecture:
//!   Chrome Background ←→ chrome.runtime.connect → Comet Bridge ←→ WebSocket (9005)

use crate::bridge::types::{Envelope, Payload};
use crate::dom;
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use js_sys::Array;

const TRIOS_WS_URL: &str = "ws://localhost:9005/ws";

/// Comet bridge client.
///
/// Maintains WebSocket connection to trios-server and handles
/// Chrome runtime messaging for background ↔ sidepanel communication.
pub struct CometBridge {
    ws: Option<WebSocket>,
    port: Option<JsValue>,
    is_running: std::cell::Cell<bool>,
}

impl CometBridge {
    /// Create a new Comet bridge instance.
    pub fn new() -> Self {
        Self {
            ws: None,
            port: None,
            is_running: std::cell::Cell::new(false),
        }
    }

    /// Start the bridge: connect WebSocket and set up Chrome runtime.
    ///
    /// Returns error if WebSocket connection fails.
    pub fn start(&self) -> Result<(), JsValue> {
        log::info!("Comet bridge starting...");

        if self.is_running.get() {
            log::warn!("Bridge already running");
            return Ok(());
        }

        self.is_running.set(true);

        // Connect WebSocket
        let ws = WebSocket::new(TRIOS_WS_URL)?;
        self.connect_ws(&ws)?;

        // Set up Chrome runtime listener
        self.setup_chrome_runtime()?;

        Ok(())
    }

    /// Stop the bridge: close WebSocket and cleanup.
    pub fn stop(&self) {
        log::info!("Comet bridge stopping...");

        self.is_running.set(false);

        // Close WebSocket if connected
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
        self.ws = None;

        // Clear Chrome port
        self.port = None;
    }

    /// Forward an envelope via the WebSocket.
    ///
    /// Returns error if not connected.
    pub fn send(&self, envelope: Envelope) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("bridge not connected"))?;

        if ws.ready_state() != WebSocket::OPEN {
            return Err(JsValue::from_str("WebSocket not open"));
        }

        let json = envelope.to_json().map_err(|e| {
            log::error!("Envelope serialization failed: {:?}", e);
            JsValue::from_str(&e.to_string())
        })?;

        ws.send_with_str(&json)?;
        Ok(())
    }

    /// Send a chat message through the bridge.
    pub fn send_chat(&self, content: &str, role: &str) -> Result<(), JsValue> {
        let envelope = Envelope::chat_message("bridge", content, role);
        self.send(envelope)
    }

    /// Send an agent heartbeat through the bridge.
    pub fn send_heartbeat(&self, agent_id: &str, status: &str) -> Result<(), JsValue> {
        let envelope = Envelope::agent_heartbeat(agent_id, status);
        self.send(envelope)
    }

    /// Check if bridge is currently connected.
    pub fn is_connected(&self) -> bool {
        self.ws
            .as_ref()
            .map_or(false, |ws| ws.ready_state() == WebSocket::OPEN)
    }

    /// Set up WebSocket event handlers.
    fn connect_ws(&self, ws: &WebSocket) -> Result<(), JsValue> {
        log::info!("Connecting to WebSocket: {}", TRIOS_WS_URL);

        // Store closures in a Vec to prevent garbage collection
        let mut closures: Vec<Box<dyn Fn()>> = Vec::new();

        let onopen_closure: Box<dyn Fn()> = Box::new({
            log::info!("WebSocket connected");
            let _ = dom::set_status("Connected to trios-server via Comet");
            self.notify_chrome("bridge:connected", json!({ "status": "connected" }));
        });

        let onerror_closure: Box<dyn Fn()> = Box::new({
            log::error!("WebSocket error");
            let _ = dom::set_status("WebSocket error — reconnecting...");
            self.notify_chrome("bridge:error", json!({ "error": "connection_failed" }));
        });

        let onclose_closure: Box<dyn Fn()> = Box::new({
            log::warn!("WebSocket closed");
            let _ = dom::set_status("Disconnected from trios-server");
            self.notify_chrome("bridge:disconnected", json!({ "status": "disconnected" }));
            self.ws = None;
        });

        let onmessage_closure: Box<dyn Fn(MessageEvent)> = Box::new({
            let bridge_clone = std::rc::Rc::new(*self);
            move |ev: MessageEvent| {
                if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    match Envelope::from_json(&text) {
                        Ok(envelope) => bridge_clone.process_envelope(envelope),
                        Err(e) => {
                            log::error!("Failed to parse envelope: {:?}", e);
                            log::error!("Raw message: {}", text);
                        }
                    }
                }
            }
        });

        ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));

        // Keep closures alive
        closures.push(onopen_closure);
        closures.push(onerror_closure);
        closures.push(onclose_closure);
        closures.push(onmessage_closure);

        // Store WebSocket reference
        self.ws = Some(ws);

        Ok(())
    }

    /// Process an incoming envelope and update DOM.
    fn process_envelope(&self, envelope: Envelope) {
        log::debug!("Processing envelope: {:?}", envelope.event_type);

        match envelope.payload {
            Payload::AgentHeartbeat { agent_id, status, .. } => {
                let status_text = format!("{}: {}", agent_id, status);
                let _ = dom::set_agent_list(&status_text);
            }
            Payload::ChatMessage { content, role, .. } => {
                let _ = dom::append_message(role, &content);
            }
            Payload::AgentConnected { agent_id, .. } => {
                let list_text = format!("{}: connected", agent_id);
                let _ = dom::set_agent_list(&list_text);
            }
            Payload::AgentDisconnected { agent_id, .. } => {
                let list_text = format!("{}: disconnected", agent_id);
                let _ = dom::set_agent_list(&list_text);
            }
            Payload::McpToolCall { tool, params, .. } => {
                let tool_text = format!("Tool: {}\n{}", tool, serde_json::to_string(params).unwrap_or_default());
                let _ = dom::set_tool_list(&tool_text);
            }
        }

        // Forward to Chrome runtime
        self.notify_chrome(&envelope.event_type, json!(envelope.payload));
    }

    /// Set up Chrome runtime messaging.
    fn setup_chrome_runtime(&self) -> Result<(), JsValue> {
        log::info!("Setting up Chrome runtime messaging...");

        let chrome = get_chrome_global()?;
        let runtime = js_sys::Reflect::get(&chrome, &JsValue::from_str("runtime"))?;

        // Set up onConnect listener
        let on_connect = js_sys::Reflect::get(&runtime, &JsValue::from_str("onConnect"))?;

        let bridge_rc = std::rc::Rc::new(*self);
        let closure: Box<dyn Fn(JsValue)> = Box::new(move |port: JsValue| {
            log::info!("Chrome runtime port connected");
            bridge_rc.handle_chrome_port_connected(port);
        });

        let add_listener = js_sys::Reflect::get(&on_connect, &JsValue::from_str("addListener"))?;

        let args = Array::new();
        args.push(closure.as_ref());
        let _ = js_sys::Function::from(add_listener).apply(&on_connect, &args);

        // Note: we don't forget the closure to keep the listener alive
        Ok(())
    }

    /// Handle Chrome runtime port connected event.
    fn handle_chrome_port_connected(&self, port: JsValue) {
        log::info!("Chrome port connected");
        self.port = Some(port);

        // Set up message listener from the port
        let on_message = js_sys::Reflect::get(&port, &JsValue::from_str("onMessage"))?;

        if let Some(on_msg) = on_message {
            let bridge_rc = std::rc::Rc::new(*self);
            let closure: Box<dyn Fn(JsValue, JsValue)> = Box::new(move |_sender, message| {
                bridge_rc.handle_chrome_message(message);
            });

            let add_listener = js_sys::Reflect::get(&on_msg, &JsValue::from_str("addListener"))?;

            if let Some(add) = add_listener {
                let args = Array::new();
                args.push(closure.as_ref());
                let _ = js_sys::Function::from(add).apply(&on_msg, &args);
            }
        }

        let _ = dom::set_status("Chrome extension connected via Comet");
    }

    /// Handle incoming Chrome runtime message.
    fn handle_chrome_message(&self, message: JsValue) {
        log::debug!("Chrome message: {:?}", message);

        // Parse as BridgeMessage
        if let Ok(msg_str) = message.dyn_into::<js_sys::JsString>() {
            let s: String = msg_str.into();

            match s.as_str() {
                "connect" => {
                    let _ = self.start();
                }
                "disconnect" => {
                    self.stop();
                }
                _ => {
                    log::warn!("Unknown bridge message: {}", s);
                }
            }
        }
    }

    /// Send a message to Chrome runtime (all connected ports).
    fn notify_chrome(&self, event_type: &str, payload: serde_json::Value) {
        if let Some(port) = &self.port {
            let post_message = js_sys::Reflect::get(&port, &JsValue::from_str("postMessage"))?;

            if let Some(post_msg) = post_message {
                let message = json!({
                    "type": event_type,
                    "payload": payload,
                });
                let args = Array::new();
                args.push(&message);
                let _ = js_sys::Function::from(post_msg).call(port, &args);
            }
        }
    }
}

/// Get the chrome global object.
fn get_chrome_global() -> Result<js_sys::Object, JsValue> {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &JsValue::from_str("chrome"))
        .ok_or(JsValue::from_str("no chrome global"))
}

thread_local! {
    static BRIDGE: std::cell::RefCell<Option<CometBridge>> = std::cell::RefCell::new(None);
}

/// Initialize the Comet bridge and start connecting.
///
/// Called from JavaScript when extension initializes.
#[wasm_bindgen]
pub fn comet_bridge_init() -> Result<(), JsValue> {
    log::info!("Initializing Comet bridge...");

    let bridge = CometBridge::new();
    BRIDGE.with(|b| {
        *b.borrow_mut() = Some(bridge);
    });

    bridge.start()
}

/// Connect the Comet bridge (start WebSocket connection).
#[wasm_bindgen]
pub fn comet_bridge_connect() -> Result<(), JsValue> {
    BRIDGE.with(|b| {
        if let Some(bridge) = b.borrow().as_ref() {
            bridge.start()
        } else {
            log::warn!("Bridge not initialized, call comet_bridge_init first");
            Err(JsValue::from_str("bridge not initialized"))
        }
    })
}

/// Disconnect the Comet bridge (close WebSocket).
#[wasm_bindgen]
pub fn comet_bridge_disconnect() {
    BRIDGE.with(|b| {
        if let Some(bridge) = b.borrow().as_ref() {
            bridge.stop();
        }
    });
}

/// Send a chat message via the Comet bridge.
#[wasm_bindgen]
pub fn comet_send_chat(content: &str) -> Result<(), JsValue> {
    BRIDGE.with(|b| {
        if let Some(bridge) = b.borrow().as_ref() {
            bridge.send_chat(content, "user")
        } else {
            Err(JsValue::from_str("bridge not initialized"))
        }
    })
}

/// Check if the Comet bridge is connected.
#[wasm_bindgen]
pub fn comet_is_connected() -> bool {
    BRIDGE.with(|b| {
        b.borrow()
            .as_ref()
            .map_or(false, |bridge| bridge.is_connected())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ws_url_uses_port_9005() {
        assert!(TRIOS_WS_URL.contains("9005"));
    }

    #[test]
    fn bridge_new() {
        let bridge = CometBridge::new();
        assert!(!bridge.is_connected());
    }

    #[test]
    fn envelope_chat_message() {
        let envelope = Envelope::chat_message("ALFA-01", "test message", "user");
        assert_eq!(envelope.event_type, "chat:message");

        match envelope.payload {
            Payload::ChatMessage { content, role, .. } => {
                assert_eq!(content, "test message");
                assert_eq!(role, "user");
            }
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn envelope_agent_heartbeat() {
        let envelope = Envelope::agent_heartbeat("BRAVO-02", "active");
        assert_eq!(envelope.event_type, "agent:heartbeat");

        match envelope.payload {
            Payload::AgentHeartbeat { agent_id, status, .. } => {
                assert_eq!(agent_id, "BRAVO-02");
                assert_eq!(status, "active");
            }
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn envelope_serialization_roundtrip() {
        let original = Envelope::chat_message("CHARLIE-03", "hello", "agent");
        let json = original.to_json().unwrap();

        let deserialized = Envelope::from_json(&json).unwrap();

        match deserialized.payload {
            Payload::ChatMessage { agent_id, content, role } => {
                assert_eq!(agent_id, "CHARLIE-03");
                assert_eq!(content, "hello");
                assert_eq!(role, "agent");
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}
