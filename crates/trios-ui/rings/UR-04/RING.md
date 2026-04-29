# UR-04 — Chat UI

## Purpose
Chat panel component for Trinity sidebar: message list with bubbles and input bar.

## Exported Components
- `ChatPanel()` — full chat panel with message history and input
- `ChatBubble()` — single message bubble (role-styled)
- `ChatInputBar()` — text input with send button

## Dependencies
- UR-00 (chat state atoms)
- UR-01 (theme/palette tokens)
- UR-02 (Button, Input primitives)

## Constraints
- No direct API calls — state flows through UR-00 atoms
- No CSS hardcoding — use UR-01 design tokens only
