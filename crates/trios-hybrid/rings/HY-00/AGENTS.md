# AGENTS.md — HY-00 (trios-hybrid)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: HY-00
- Package: trios-hybrid-hy00
- Role: runtime (scaffold — logic migration: TODO)

## What this ring does (target)

Owns runtime logic for trios-hybrid after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (runtime)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-hybrid/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-hybrid-hy00
cargo test  -p trios-hybrid-hy00
```
