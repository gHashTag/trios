# RING — SR-02 (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-a2a-sr02 |
| Sealed | No |

## Purpose

Registry + MCP tools ring. Provides in-memory storage of agents, tasks, and messages.
Exposes MCP-compatible tool definitions that trios-server registers as callable tools.

## Why the registry is Silver, not Bronze

BR-OUTPUT (Bronze) is output-only — it assembles and routes, but doesn't store state.
The registry is stateful business logic: it mutates, queries, and enforces invariants.
Putting storage in Silver keeps Bronze purely orchestration.

## API Surface (pub)

| Type / fn | Role |
|-----------|------|
| `A2ARegistry` | In-memory store: agents, tasks, messages |
| `SharedRegistry` | `Arc<Mutex<A2ARegistry>>` — thread-safe wrapper |
| `shared_registry()` | Constructor for SharedRegistry |
| `mcp_tool_definitions()` | Returns Vec<Value> of MCP tool schemas |

## MCP Tools exposed

- `a2a_list_agents` — list all registered agents
- `a2a_send` — send direct message
- `a2a_broadcast` — broadcast to all agents
- `a2a_assign_task` — create + assign task

## Dependencies

- SR-00 (AgentId, AgentCard)
- SR-01 (A2AMessage, Task, TaskState)
- `serde_json`, `chrono`

## Laws

- R1: No imports from BR-OUTPUT
- L6: Pure Rust only
- L24: No WebSocket — registry is in-memory, transport is trios-server's responsibility
- `Arc<Mutex<>>` for thread safety — no async locks
