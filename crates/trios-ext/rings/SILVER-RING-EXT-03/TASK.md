# TASK.md — SILVER-RING-EXT-03

## Current Task
Migrate Comet bridge and message types from legacy `src/bridge/` into this ring.

## Status
✅ Complete — code migrated and compiles independently.

## Notes
- `src/bridge/types.rs` → `src/types.rs` (no changes needed)
- `src/bridge/comet.rs` → `src/comet.rs` (adapted `crate::dom` → `trios_ext_ring_ex01`, `crate::bridge::types` → `crate::types`)
- `src/bridge/tests.rs` tests inlined into `src/types.rs` and `src/comet.rs`
