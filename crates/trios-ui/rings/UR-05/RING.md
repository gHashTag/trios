# UR-05 — Agent UI

## Purpose
Agent list and agent card components for Trinity sidebar.

## Exported Components
- `AgentList()` — scrollable list of agent cards
- `AgentCard(props: AgentCardProps)` — single agent status card

## Exported Types
- `AgentCardProps` — props struct for agent card

## Dependencies
- UR-00 (agent state atoms)
- UR-01 (theme/palette tokens)
- UR-02 (Button, Badge primitives)

## Constraints
- No direct API calls — state flows through UR-00 atoms
- No CSS hardcoding — use UR-01 design tokens only
