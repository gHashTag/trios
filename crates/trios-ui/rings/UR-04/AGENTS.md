# Agent Instructions UR-04

## What to touch
- `src/lib.rs` — ChatPanel, ChatBubble, ChatInputBar components

## What NOT to touch
- UR-00 atom definitions (modify atoms in UR-00, consume here)
- UR-01 palette values (consume only)
- UR-02 component internals (use their public API)

## Testing
- Component rendering via Dioxus test harness
- Verify ChatBubble renders MessageRole variants (User, Assistant, System, Tool)
