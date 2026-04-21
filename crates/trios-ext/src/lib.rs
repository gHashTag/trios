//! Trios Chrome Extension — Rust + WASM
//! Trinity Stack Law Compliant: Zero handwritten JS

pub mod bg;
pub mod bridge;
pub mod dom;
pub mod mcp;

// Re-export bridge functions for JavaScript access
pub use bridge::comet::{
    comet_bridge_init, comet_bridge_connect, comet_bridge_disconnect,
    comet_send_chat, comet_is_connected,
    CometBridge,
};

// Note: Dioxus launch requires different setup for Chrome Extensions
// Using direct DOM manipulation for now (L6 compliant)
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    // Initialize the extension
    console_error_panic_hook::set_once();
    log::info!("Trios extension initialized");

    // Initialize Comet bridge for trios-server connection
    if let Err(e) = crate::comet_bridge_init() {
        log::error!("Failed to initialize Comet bridge: {:?}", e);
    }

    // For MVP, use simple DOM setup
    if let Err(e) = crate::dom::build_ui() {
        log::error!("Failed to build UI: {:?}", e);
    }

    // Load initial data
    let _ = crate::mcp::mcp_list_agents();
    let _ = crate::mcp::mcp_list_tools();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_url_uses_port_9005() {
        assert!(mcp::MCP_WS_URL.contains("9005"));
    }

    #[test]
    fn mcp_client_starts_disconnected() {
        let client = mcp::McpClient::new();
        assert!(!client.is_connected());
    }
}
