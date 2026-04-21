pub mod dom;
pub mod mcp;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    log::info!("Trios extension WASM initialized (sidepanel context)");
    if let Err(e) = crate::dom::build_ui() {
        log::error!("Failed to build UI: {:?}", e);
    }
    if let Err(e) = crate::mcp::mcp_connect() {
        log::warn!("MCP connect deferred: {:?}", e);
    }
    let _ = crate::mcp::mcp_list_tools();
}
