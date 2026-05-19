# RING.md — SILVER-RING-EXT-02

## Metal
Silver

## Crate
trios-ext

## Package
trios-ext-ring-ex02

## Purpose
MCP WebSocket client and Chrome extension background service worker.

## Dependencies (internal rings)
- SILVER-RING-EXT-01 (DOM bridge) — for UI status updates and message display

## Dependencies (external)
- `wasm-bindgen`
- `web-sys`
- `js-sys`
- `serde` / `serde_json`
- `log`

## API Surface
- `pub fn mcp_connect() -> Result<(), JsValue>`
- `pub fn mcp_send_chat(message: &str) -> Result<(), JsValue>`
- `pub fn mcp_list_agents() -> Result<(), JsValue>`
- `pub fn mcp_list_tools() -> Result<(), JsValue>`
- `pub fn mcp_ping() -> Result<(), JsValue>`
- `pub fn mcp_is_connected() -> bool`
- `pub fn background_init() -> Result<(), JsValue>`
- `pub const MCP_WS_URL: &str`
- `pub struct McpClient`
- `pub struct McpRequest`

## Build
```bash
cargo build -p trios-ext-ring-ex02
cargo test -p trios-ext-ring-ex02
```
