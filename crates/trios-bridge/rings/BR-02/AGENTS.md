# AGENTS.md — BR-02 (trios-bridge)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: BR-02
- Package: trios-bridge-br02
- Role: sse (scaffold — logic migration: TODO)

## What this ring does (target)

Owns sse logic for trios-bridge after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (sse)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-bridge/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-bridge-br02
cargo test  -p trios-bridge-br02
```
