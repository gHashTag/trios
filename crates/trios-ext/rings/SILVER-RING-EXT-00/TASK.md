# TASK.md — SILVER-RING-EXT-00

## Current Task
Migrate WASM entry point from legacy `src/lib.rs` into this ring.

## Status
✅ Complete — code migrated and compiles independently.

## Notes
- `src/lib.rs` → `src/lib.rs` (adapted to use ring crates instead of internal modules)
- Cross-ring chat listener wiring (DOM → MCP) lives here since it depends on both Ring-01 and Ring-02
