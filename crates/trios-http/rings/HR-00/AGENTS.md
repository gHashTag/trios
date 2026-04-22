# Agent Instructions — HR-00

## Context

Pure types ring. No I/O, no axum, no network.

## Files

- `src/lib.rs` — all types here

## Rules

- Only `serde`, `serde_json`, `tokio::sync` as dependencies
- Never add axum here
- All structs must have unit tests

## Verification

```bash
cargo test -p hr-00
cargo clippy -p hr-00 -- -D warnings
```
