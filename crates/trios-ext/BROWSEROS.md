# BrowserOS — A2A MCP Browser Control

## Vision

Chrome extension (trios-ext) becomes a **first-class A2A agent**.
AI agents in trios-server can control the browser: open tabs, navigate,
read DOM, inject text, click buttons — via standard MCP tool calls.

## Why this is powerful

Currently agents work in the dark: they can read GitHub issues and write code,
but they cannot *see* what's happening in the browser. BrowserOS closes this gap:
agents get eyes + hands in Chrome.

## Architecture

```
trios-server (Rust)
    ↓ HTTP POST /a2a
    ↓ JSON: { tool: "browser_open_tab", params: { url: "..." } }

Chrome Extension (WASM)
    EXT-02 (MCP Client)
        ↓ polls GET /mcp/browser-commands every 2s
        ↓ receives pending commands
    EXT-04 (BrowserOS Agent)
        ↓ executes: tabs, navigation, DOM, clipboard, screenshot
        ↓ reports result back via POST /mcp/browser-result
```

## A2A Registration

On extension load, EXT-04 registers itself as an A2A agent:
```json
{
  "tool": "a2a_register_agent",
  "params": {
    "id": "browser-agent-{tab_id}",
    "name": "BrowserOS Agent",
    "capabilities": ["browser_control", "dom_read", "dom_write"]
  }
}
```

## MCP Tools exposed by BrowserOS agent

| Tool | Description |
|------|-------------|
| `browser_open_tab` | Open URL in new tab |
| `browser_close_tab` | Close tab by id |
| `browser_navigate` | Navigate current tab to URL |
| `browser_get_url` | Get current tab URL |
| `browser_get_title` | Get current tab title |
| `browser_get_dom` | Get full page HTML |
| `browser_query_selector` | Query DOM element |
| `browser_click` | Click element by selector |
| `browser_type` | Type text into element |
| `browser_scroll` | Scroll page |
| `browser_screenshot` | Capture viewport as base64 PNG |
| `browser_eval` | Evaluate JS expression (sandboxed) |

## Ring Mapping

| Ring | Role |
|------|------|
| EXT-02 | MCP HTTP client — polls server, sends results |
| EXT-04 | BrowserOS agent — executes browser commands |
| EXT-03 | Content injectors — GitHub, Claude.ai |
| EXT-00 | WASM entry — bootstraps all rings |

## Security Model

- Commands execute only on **whitelisted domains** (configurable in EXT-02 settings)
- `browser_eval` is sandboxed — no access to extension APIs
- All commands logged to `chrome.storage.local` with TTL 1h
- Agent can be disabled via extension popup toggle
