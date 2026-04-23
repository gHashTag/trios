# TASK.md — SILVER-RING-EXT-01

## Current Task
Migrate DOM bridge code from legacy `src/dom.rs` into this ring.

## Status
✅ Complete — code migrated and compiles independently.

## Notes
- `setup_chat_listener` was removed from this ring (it had a cross-ring dependency on MCP)
- Chat listener wiring is now handled by SILVER-RING-EXT-00 (entry ring)
