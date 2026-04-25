# AGENTS.md — BR-EXT

## Invariants
- This is the ONLY ring with `crate-type = ["cdylib", "rlib"]`
- All other rings are library crates
- build.rs copies WASM artifacts to `../../extension/dist/`
- Legacy file names (trios_ext.js, trios_ext_bg.wasm) for backward compat

## Testing
```bash
cargo check --target wasm32-unknown-unknown
wasm-pack build --target no-modules --out-dir pkg
```

## How to Extend
- Add new ring dependency to Cargo.toml
- Import and use in `run()` or add re-export
