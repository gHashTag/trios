# Experience: Rust-Only Extension Rewrite (#156)

**Date:** 2026-04-21
**Agent:** OPENCODE
**Issue:** #156

## Context

Chrome Extension had handwritten JS files (`background.js`, `sidepanel.js`) violating L9 (Rust-Only Extension). The `trios-ext` crate already had full Rust/WASM implementation via `web-sys`.

## Decision

1. Deleted `extension/sidepanel.js` (73 lines handwritten JS)
2. Deleted `extension/background.js` (21 lines handwritten JS)
3. Deleted `extension/style.css` (CSS now in Rust/WASM)
4. Made `trios-ext/src/lib.rs` detect window vs service-worker context
5. Moved `bg-sw.js` (3-line WASM loader) into `extension/dist/`
6. Added CI check: `find extension -name '*.js' -not -path '*/dist/*'` must be empty
7. Fixed merge conflicts in `Cargo.toml` and `trios-claude/src/lib.rs`

## Key Learnings

- Chrome MV3 service workers require JS entry point, but wasm-bindgen glue is auto-generated
- `#[wasm_bindgen(start)]` must handle both window (sidepanel) and worker (background) contexts
- `web_sys::window().is_none()` detects service worker context reliably
- wasm-pack `--target web` generates ES module glue compatible with Chrome extensions

## Metrics

- Tests: 8 passed (up from 3)
- Clippy: 0 warnings
- WASM size: 96KB (optimized)
- JS files outside dist/: 0
