# AGENTS.md — EXT-00

## Invariants
- DOM and MCP are in the same ring because they have bidirectional coupling
- MCP calls `super::dom::*` for rendering responses
- DOM calls `super::mcp::mcp_send_chat()` for sending messages
- External deps use crate paths: `trios_ext_01::`, `trios_ext_02::`

## Testing
```bash
cargo check --target wasm32-unknown-unknown
```

## How to Extend
- New tab: Add to `build_ui()` tab-bar innerHTML and add panel div
- New MCP method: Add to `McpClient` impl and `handle_mcp_response()`
- New DOM panel: Add `set_xxx()` function following `set_tool_list()` pattern
