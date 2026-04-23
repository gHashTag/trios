# AGENTS.md — UR-07

## Agent: ALPHA
- Add new API methods (e.g., `get_agent_status`, `execute_tool`)
- Modify JSON-RPC protocol messages

## Agent: BETA
- Test WebSocket connection to trios-server
- Verify message serialization/deserialization

## Rules
- R1: All methods use JSON-RPC 2.0 format
- R2: Server URL is configurable via `SERVER_WS_URL` constant
- R3: Callbacks use `FnMut` trait for Dioxus Signal compatibility
