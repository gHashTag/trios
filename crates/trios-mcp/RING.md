# RING.md — trios-mcp

## Ring: GOLD

**Version:** 0.1.0
**Status:** 🟡 In Development

---

## Purpose

`trios-mcp` is a Rust MCP (Model Context Protocol) client adapter for connecting Perplexity Agents to browser-tools-server via WebSocket tunnel.

---

## Ring Architecture

```
trios-mcp/                    🥇 GOLD RING
├── src/lib.rs                🟡 Entry point (aggregator)
├── RING.md                   📄 This file
├── Cargo.toml                📦 Dependencies
└── rings/
    ├── SR-00/                🥈 SILVER RING 00: MCP Protocol Core
    │   ├── src/lib.rs
    │   └── Cargo.toml
    └── SR-01/                🥈 SILVER RING 01: Auth + WebSocket
        ├── src/lib.rs
        └── Cargo.toml
```

---

## SR-00: MCP Protocol Core

**Responsibility:** MCP types and JSON-RPC 2.0

- `JsonRpcRequest` / `JsonRpcResponse` — JSON-RPC 2.0
- `McpTool` / `McpResource` — MCP entities
- `McpContent` — content types (text, image, resource)
- `JsonRpcMessage` — unified Request/Response type

---

## SR-01: Auth + WebSocket

**Responsibility:** Connection to browser-tools-server

- `AuthConfig` — Basic Auth (username:password → Base64)
- `ConnectionConfig` — host, port, TLS, reconnect settings
- `McpWebSocketClient` — WebSocket client with auto-reconnect
- `health_check()` — HTTP /.identity check

---

## Configuration

```rust
use trios_mcp::{ConnectionConfig, McpClient};

let config = ConnectionConfig::new("127.0.0.1", 3025)
    .with_auth(AuthConfig::new("perplexity", "test123"));

let mut client = McpClient::new(config);
client.connect().await?;
```

---

## Perplexity Integration

```json
{
  "mcpServers": [{
    "url": "wss://playras-macbook-pro-1.tail01804b.ts.net",
    "transport": "websocket",
    "headers": {
      "Authorization": "Basic cGVycGxleGl0eTp0ZXN0MTIz"
    }
  }]
}
```

---

## Dependencies

| Crate | Version | Purpose |
|-------|--------|---------|
| tokio | workspace | Async runtime |
| serde | workspace | Serialization |
| serde_json | workspace | JSON |
| anyhow | workspace | Error handling |
| tracing | workspace | Logging |
| tokio-tungstenite | 0.24 | WebSocket |
| base64 | 0.22 | Basic Auth |
| url | 2.5 | URL parsing |
| reqwest | 0.12 | HTTP health check |

---

## Tests

```bash
# Unit tests
cargo test -p trios-mcp

# Clippy
cargo clippy -p trios-mcp -- -D warnings
```

---

## TODO

- [ ] Implement full MCP protocol (tools/list, tools/call, resources/list)
- [ ] Support streaming responses
- [ ] SSE fallback when WebSocket unavailable
- [ ] Integrate with trios-server

---

## Connections

- **browser-tools-server:**3025 — target server
- **Perplexity Agents** — MCP API consumer
- **trios-server** — potential integration
