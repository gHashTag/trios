# HR-01 — Route Handlers

## Purpose

axum route handlers for trios-http:
- `health()` → GET /health
- `status()` → GET /api/status
- `chat()` → POST /api/chat

## API

```rust
use hr_01::{health, status, chat};
```

## Dependencies

- HR-00 (types)
- axum 0.7

## Invariants

- All handlers return JSON
- No direct DB/FS access
- Logging via `tracing::info!`
