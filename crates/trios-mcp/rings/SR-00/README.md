# SR-00: HTTP + WebSocket Server

## Ring Purpose

axum HTTP server on port 3025 + WebSocket for real-time communication with Chrome Extension.
Replaces `browser-tools-server/browser-connector.ts` (1439 lines, Express + ws).

## TypeScript Reference

`browser-tools-server/browser-connector.ts`

## Rust Files

- `src/lib.rs` — BrowserConnector struct, main entry point
- `src/server.rs` — axum HTTP routes (Express replacement)
- `src/auth.rs` — Basic Auth middleware
- `src/ws.rs` — WebSocket handler (ws lib replacement)
- `src/screenshot.rs` — Capture + save PNG
- `src/logs.rs` — In-memory log storage

## Dependencies (workspace)

- tokio, axum, serde, serde_json, anyhow, tracing
- base64 — Basic Auth
- uuid — screenshot requestId
- tokio-tungstenite — WebSocket

## Axum Routes (exact match to browser-connector.ts)

| Method | Route | Description |
|--------|--------|-------------|
| POST | `/extension-log` | Chrome Extension → server |
| GET | `/console-logs` | Retrieve console logs |
| GET | `/console-errors` | Retrieve console errors |
| GET | `/network-errors` | Retrieve failed network requests (status ≥ 400) |
| GET | `/network-success` | Retrieve successful network requests |
| GET | `/all-xhr` | Merge and sort all network requests |
| GET | `/selected-element` | Get currently selected element |
| POST | `/selected-element` | Update selected element |
| POST | `/current-url` | Update current URL from extension |
| GET | `/current-url` | Get current URL |
| POST | `/wipelogs` | Clear all stored logs |
| POST | `/screenshot` | Save screenshot data (direct upload) |
| POST | `/capture-screenshot` | Request screenshot from extension |
| GET | `/.port` | Return actual port server is using |
| GET | `/.identity` | Return server identity info ("mcp-browser-connector-24x7") |
| POST | `/accessibility-audit` | Run Lighthouse accessibility audit |
| POST | `/performance-audit` | Run Lighthouse performance audit |
| POST | `/seo-audit` | Run Lighthouse SEO audit |
| POST | `/best-practices-audit` | Run Lighthouse best practices audit |
| WS | `/extension-ws` | Real-time bidirectional communication |

## Auth Middleware

- Basic Auth with env vars: `AUTH_USERNAME` (default: "admin"), `AUTH_PASSWORD` (default: "")
- Skip auth for `/.identity`, `/.port` endpoints

## WebSocket Protocol

### From Extension:
- `console-log` — Console messages
- `console-error` — Console errors
- `network-request` — Network requests (with status routing to success/error)
- `page-navigated` — Page navigation events
- `selected-element` — DOM element selection
- `current-url-response` — URL update response
- `screenshot-data` — Screenshot capture data (base64 PNG)
- `screenshot-error` — Screenshot error response

### To Extension:
- `take-screenshot` — Request screenshot capture (with UUID requestId)
- `server-shutdown` — Notify of server shutdown

## Screenshot Handler

- Save base64 PNG to configured directory
- Default: `~/.trios/screenshots/` (create if missing)
- Optional macOS auto-paste via AppleScript

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MCP_HOST` | `127.0.0.1` | Server host |
| `MCP_PORT` | `3025` | Server port (law L5) |
| `AUTH_USERNAME` | `admin` | Basic auth username |
| `AUTH_PASSWORD` | `` | Basic auth password |
| `ENABLE_AUTH` | `true` | Enable/disable auth |
| `SCREENSHOT_DIR` | `~/.trios/screenshots/` | Screenshot save path |

## Ring Status

- [ ] All axum routes implemented and working
- [ ] WebSocket with Chrome Extension alive
- [ ] Basic Auth middleware functional
- [ ] Screenshot capture + save working
- [ ] Log storage (consoleLogs, networkErrors, etc.) working
- [ ] Audit endpoints call SR-01 correctly
- [ ] `RING.md` present (R3)
- [ ] Separate `Cargo.toml` (R2)
- [ ] Tests pass (R4, ≥10 tests)
