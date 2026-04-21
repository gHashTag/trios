# TRIOS Global Invariants

These rules MUST hold after every PR merge. Any PR that violates them is BLOCKED.

## Extension (crates/trios-ext)

| ID | Invariant | Verification |
|---|---|---|
| I1 | `extension/dist/bg-sw.js` md5 stable across `cargo build` | `md5 -q crates/trios-ext/extension/dist/bg-sw.js` before/after |
| I2 | `extension/dist/bootstrap.js` md5 stable across `cargo build` | `md5 -q crates/trios-ext/extension/dist/bootstrap.js` before/after |
| I3 | 0 direct WASM imports in service worker | `grep -c 'wasm\|WASM' crates/trios-ext/extension/dist/bg-sw.js` == 0 |
| I4 | 0 WebSocket references in extension code | `grep -rc 'WebSocket\|ws://' crates/trios-ext/src/` == 0 |
| I5 | Single extension tree at `crates/trios-ext/extension/` | `test -d crates/trios-ext/extension && ! test -d extension/dist` |
| I6 | `build.rs` never overwrites committed source files | `build.rs` only copies from `pkg/` to `extension/dist/` |
| I7 | CSP includes `wasm-unsafe-eval` | `grep wasm-unsafe-eval crates/trios-ext/extension/manifest.json` |

## Ports

| ID | Invariant | Verification |
|---|---|---|
| I8 | Port 9105 = MCP HTTP (StreamableHTTPTransport) | `curl -s -X POST http://127.0.0.1:9105/mcp -H 'Accept: application/json' -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' \| grep protocolVersion` |
| I9 | Port 9002 = BrowserOS upstream (proxy to 9105) | `curl -s http://127.0.0.1:9002/health` |
| I10 | Port 9200 = Console bridge | `curl -s http://127.0.0.1:9200/` |

## Build

| ID | Invariant | Verification |
|---|---|---|
| I11 | `cargo build --workspace` green (warnings ok, errors not) | `cargo build --workspace 2>&1 \| grep '^error'` == empty |
| I12 | WASM built with `--target web` (not bundler) | `grep -c 'new URL.*import.meta.url' crates/trios-ext/extension/dist/trios_ext.js` >= 1 |

## Protocol

| ID | Invariant | Verification |
|---|---|---|
| I13 | MCP requests include `Accept: application/json, text/event-stream` | `grep Accept crates/trios-ext/src/mcp.rs` |
| I14 | Chat uses REST `POST /chat`, not MCP method | `grep CHAT_HTTP_URL crates/trios-ext/src/mcp.rs` |
