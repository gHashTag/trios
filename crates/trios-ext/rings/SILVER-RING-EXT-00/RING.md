# RING.md — SILVER-RING-EXT-00

## Metal
Silver

## Crate
trios-ext

## Package
trios-ext-ring-ex00

## Purpose
WASM entry point — wires together all other Silver rings and serves as the main entry for the Trios Chrome Extension.

## Dependencies (internal rings)
- SILVER-RING-EXT-01 (DOM bridge)
- SILVER-RING-EXT-02 (MCP client + BG)
- SILVER-RING-EXT-03 (Comet bridge / Types)

## Dependencies (external)
- `wasm-bindgen`
- `web-sys`
- `js-sys`
- `log`
- `console_error_panic_hook`

## API Surface
- `#[wasm_bindgen(start)] pub fn run()` — WASM entry point
- Re-exports from all other rings

## Build
```bash
cargo build -p trios-ext-ring-ex00
cargo test -p trios-ext-ring-ex00
```
