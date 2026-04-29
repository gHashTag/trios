# AGENTS.md — UR-06

## Agent: ALPHA
- Add tool execution result display
- Add tool parameter form (dynamic inputs from tool schema)
- Add tool execution history

## Agent: BETA
- Test McpPanel with empty tool list
- Test McpToolCard rendering with sample tools
- Test connection status badge states

## Rules
- R1: Tool execution delegates to UR-07 via UR-00 atom
- R2: Tool cards use Button and Badge from UR-02
- R3: No direct WebSocket calls in this ring
