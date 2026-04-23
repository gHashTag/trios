# Plan: trios-ui — Dioxus Sidebar App

## Overview

Create `crates/trios-ui` as a Dioxus web app that compiles to WASM and runs as a Chrome Extension sidebar. It connects to `trios-server` via WebSocket (port 9005) and provides a chat interface, agent list, and MCP tools panel.

## Architecture

```
crates/trios-ui/
├── rings/
│   ├── UR-00/  ← WASM entry (Dioxus launch, wires all rings)
│   ├── UR-01/  ← App shell (layout, routing)
│   ├── UR-02/  ← Sidebar shell (tabs, header, status bar)
│   ├── UR-03/  ← Chat component (messages, input)
│   ├── UR-04/  ← Agent list component
│   ├── UR-05/  ← MCP tools component
│   ├── UR-06/  ← State management (Dioxus signals/context)
│   ├── UR-07/  ← API client (WebSocket to trios-server)
│   ├── UR-08/  ← Theme/branding (Trinity brand kit CSS)
│   └── BR-APP/ ← Output ring (HTML, CSS, dist/)
```

## Dependency Graph

```
UR-07 (API client) ← standalone
UR-08 (Theme)      ← standalone
UR-06 (State)      ← depends on UR-07
UR-03 (Chat)       ← depends on UR-06, UR-08
UR-04 (Agents)     ← depends on UR-06, UR-08
UR-05 (Tools)      ← depends on UR-06, UR-08
UR-02 (Sidebar)    ← depends on UR-03, UR-04, UR-05, UR-08
UR-01 (App shell)  ← depends on UR-02, UR-08
UR-00 (Entry)      ← depends on UR-01
BR-APP (Output)    ← dist/ from UR-00 WASM build
```

## Connection to BRONZE-RING-EXT

```
BRONZE-RING-EXT/sidepanel.html
  → loads dist/ from trios-ui BR-APP
  → sidebar opens when extension button clicked

BRONZE-RING-EXT/sw.js
  → background service worker
  → sets up sidePanel behavior
```

## MVP Scope (Phase 1)

1. **UR-07**: WebSocket client connecting to `ws://localhost:9005/ws`
2. **UR-06**: Basic state (connection status, messages, agents)
3. **UR-08**: Trinity brand CSS (black/gold theme)
4. **UR-03**: Chat component (send/receive messages)
5. **UR-02**: Sidebar shell with tabs
6. **UR-00**: Dioxus WASM entry point
7. **BR-APP**: Build output + HTML
8. **Connect**: BR-APP dist/ → BRONZE-RING-EXT sidepanel.html

## Tech Stack

- **Dioxus 0.6** (latest stable) — React-like UI framework for Rust
- **wasm-bindgen** — Rust ↔ JS interop
- **web-sys** — Web API bindings
- **serde/serde_json** — JSON serialization

## Build Command

```bash
# Build trios-ui for WASM
dx build --platform web --release

# Or manual:
cargo build -p trios-ui-ring-ur00 --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir dist/ target/.../trios_ui.wasm
```
