# UR-00 — WASM Entry Point

## Purpose
Dioxus WASM entry point for the trios-ui sidebar. Launches the Dioxus app,
injects Trinity theme CSS, connects to trios-server via WebSocket (UR-07),
and renders the full sidebar UI (Chat, Agents, Tools tabs).

## Dependencies
- UR-07 (WebSocket API client)
- UR-08 (Trinity brand theme/CSS)

## Ring Rules
- R1: This is the ONLY ring with `#[wasm_bindgen(start)]` entry point
- R2: All UI state lives here as Dioxus Signals
- R3: No direct DOM manipulation — use Dioxus rsx! only
