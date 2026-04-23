# HR-00 — Core Types

## Purpose

Defines shared data types for trios-http:
- `AppState` — shared state (agent count, tool count)
- `ChatRequest` — POST /api/chat body
- `ChatResponse` — POST /api/chat response
- `StatusResponse` — GET /api/status response

## API

```rust
use hr_00::{AppState, ChatRequest, StatusResponse};

let state = AppState::new(19);
```

## Dependencies

None (only serde, tokio::sync)

## Invariants

- No I/O in this ring
- No dependencies on HR-01 or BR-OUTPUT
- All types derive `Serialize`/`Deserialize`
