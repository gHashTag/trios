# Agent Instructions — CR-00

## Context
You are implementing the identity ring of trios-core. This is the foundation — keep it minimal and dependency-free.

## Files to modify
- `src/lib.rs` — core types

## Forbidden
- No external dependencies (only std)
- No async
- No unsafe

## Verification
```bash
cargo test -p trios-core-cr-00
cargo clippy -p trios-core-cr-00 -- -D warnings
cargo check -p trios-core-cr-00 --target wasm32-unknown-unknown
```
