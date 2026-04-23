# TASK.md — SILVER-RING-EXT-02

## Current Task
Migrate MCP client and background service code from legacy `src/mcp.rs` and `src/bg.rs` into this ring.

## Status
✅ Complete — code migrated and compiles independently.

## Notes
- `src/mcp.rs` → `src/mcp.rs` (adapted `crate::dom` → `trios_ext_ring_ex01`)
- `src/bg.rs` → `src/bg.rs` (no changes needed)
