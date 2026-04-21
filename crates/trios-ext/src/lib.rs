//! Trios Chrome Extension — Rust + Dioxus + WASM
//! Trinity Stack Law Compliant: Zero handwritten JS

pub mod bg;
pub mod dom;
pub mod mcp;
pub mod app;

use app::App;

#[wasm_bindgen(start)]
pub fn run() {
    // Launch Dioxus app
    dioxus_web::launch(App);
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
