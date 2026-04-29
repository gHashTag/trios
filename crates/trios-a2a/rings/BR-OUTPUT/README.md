# RING — BR-OUTPUT (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-a2a-br-output |
| Sealed | No |

## Purpose

Assembly + routing ring. The ONLY ring that sees all Silver rings simultaneously.
Provides `A2ARouter` — a thread-safe dispatcher that maps MCP tool call names
to registry operations. This is what trios-server imports to handle A2A requests.

## Why A2ARouter is Bronze

The router has no logic of its own — it's a pattern-match dispatch table.
All logic lives in SR-02 (registry). Bronze exists to:
1. Assemble all rings into one usable interface
2. Own the `match tool { ... }` dispatch without polluting Silver rings
3. Be the single import point for trios-server

## API Surface (pub)

| Type | Role |
|------|------|
| `A2ARouter` | MCP tool dispatcher → A2ARegistry |
| `A2ARouter::new()` | Creates router with empty registry |
| `A2ARouter::call(tool, params)` | Dispatches tool call by name |
| `A2ARouter::registry()` | Returns SharedRegistry for direct access |

## Dispatched tools

- `a2a_list_agents`
- `a2a_send`
- `a2a_broadcast`
- `a2a_assign_task`
- `a2a_task_status`
- `a2a_update_task`

## Dependencies

- SR-00, SR-01, SR-02 (all Silver rings)
- `serde_json`

## Laws

- R4: Bronze files stay in Bronze — never move logic up to Silver
- No business logic in `A2ARouter::call` — only dispatch
- All real logic lives in SR-02 registry methods
