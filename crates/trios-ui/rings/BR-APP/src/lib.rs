//! BR-APP — WASM Entry Point
//!
//! Wires all UI rings together. This is the single WASM binary
//! that gets compiled for the browser.
//!
//! ## Usage
//!
//! ```rust,ignore
//! // From Chrome Extension:
//! trios_ui::mount_app();
//! ```

use wasm_bindgen::prelude::*;

/// WASM entry point. Called by the browser when the WASM module loads.
#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Trinity UI WASM initializing...");
    trios_ui_ur08::mount_app();
    log::info!("Trinity UI WASM initialized successfully");
}
