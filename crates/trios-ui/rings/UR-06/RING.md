# UR-06 — MCP Panel

## Purpose
MCP tools panel: lists available tools with status, shows connection status,
and provides tool execution interface. Reads `McpAtom` from UR-00.

## Public API
- `McpPanel() -> Element` — full MCP tools panel with connection status and tool list
- `McpToolCardProps` — props: `tool: McpTool`
- `McpToolCard(props) -> Element` — single tool card with name, description, execute button

## Dependencies
- `trios-ui-ur00` — `use_mcp_atom()`, `McpTool`
- `trios-ui-ur01` — `use_palette()`, `radius`, `spacing`, `typography`
- `trios-ui-ur02` — `Badge`, `BadgeVariant`, `Button`, `ButtonVariant`

## Ring Rules
- R1: Only ring that renders MCP tools
- R2: Tool execution goes through UR-07 WebSocket client (via UR-00 atom)
- R3: Connection status badge shows connected/disconnected state

## Compilation Status
Blocked on UR-00/UR-01/UR-02 compilation.
