# RING — EXT-02 (trios-ext)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-ext-ext02 |
| Sealed | No |

## Current state

Settings ring: `chrome.storage.local` wrapper for API key.

## Upgraded purpose (BrowserOS A2A)

MCP HTTP Client ring. Polls `trios-server` for pending browser commands
and reports results back. Also manages A2A agent registration on startup.

## Why EXT-02 owns the HTTP client

EXT-03 owns DOM injection. EXT-04 owns command execution.
Something must own the **transport** — the HTTP poll loop and result reporting.
EXT-02 already owns `chrome.storage.local` (settings) so it's the natural home
for the client config (server URL, poll interval, auth token).

## API Surface (target after upgrade)

| Function | Role |
|----------|------|
| `mcp_poll_commands()` | GET /mcp/browser-commands → Vec<BrowserCommand> |
| `mcp_report_result()` | POST /mcp/browser-result |
| `a2a_register()` | POST /a2a with a2a_register_agent tool call |
| `get_server_url()` | Read server URL from chrome.storage.local |
| `get_poll_interval_ms()` | Read poll interval (default: 2000ms) |

## Laws

- L24: HTTP only — no WebSocket
- I4: No direct WebSocket connections
- L6: Pure Rust/WASM — no TypeScript
- R1: No imports from EXT-03, EXT-04
