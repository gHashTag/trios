# AGENTS.md — EXT-02

## Invariants
- No dependencies on other EXT rings
- Uses thread_local! for WASM-compatible state
- All chrome.storage calls are async via JS interop

## Testing
```bash
cargo check --target wasm32-unknown-unknown
```

## How to Extend
- New setting: Add thread_local! static + getter/setter + chrome.storage key
