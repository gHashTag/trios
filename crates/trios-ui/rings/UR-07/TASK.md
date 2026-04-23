# TASK.md — UR-07

## Current
- [x] WebSocket client with connect/disconnect
- [x] `send_chat()` — JSON-RPC agents/chat
- [x] `list_agents()` — JSON-RPC agents/list
- [x] `list_tools()` — JSON-RPC tools/list
- [x] `connect_with_callback()` — FnMut callback for incoming messages

## Next
- [ ] Add reconnection with exponential backoff
- [ ] Add connection state tracking (Connecting/Connected/Disconnected/Error)
- [ ] Add message queue for offline support
- [ ] Add authentication token support
