# EXT-00 — Shell & Transport

Sidepanel UI builder (DOM) + MCP client (HTTP transport).

## API

### DOM (`dom` module)
- `document()` → `web_sys::Document`
- `build_ui()` — Build the full sidepanel UI with 6 tabs
- `set_status(text)` — Update status bar
- `append_message(role, text)` — Append chat message
- `set_agent_list(text)` — Update agents panel
- `set_tool_list(json)` — Render tools from JSON
- `set_issue_list(json)` — Render issues from JSON
- `set_artifacts(json)` — Render BR-OUTPUT artifacts
- `append_artifact(json)` — Append single artifact
- `STYLE` — CSS constants for the sidepanel

### MCP (`mcp` module)
- `mcp_connect()` — Initialize MCP connection
- `mcp_send_chat(msg)` — Send chat message
- `mcp_list_tools()` — Request tools list
- `mcp_list_issues()` — Request issues list
- `mcp_list_agents()` — Request agents list
- `mcp_ping()` — Health check
- `McpClient` — Internal MCP JSON-RPC client

## Dependencies
- `trios-ext-01` — Artifact rendering (CSS + HTML)
- `trios-ext-02` — Settings (API key)

## Usage
```rust
use trios_ext_00::{build_ui, mcp_connect, set_status};

build_ui()?;
mcp_connect()?;
```
