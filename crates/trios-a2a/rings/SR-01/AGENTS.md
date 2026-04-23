# AGENTS.md — SR-01 (trios-a2a)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-01
- Package: trios-a2a-sr01
- Role: A2A message protocol + task lifecycle

## What this ring does

Defines `A2AMessage` envelope and `Task` with full state machine.
No registry logic — that's SR-02. No routing — that's BR-OUTPUT.

## Rules (ABSOLUTE)

- R1: Do NOT import from SR-02 or BR-OUTPUT
- L6: Pure Rust only — no async, no I/O
- All timestamps must be RFC3339 UTC
- All IDs must be UUID v4

## You MAY

- ✅ Add new `A2AMessageType` variants
- ✅ Add new `TaskState` variants (document state transitions)
- ✅ Add fields to `Task` (non-breaking)
- ✅ Add factory methods to `A2AMessage`
- ✅ Add tests

## You MAY NOT

- ❌ Import from SR-02 or BR-OUTPUT
- ❌ Add registry/storage logic
- ❌ Add async or network calls
- ❌ Use non-UUID ID generation

## Build

```bash
cargo build -p trios-a2a-sr01
cargo test -p trios-a2a-sr01
```
