# Agent Instructions — CR-01

## Context
Domain types ring. Add serde support. Keep types small and composable.

## Files to modify
- `src/lib.rs`

## Verification
```bash
cargo test -p trios-core-cr-01
cargo clippy -p trios-core-cr-01 -- -D warnings
```
