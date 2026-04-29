# UR-06 — MCP Panel

## Purpose
MCP tools panel for Trinity sidebar: displays available MCP tools and their status.

## Exported Components
- `McpPanel()` — full tools panel with tool cards
- `McpToolCard(props: McpToolCardProps)` — single MCP tool card

## Exported Types
- `McpToolCardProps` — props struct for tool card

## Dependencies
- UR-00 (MCP state atoms)
- UR-01 (theme/palette tokens)
- UR-02 (Button, Badge primitives)

## Constraints
- No direct API calls — state flows through UR-00 atoms
- No CSS hardcoding — use UR-01 design tokens only
