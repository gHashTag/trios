# Agent Instructions — HR-01

## Context

Route handlers ring. Uses axum extractors and HR-00 types.

## Files

- `src/lib.rs` — all handlers

## Rules

- Import types only from `hr_00`
- Never import from `br_output` in handler logic (only tests may)
- Use `tracing::info!` for all requests
- Return `impl IntoResponse` for complex handlers

## Verification

```bash
cargo test -p hr-01
cargo clippy -p hr-01 -- -D warnings
```
