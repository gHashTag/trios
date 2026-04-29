# RING.md — SILVER-RING-EXT-01

## Metal
Silver

## Crate
trios-ext

## Package
trios-ext-ring-ex01

## Purpose
DOM bridge — provides DOM manipulation utilities, UI building, and element access for the Trios Chrome Extension.

## Dependencies (internal rings)
None — this is a standalone ring.

## Dependencies (external)
- `wasm-bindgen`
- `web-sys`
- `js-sys`
- `log`

## API Surface
- `pub fn document() -> Result<Document, JsValue>`
- `pub fn set_status(text: &str) -> Result<(), JsValue>`
- `pub fn append_message(role: &str, text: &str)`
- `pub fn set_agent_list(text: &str)`
- `pub fn set_tool_list(text: &str)`
- `pub fn get_style() -> &'static str`
- `pub fn build_ui() -> Result<(), JsValue>`

## Build
```bash
cargo build -p trios-ext-ring-ex01
cargo test -p trios-ext-ring-ex01
```
