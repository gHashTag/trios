# AGENTS.md — DT-01 (trios-data)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: DT-01
- Package: trios-data-dt01
- Role: query (scaffold — logic migration: TODO)

## What this ring does (target)

Owns query logic for trios-data after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (query)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-data/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-data-dt01
cargo test  -p trios-data-dt01
```
