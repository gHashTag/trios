# Agent Instructions UR-05

## What to touch
- `src/lib.rs` — AgentList, AgentCard components

## What NOT to touch
- UR-00 atom definitions (modify atoms in UR-00, consume here)
- UR-01 palette values (consume only)
- UR-02 component internals (use their public API)

## Testing
- AgentList renders N agent cards from atom state
- AgentCard displays status badge (Idle/Busy/Error)
