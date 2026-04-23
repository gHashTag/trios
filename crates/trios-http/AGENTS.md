# Agent Instructions — trios-http

## Context

This crate is the HTTP REST gateway layer between the internet and trios-server (WS/MCP core).
It follows Ring architecture HR-00/HR-01/BR-OUTPUT per issue #238.

## Ring Responsibilities

- **HR-00**: Define `AppState`, `ChatRequest`, `ChatResponse`, `StatusResponse`
- **HR-01**: Implement axum route handlers using HR-00 types
- **BR-OUTPUT**: Assemble the final `Router` and expose `build_router(state)`

## Files to Modify

- `rings/HR-00/src/lib.rs` — types only, no I/O
- `rings/HR-01/src/lib.rs` — route handlers
- `rings/BR-OUTPUT/src/lib.rs` — router assembly

## Rules

- HR-00 MUST NOT depend on HR-01 or BR-OUTPUT
- HR-01 MAY depend on HR-00
- BR-OUTPUT depends on HR-00 + HR-01
- Zero hardcoded ports — read from env `TRIOS_PORT` (default 9005)
- All handlers must return JSON

## Verification

```bash
cargo check -p trios-http
cargo test -p trios-http
cargo clippy -p trios-http -- -D warnings
```
