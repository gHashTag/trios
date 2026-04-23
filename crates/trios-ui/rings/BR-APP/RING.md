# BR-APP — Bronze Output Ring

## Purpose
Bronze ring for trios-ui. Contains the HTML entry point and serves as the WASM build target.

## Build Pipeline
```bash
# 1. Build WASM
cargo build -p trios-ui-br-app --target wasm32-unknown-unknown --release

# 2. Generate JS glue
wasm-bindgen --target web \
  target/wasm32-unknown-unknown/release/trios_ui_br_app.wasm \
  --out-dir crates/trios-ext/rings/BRONZE-RING-EXT/dist/

# 3. Load in Chrome Extension
# BRONZE-RING-EXT/sidepanel.html loads ./dist/trios_ui_br_app.js
```

## Dependencies
- UR-00 (WASM entry point)

## Output
- `index.html` — HTML entry point (also copied to BRONZE-RING-EXT as sidepanel.html)
- WASM binary via `trios_ui_br_app.wasm`
