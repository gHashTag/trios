# AGENTS.md — UR-04

## Agent: ALPHA
- Add markdown rendering in ChatBubble
- Add syntax highlighting for code blocks
- Add message streaming support (partial updates)

## Agent: BETA
- Test ChatPanel rendering with empty messages
- Test ChatBubble role colors (user/assistant/system)
- Test ChatInputBar send behavior

## Rules
- R1: ChatBubble uses MessageRole for color coding
- R2: Input bar clears after send
- R3: No direct API calls — all via UR-00 atoms
