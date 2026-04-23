//! BR-EXT — WASM Entry Point
//!
//! Wires all EXT rings together. This is the single WASM binary
//! that `wasm-pack build` compiles.

// Re-export injector functions for wasm-bindgen
pub use trios_ext_03::*;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    log::info!("Trios extension WASM initialized (sidepanel context)");
    // Load API key from chrome.storage.local
    trios_ext_02::load_api_key();
    if let Err(e) = trios_ext_00::build_ui() {
        log::error!("Failed to build UI: {:?}", e);
    }
    // Try MCP connect (server may be offline — that's OK, z.ai direct chat works via ⚙ Settings)
    match trios_ext_00::mcp_connect() {
        Ok(()) => {
            let _ = trios_ext_00::mcp_list_tools();
            let _ = trios_ext_00::mcp_list_issues();
        }
        Err(_) => {
            let _ = trios_ext_00::set_status("MCP offline — configure z.ai key in ⚙ Settings for direct chat");
        }
    }
}
