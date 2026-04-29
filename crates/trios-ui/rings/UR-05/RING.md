# UR-05 — Agent UI

## Purpose
Agent list, agent cards with status badges, and agent selection.
Reads `AgentsAtom` from UR-00.

## Public API
- `AgentList() -> Element` — full agent list panel with cards
- `AgentCardProps` — props: `agent: Agent`
- `AgentCard(props) -> Element` — single agent card with name, status badge, description

## Dependencies
- `trios-ui-ur00` — `use_agents_atom()`, `use_chat_atom()`, `Agent`, `AgentStatus`
- `trios-ui-ur01` — `use_palette()`, `radius`, `spacing`, `typography`
- `trios-ui-ur02` — `Badge`, `BadgeVariant`

## Ring Rules
- R1: Only ring that renders agent list
- R2: Agent selection writes to UR-00 `ChatAtom` (sets active agent)
- R3: Status badge uses UR-02 `Badge` component

## Compilation Status
Blocked on UR-00/UR-01/UR-02 compilation.
