//! BR-APP — Bronze Output Ring for trios-ui
//!
//! This ring re-exports the UR-00 WASM entry point.
//! Build with: `cargo build -p trios-ui-br-app --target wasm32-unknown-unknown --release`
//! Then run: `wasm-bindgen --target web target/wasm32-unknown-unknown/release/trios_ui_br_app.wasm --out-dir dist/`

pub use trios_ui_ring_ur00::*;
