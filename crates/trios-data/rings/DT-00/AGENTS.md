# AGENTS.md — DT-00 (trios-data)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: DT-00
- Package: trios-data-dt00
- Role: store (scaffold — logic migration: TODO)

## What this ring does (target)

Owns store logic for trios-data after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (store)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-data/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-data-dt00
cargo test  -p trios-data-dt00
```
