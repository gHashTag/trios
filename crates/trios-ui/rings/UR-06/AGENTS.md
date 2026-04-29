# Agent Instructions UR-06

## What to touch
- `src/lib.rs` — McpPanel, McpToolCard components

## What NOT to touch
- UR-00 atom definitions (modify atoms in UR-00, consume here)
- UR-01 palette values (consume only)
- UR-02 component internals (use their public API)

## Testing
- McpPanel renders N tool cards from atom state
- McpToolCard shows tool name, description, status badge
