# AGENTS.md — BR-APP

## Agent: ALPHA
- Build WASM target: `cargo build -p trios-ui-br-app --target wasm32-unknown-unknown --release`
- Run wasm-bindgen to generate JS glue

## Agent: BETA
- Test Chrome Extension loading with updated sidepanel.html
- Verify WASM initialization in browser console

## Rules
- R1: This ring only re-exports UR-00, no business logic
- R2: Build output goes to BRONZE-RING-EXT/dist/
- R3: index.html must match BRONZE-RING-EXT/sidepanel.html structure
