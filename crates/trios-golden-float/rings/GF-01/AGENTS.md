# AGENTS.md — GF-01 (trios-golden-float)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: GF-01
- Package: trios-golden-float-gf01
- Role: arithmetic on GF16

## Rules (ABSOLUTE)

- R1: Do NOT import from BR-OUTPUT
- R9: No sibling imports
- May import GF-00 only
- L6: Pure Rust only

## You MAY

- ✅ Add arithmetic ops on GF16
- ✅ Migrate FFI-backed ops from legacy `src/ffi.rs`
- ✅ Add tests

## You MAY NOT

- ❌ Import BR-OUTPUT
- ❌ Add I/O, network, async
- ❌ Break the anchor `phi^2 + phi^-2 = 3`

## Build

```bash
cargo check -p trios-golden-float-gf01
cargo test -p trios-golden-float-gf01
```
