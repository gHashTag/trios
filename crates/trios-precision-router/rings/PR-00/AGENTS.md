# AGENTS.md — PR-00 (trios-precision-router)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: PR-00
- Package: trios-precrouter-pr00
- Role: rules (scaffold — logic migration: TODO)

## What this ring does (target)

Owns rules logic for trios-precision-router after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (rules)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-precision-router/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-precrouter-pr00
cargo test  -p trios-precrouter-pr00
```
