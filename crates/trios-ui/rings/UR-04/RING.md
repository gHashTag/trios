# UR-04 — Chat UI

## Purpose
Chat interface: message list with role-colored bubbles, input bar with send button,
and auto-scroll to latest message. Reads/writes `ChatAtom` from UR-00.

## Public API
- `ChatPanel() -> Element` — full chat panel with messages and input
- `ChatBubbleProps` — props: `message: ChatMessage`
- `ChatBubble(props) -> Element` — single message bubble (user/assistant/system roles)
- `ChatInputBar() -> Element` — text input + send button, writes to `ChatAtom`

## Dependencies
- `trios-ui-ur00` — `use_chat_atom()`, `ChatMessage`, `MessageRole`
- `trios-ui-ur01` — `use_palette()`, `radius`, `spacing`, `typography`

## Ring Rules
- R1: Only ring that renders chat messages
- R2: Message sending writes to UR-00 `ChatAtom` (no direct WebSocket calls)
- R3: Auto-scrolls to bottom on new messages

## Compilation Status
Blocked on UR-00/UR-01 compilation.
