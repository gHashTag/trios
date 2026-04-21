//! Trios Chrome Extension — Rust + Dioxus + WASM
//! Trinity Stack Law Compliant: Zero handwritten JS

pub mod bg;
pub mod dom;
pub mod mcp;
pub mod app;

use app::App;
use dioxus_web::Config;

#[wasm_bindgen(start)]
pub fn run() {
    // Launch Dioxus app for web target
    dioxus_web::launch(App, &Config::default());
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
