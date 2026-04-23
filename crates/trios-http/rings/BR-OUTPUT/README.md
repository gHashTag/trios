# BR-OUTPUT — Router Assembly

## Purpose

Final binary ring. Assembles all routes into one axum `Router`.

## API

```rust
use br_output::build_router;
use hr_00::AppState;

let state = AppState::new(19);
let router = build_router(state);
```

## Dependencies

- HR-00 (types)
- HR-01 (handlers)
- axum 0.7

## Invariants

- Single public function: `build_router(state: AppState) -> Router`
- No business logic here — only wiring
