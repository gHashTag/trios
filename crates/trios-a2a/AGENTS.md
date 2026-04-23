# AGENTS.md — trios-a2a

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Crate: trios-a2a
- Metal: Gold
- Protocol: Google A2A v0.2.1
- Repo: gHashTag/trios

## What this crate does

Agent-to-Agent protocol: typed identity (SR-00), message envelope + tasks (SR-01),
MCP-compatible registry (SR-02), thread-safe router (BR-OUTPUT).

## Ring map

| Ring | Package | Role | Sealed |
|------|---------|------|--------|
| SR-00 | trios-a2a-sr00 | AgentId, AgentCard, Capability | No |
| SR-01 | trios-a2a-sr01 | A2AMessage, Task, TaskState | No |
| SR-02 | trios-a2a-sr02 | A2ARegistry, MCP tool defs | No |
| BR-OUTPUT | trios-a2a-br-output | A2ARouter (assembly) | No |

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- L-ARCH-001: Logic lives in `rings/` — `src/lib.rs` is RE-EXPORT ONLY
- R1–R5: Ring Isolation — no cross-imports except via Cargo.toml
- L6: Pure Rust only
- L24: No WebSocket — HTTP only
- Each ring has its own AGENTS.md — read it before touching that ring

## You MAY NOT

- ❌ Add business logic to `src/lib.rs`
- ❌ Add files outside `rings/` (except src/lib.rs facade)
- ❌ Cross-import Silver rings directly without Cargo.toml declaration
- ❌ Add async runtime without explicit approval
