# AGENTS.md — PR-01 (trios-precision-router)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: PR-01
- Package: trios-precrouter-pr01
- Role: router (scaffold — logic migration: TODO)

## What this ring does (target)

Owns router logic for trios-precision-router after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (router)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-precision-router/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-precrouter-pr01
cargo test  -p trios-precrouter-pr01
```
