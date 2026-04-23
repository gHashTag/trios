# RING — SR-01 (trios-a2a)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-a2a-sr01 |
| Sealed | No |

## Purpose

Message protocol ring. Defines the A2A message envelope and task lifecycle.
Every interaction between agents in TRIOS flows through `A2AMessage`.
Tasks are the unit of work — they have a state machine, priority, and ownership.

## Why tasks are in SR-01 and not SR-02

`Task` is a protocol-level concept, not a registry concept.
The registry (SR-02) stores and queries tasks, but the task's shape
(fields, state machine, priority) is part of the protocol contract.
Separating them means SR-02 can evolve its storage without touching the protocol.

## API Surface (pub)

| Type | Role |
|------|------|
| `A2AMessage` | Universal message envelope |
| `A2AMessageType` | Direct, Broadcast, TaskAssign, TaskUpdate, TaskResult, Heartbeat, Error |
| `Task` | Unit of work with full lifecycle |
| `TaskState` | Pending → Assigned → InProgress → Completed/Failed/Cancelled |
| `TaskPriority` | Low, Medium, High, Critical (Ord implemented) |

## Dependencies

- SR-00 (AgentId)
- `serde`, `uuid` (v4), `chrono` (UTC timestamps)

## Laws

- R1: No imports from SR-02, BR-OUTPUT
- L6: Pure Rust only
- All timestamps: RFC3339 UTC via `chrono::Utc::now().to_rfc3339()`
- All IDs: UUID v4 via `uuid::Uuid::new_v4()`
