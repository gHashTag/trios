# AGENTS.md — MD-01 (trios-model)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: MD-01
- Package: trios-model-md01
- Role: validation (scaffold — logic migration: TODO)

## What this ring does (target)

Owns validation logic for trios-model after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (validation)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-model/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-model-md01
cargo test  -p trios-model-md01
```
