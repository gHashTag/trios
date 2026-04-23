# RING.md — SILVER-RING-EXT-03

## Metal
Silver

## Crate
trios-ext

## Package
trios-ext-ring-ex03

## Purpose
Comet bridge and typed message envelopes for trios-server ↔ Chrome extension communication.

## Dependencies (internal rings)
- SILVER-RING-EXT-01 (DOM bridge) — for UI status updates and message display

## Dependencies (external)
- `wasm-bindgen`
- `web-sys`
- `js-sys`
- `serde` / `serde_json`
- `log`

## API Surface
- `pub fn comet_bridge_init() -> Result<(), JsValue>`
- `pub fn comet_bridge_connect() -> Result<(), JsValue>`
- `pub fn comet_bridge_disconnect()`
- `pub fn comet_send_chat(message: &str) -> Result<(), JsValue>`
- `pub fn comet_is_connected() -> bool`
- `pub struct CometBridge`
- `pub struct Envelope`
- `pub enum Payload`

## Build
```bash
cargo build -p trios-ext-ring-ex03
cargo test -p trios-ext-ring-ex03
```
