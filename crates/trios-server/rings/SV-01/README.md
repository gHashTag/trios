# SV-01 — MCP Protocol Handlers

## Purpose
JSON-RPC 2.0 MCP protocol implementation over SSE + WebSocket + REST.

## Transports
- SSE: `GET /sse`
- WebSocket: `GET /ws`
- REST: `POST /api/chat`

## API
- `handle_mcp_request(req: JsonRpcRequest) -> JsonRpcResponse`
- `McpSession` — per-connection session state
