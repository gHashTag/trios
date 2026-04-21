pub mod bg;
pub mod dom;
pub mod mcp;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    if web_sys::window().is_none() {
        bg::background_init()?;
        return Ok(());
    }

    dom::build_ui()?;
    mcp::mcp_connect()?;

    if let Ok(_doc) = dom::document() {
        crate::dom::set_agent_list("Loading agents...");
        crate::dom::set_tool_list("Loading tools...");
    }
    let _ = mcp::mcp_list_agents();
    let _ = mcp::mcp_list_tools();

    Ok(())
}

#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Trios says: Hello, {}!", name)
}

#[wasm_bindgen]
pub fn init_background() -> Result<(), JsValue> {
    bg::background_init()
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

    #[test]
    fn mcp_request_serializes() {
        let req = mcp::McpRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "agents/list".to_string(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("agents/list"));
        assert!(json.contains("2.0"));
    }

    #[test]
    fn style_uses_total_black() {
        let style = dom::get_style();
        assert!(style.contains("#000000"));
    }

    #[test]
    fn style_uses_gold_accent() {
        let style = dom::get_style();
        assert!(style.contains("#D4AF37"));
    }
}
