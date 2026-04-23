# AGENTS.md — BR-OUTPUT (trios-a2a)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: BR-OUTPUT
- Package: trios-a2a-br-output
- Role: A2ARouter — assembly + MCP dispatch

## What this ring does

Assembles SR-00/01/02 into `A2ARouter`. Dispatches MCP tool calls by name
to the corresponding registry method. No business logic here.

## Rules (ABSOLUTE)

- R4: Bronze files stay in Bronze — never move `match` logic into Silver rings
- No business logic in `call()` — only dispatch + parameter extraction
- All real logic must live in SR-02

## You MAY

- ✅ Add new tool dispatches to `A2ARouter::call()` match arms
- ✅ Add new public methods to `A2ARouter` (delegating to registry)
- ✅ Add tests

## You MAY NOT

- ❌ Add business logic in `call()` beyond parameter parsing + dispatch
- ❌ Add storage/state to `A2ARouter` beyond `SharedRegistry`
- ❌ Bypass SR-02 methods — all mutations go through registry

## Build

```bash
cargo build -p trios-a2a-br-output
cargo test -p trios-a2a-br-output
```
