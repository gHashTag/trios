# AGENTS.md — SR-02 (trios-a2a)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-02
- Package: trios-a2a-sr02
- Role: In-memory A2A registry + MCP tool definitions

## What this ring does

Stores agents, tasks, messages in `A2ARegistry`. Exposes `mcp_tool_definitions()`
for trios-server to register. Thread-safe via `Arc<Mutex<>>`.

## Rules (ABSOLUTE)

- R1: Do NOT import from BR-OUTPUT
- L6: Pure Rust only
- L24: No WebSocket — HTTP only
- Use `Arc<Mutex<>>` for shared state — no async locks (tokio::Mutex)
- Registry is in-memory only — no filesystem, no DB

## You MAY

- ✅ Add new MCP tool definitions to `mcp_tool_definitions()`
- ✅ Add new registry methods (e.g., `get_agent`, `list_tasks`)
- ✅ Add message history queries
- ✅ Add tests

## You MAY NOT

- ❌ Import from BR-OUTPUT
- ❌ Add persistent storage (filesystem, DB)
- ❌ Add WebSocket or async transport
- ❌ Use `tokio::Mutex` — use `std::sync::Mutex` only

## Build

```bash
cargo build -p trios-a2a-sr02
cargo test -p trios-a2a-sr02
```
