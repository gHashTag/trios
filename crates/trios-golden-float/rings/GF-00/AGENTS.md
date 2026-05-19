# AGENTS.md — GF-00 (trios-golden-float)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: GF-00
- Package: trios-golden-float-gf00
- Role: phi constants + GF16 newtype (bottom of dependency graph)

## What this ring does

Defines `PHI`, `INV_PHI`, and `GF16(u16)`. Pure constants and a newtype.

## Rules (ABSOLUTE)

- R1: Do NOT import from GF-01, BR-OUTPUT
- R9: No sibling imports
- L6: Pure Rust only — no async, no I/O, no FFI here
- Types only — no business logic

## You MAY

- ✅ Add new phi-derived constants
- ✅ Add `impl` blocks for `GF16` (non-breaking)
- ✅ Add tests in `src/lib.rs`

## You MAY NOT

- ❌ Import from GF-01, BR-OUTPUT
- ❌ Add I/O, filesystem, subprocess, network
- ❌ Add async/tokio
- ❌ Break the anchor `phi^2 + phi^-2 = 3`

## Build

```bash
cargo check -p trios-golden-float-gf00
cargo test -p trios-golden-float-gf00
```
