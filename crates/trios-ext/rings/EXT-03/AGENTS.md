# AGENTS.md — EXT-03

## Invariants
- GitHub injector uses `trios_ext_00::document()` for DOM access
- Claude injector uses `web_sys::window()` directly
- Both injectors are idempotent (safe to call multiple times)
- Bootstrap loaders (github-bootstrap.js, claude-bootstrap.js) are the only JS (I9 exception)

## Testing
```bash
cargo check --target wasm32-unknown-unknown
```

## How to Extend
- New site injector: Add functions following the claude.rs pattern
- Register in manifest.json content_scripts
