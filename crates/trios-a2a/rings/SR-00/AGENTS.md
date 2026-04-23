# AGENTS.md — SR-00 (trios-a2a)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SR-00
- Package: trios-a2a-sr00
- Role: Agent identity types (bottom of dependency graph)

## What this ring does

Defines `AgentId`, `AgentCard`, `Capability`, `AgentStatus`.
No logic. No I/O. Pure data + builder methods.

## Rules (ABSOLUTE)

- R1: Do NOT import from SR-01, SR-02, BR-OUTPUT
- L6: Pure Rust only — no async, no I/O, no subprocess
- Types only — no network calls, no file ops

## You MAY

- ✅ Add new `Capability` variants
- ✅ Add new `AgentStatus` variants
- ✅ Add fields to `AgentCard` (non-breaking)
- ✅ Add `impl` blocks and builder methods
- ✅ Add tests in `src/lib.rs`

## You MAY NOT

- ❌ Import from SR-01, SR-02, BR-OUTPUT
- ❌ Add I/O, filesystem, subprocess, network
- ❌ Add async/tokio

## Build

```bash
cargo build -p trios-a2a-sr00
cargo test -p trios-a2a-sr00
```
