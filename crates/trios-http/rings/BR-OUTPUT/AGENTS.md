# Agent Instructions — BR-OUTPUT

## Context

Router assembly ring. Only wires together HR-00 + HR-01. No logic.

## Files

- `src/lib.rs` — only `build_router()`

## Rules

- Single public function: `build_router`
- No business logic
- Add middleware via `ServiceBuilder` layers only

## Verification

```bash
cargo test -p br-output
cargo clippy -p br-output -- -D warnings
```
