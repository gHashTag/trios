# Agent Instructions — SV-01

## Context
MCP protocol ring. Three transports: SSE, WebSocket, REST. All tested as of fc4458bf.

## Forbidden
- No handwritten JS
- No panic on malformed input — return JSON-RPC error response

## Verification
```bash
cargo test -p trios-server
# Manual: curl http://localhost:9005/sse
# Manual: wscat -c ws://localhost:9005/ws
```
