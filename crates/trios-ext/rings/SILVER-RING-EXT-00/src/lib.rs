//! SILVER-RING-EXT-00 — WASM Entry Point
//!
//! Trinity Stack Law Compliant: Zero handwritten JS.
//! This ring wires together all other Silver rings and serves as the
//! main WASM entry point for the Trios Chrome Extension.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, HtmlInputElement};

use trios_ext_ring_ex01 as dom;
use trios_ext_ring_ex02 as mcp_ring;
use trios_ext_ring_ex03 as bridge;

// Re-export bridge functions for JavaScript access
pub use bridge::comet::{
    comet_bridge_init, comet_bridge_connect, comet_bridge_disconnect,
    comet_send_chat, comet_is_connected,
    CometBridge,
};

// Re-export background init
pub use mcp_ring::bg::background_init;

#[wasm_bindgen(start)]
pub fn run() {
    // Initialize the extension
    console_error_panic_hook::set_once();
    log::info!("Trios extension initialized");

    // Initialize Comet bridge for trios-server connection
    if let Err(e) = bridge::comet::comet_bridge_init() {
        log::error!("Failed to initialize Comet bridge: {:?}", e);
    }

    // Build UI (DOM ring)
    if let Err(e) = dom::build_ui() {
        log::error!("Failed to build UI: {:?}", e);
    }

    // Set up chat listener (cross-ring wiring: DOM → MCP)
    if let Ok(doc) = dom::document() {
        if let Err(e) = setup_chat_listener(&doc) {
            log::error!("Failed to setup chat listener: {:?}", e);
        }
    }

    // Load initial data
    let _ = mcp_ring::mcp::mcp_list_agents();
    let _ = mcp_ring::mcp::mcp_list_tools();
}

/// Chat listener — wires DOM input to MCP send.
/// This function lives in the entry ring because it crosses ring boundaries
/// (DOM ring ↔ MCP ring), which neither ring should depend on directly.
fn setup_chat_listener(doc: &Document) -> Result<(), JsValue> {
    let input = doc
        .get_element_by_id("chat-input")
        .ok_or("no chat-input")?;
    let input: HtmlInputElement = input.dyn_into()?;

    let closure =
        Closure::<dyn Fn(web_sys::KeyboardEvent)>::new(move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Enter" {
                if let Some(val) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("chat-input"))
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                {
                    let text = val.value().trim().to_string();
                    if !text.is_empty() {
                        dom::append_message("you", &text);
                        val.set_value("");
                        let msg = text.clone();
                        let _ = mcp_ring::mcp::mcp_send_chat(&msg);
                    }
                }
            }
        });

    let _ = input.add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
    closure.forget();

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn entry_ring_compiles() {}
}
