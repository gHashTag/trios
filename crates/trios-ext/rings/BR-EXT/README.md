# BR-EXT — WASM Entry Point

Wires all EXT rings together. This is the single WASM binary that `wasm-pack build` compiles.

## API
- `run()` — wasm_bindgen(start) entry point
- Re-exports all `trios_ext_03` wasm_bindgen functions (injectors)

## Dependencies
- `trios-ext-00` — Shell & Transport (UI + MCP)
- `trios-ext-01` — Artifact Rendering
- `trios-ext-02` — Settings
- `trios-ext-03` — Content Injectors

## Build
```bash
cd crates/trios-ext/rings/BR-EXT
wasm-pack build --target no-modules --out-dir pkg
```

## Startup Sequence
1. `console_error_panic_hook::set_once()` — panic handler
2. `trios_ext_02::load_api_key()` — Load API key from chrome.storage
3. `trios_ext_00::build_ui()` — Build sidepanel UI
4. `trios_ext_00::mcp_connect()` — Connect to MCP server (optional)
