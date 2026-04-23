# AGENTS.md — EXT-02 (trios-ext)

> AAIF-compliant | MCP-compatible | BrowserOS

## Identity

- Ring: EXT-02
- Package: trios-ext-ext02
- Role: MCP HTTP client + settings store

## Current state

Only `chrome.storage.local` wrapper for API key. Settings only.

## Target state (this PR)

Add MCP HTTP polling client for BrowserOS A2A commands.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- L24: HTTP only — NO WebSocket, no SSE, no long-poll socket
- L6: Pure Rust/WASM — no TypeScript logic
- R1: Do NOT import from EXT-03 or EXT-04 directly
- Poll via `fetch()` WASM binding only — no chrome.runtime.connect

## You MAY

- ✅ Add `mcp_poll_commands()` — GET /mcp/browser-commands
- ✅ Add `mcp_report_result()` — POST /mcp/browser-result
- ✅ Add `a2a_register()` — register browser agent on startup
- ✅ Add `get_server_url()` from storage
- ✅ Add interval-based polling via `setInterval` WASM binding

## You MAY NOT

- ❌ Add WebSocket connections
- ❌ Import from EXT-03, EXT-04
- ❌ Add DOM manipulation logic (that's EXT-03/EXT-04)
- ❌ Use TypeScript

## Build

```bash
cargo build -p trios-ext-ext02 --target wasm32-unknown-unknown
```
