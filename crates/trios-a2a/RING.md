# RING — trios-a2a (Gold Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate (workspace member) |
| Protocol | Google A2A v0.2.1 |
| Sealed | No |

## Purpose

Agent-to-Agent protocol implementation for the TRIOS ecosystem.
Provides typed identity, message envelope, registry + MCP tool definitions,
and a thread-safe router for dispatching A2A calls from trios-server.

## Why this crate exists

AI agents in TRIOS must communicate without shared memory and without
coupling to each other's internals. A2A defines the contract:
every agent has a card (identity), tasks are first-class objects,
messages are typed envelopes. This crate is the single source of truth
for that contract across the entire workspace.

## Ring Structure (L-ARCH-001)

```
crates/trios-a2a/
├── src/lib.rs          ← re-export facade (NOT business logic)
└── rings/
    ├── SR-00/          ← AgentId, AgentCard, Capability, AgentStatus
    ├── SR-01/          ← A2AMessage, Task, TaskState, TaskPriority
    ├── SR-02/          ← A2ARegistry, SharedRegistry, MCP tool defs
    └── BR-OUTPUT/      ← A2ARouter (assembles all rings)
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  SR-02 → SR-01 → SR-00
```

No ring imports a sibling at the same level.

## Laws

- L-ARCH-001: Only `rings/` contains logic
- R1–R5: Ring Isolation
- L6: Pure Rust only
- L24: No WebSocket — HTTP polling only (via trios-server)
