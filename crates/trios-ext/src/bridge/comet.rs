use crate::bridge::types::{Envelope, Payload};
use crate::dom;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::WebSocket;

const TRIOS_WS_URL: &str = "ws://localhost:9005/ws";

pub struct CometBridge {
    ws: Option<WebSocket>,
    port: Option<JsValue>,
    is_running: bool,
}

impl Default for CometBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl CometBridge {
    pub fn new() -> Self {
        Self {
            ws: None,
            port: None,
            is_running: false,
        }
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        log::info!("Comet bridge starting...");

        if self.is_running {
            log::warn!("Bridge already running");
            return Ok(());
        }

        self.is_running = true;

        let ws = WebSocket::new(TRIOS_WS_URL)?;
        self.connect_ws(&ws)?;
        self.ws = Some(ws);

        self.setup_chrome_runtime()?;

        Ok(())
    }

    pub fn stop(&mut self) {
        log::info!("Comet bridge stopping...");

        self.is_running = false;

        if let Some(ws) = self.ws.take() {
            let _ = ws.close();
        }
        self.port = None;
    }

    pub fn send(&self, envelope: Envelope) -> Result<(), JsValue> {
        let ws = self
            .ws
            .as_ref()
            .ok_or_else(|| JsValue::from_str("bridge not connected"))?;

        if ws.ready_state() != WebSocket::OPEN {
            return Err(JsValue::from_str("WebSocket not open"));
        }

        let json_str = envelope.to_json().map_err(|e| {
            log::error!("Envelope serialization failed: {:?}", e);
            JsValue::from_str(&e.to_string())
        })?;

        ws.send_with_str(&json_str)?;
        Ok(())
    }

    pub fn send_chat(&self, content: &str, role: &str) -> Result<(), JsValue> {
        let envelope = Envelope::chat_message("bridge", content, role);
        self.send(envelope)
    }

    pub fn send_heartbeat(&self, agent_id: &str, status: &str) -> Result<(), JsValue> {
        let envelope = Envelope::agent_heartbeat(agent_id, status);
        self.send(envelope)
    }

    pub fn is_connected(&self) -> bool {
        self.ws
            .as_ref()
            .is_some_and(|ws| ws.ready_state() == WebSocket::OPEN)
    }

    fn connect_ws(&self, ws: &WebSocket) -> Result<(), JsValue> {
        log::info!("Connecting to WebSocket: {}", TRIOS_WS_URL);

        let onopen_closure: Closure<dyn Fn()> = Closure::new(|| {
            log::info!("WebSocket connected");
            let _ = dom::set_status("Connected to trios-server via Comet");
        });

        let onerror_closure: Closure<dyn Fn()> = Closure::new(|| {
            log::error!("WebSocket error");
            let _ = dom::set_status("WebSocket error");
        });

        let onclose_closure: Closure<dyn Fn()> = Closure::new(|| {
            log::warn!("WebSocket closed");
            let _ = dom::set_status("Disconnected from trios-server");
        });

        let onmessage_closure: Closure<dyn Fn(web_sys::MessageEvent)> =
            Closure::new(|ev: web_sys::MessageEvent| {
                if let Ok(txt) = ev.data().dyn_into::<js_sys::JsString>() {
                    let text: String = txt.into();
                    match Envelope::from_json(&text) {
                        Ok(envelope) => process_envelope(envelope),
                        Err(e) => {
                            log::error!("Failed to parse envelope: {:?}", e);
                        }
                    }
                }
            });

        ws.set_onopen(Some(onopen_closure.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror_closure.as_ref().unchecked_ref()));
        ws.set_onclose(Some(onclose_closure.as_ref().unchecked_ref()));
        ws.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));

        Ok(())
    }

    fn setup_chrome_runtime(&mut self) -> Result<(), JsValue> {
        log::info!("Setting up Chrome runtime messaging...");

        let chrome = get_chrome_global()?;
        let runtime = js_sys::Reflect::get(&chrome, &JsValue::from_str("runtime"))?;
        let on_connect = js_sys::Reflect::get(&runtime, &JsValue::from_str("onConnect"))?;
        let add_listener = js_sys::Reflect::get(&on_connect, &JsValue::from_str("addListener"))?;

        let onconnect_closure: Closure<dyn Fn(JsValue)> = Closure::new(|_port: JsValue| {
            log::info!("Chrome runtime port connected");
        });

        let _ = js_sys::Function::from(add_listener).call1(&on_connect, onconnect_closure.as_ref());

        Ok(())
    }
}

#[allow(clippy::let_unit_value)]
fn process_envelope(envelope: Envelope) {
    log::debug!("Processing envelope: {:?}", envelope.event_type);

    match envelope.payload {
        Payload::AgentHeartbeat {
            agent_id, status, ..
        } => {
            let status_text = format!("{}: {}", agent_id, status);
            let _ = dom::set_agent_list(&status_text);
        }
        Payload::ChatMessage { content, role, .. } => {
            let _ = dom::append_message(&role, &content);
        }
        Payload::AgentConnected { agent_id } => {
            let _ = dom::set_agent_list(&format!("{}: connected", agent_id));
        }
        Payload::AgentDisconnected { agent_id } => {
            let _ = dom::set_agent_list(&format!("{}: disconnected", agent_id));
        }
        Payload::McpToolCall { tool, params } => {
            let tool_text = format!(
                "Tool: {}\n{}",
                tool,
                serde_json::to_string(&params).unwrap_or_default()
            );
            let _ = dom::set_tool_list(&tool_text);
        }
    }
}

fn get_chrome_global() -> Result<js_sys::Object, JsValue> {
    let global = js_sys::global();
    js_sys::Reflect::get(&global, &JsValue::from_str("chrome")).and_then(|v| {
        v.dyn_into::<js_sys::Object>()
            .map_err(|_| JsValue::from_str("chrome is not an object"))
    })
}

thread_local! {
    static BRIDGE: std::cell::RefCell<Option<CometBridge>> = const { std::cell::RefCell::new(None) };
}

#[wasm_bindgen]
pub fn comet_bridge_init() -> Result<(), JsValue> {
    log::info!("Initializing Comet bridge...");
    BRIDGE.with(|b| {
        let mut bridge = CometBridge::new();
        let result = bridge.start();
        *b.borrow_mut() = Some(bridge);
        result
    })
}

#[wasm_bindgen]
pub fn comet_bridge_connect() -> Result<(), JsValue> {
    BRIDGE.with(|b| {
        if let Some(bridge) = b.borrow_mut().as_mut() {
            bridge.start()
        } else {
            Err(JsValue::from_str("bridge not initialized"))
        }
    })
}

#[wasm_bindgen]
pub fn comet_bridge_disconnect() {
    BRIDGE.with(|b| {
        if let Some(bridge) = b.borrow_mut().as_mut() {
            bridge.stop();
        }
    });
}

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

#[wasm_bindgen]
pub fn comet_is_connected() -> bool {
    BRIDGE.with(|b| {
        b.borrow()
            .as_ref()
            .is_some_and(|bridge| bridge.is_connected())
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
            Payload::AgentHeartbeat {
                agent_id, status, ..
            } => {
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
            Payload::ChatMessage {
                agent_id,
                content,
                role,
            } => {
                assert_eq!(agent_id, "CHARLIE-03");
                assert_eq!(content, "hello");
                assert_eq!(role, "agent");
            }
            _ => panic!("Roundtrip failed"),
        }
    }
}
