# AGENTS.md — BR-OUTPUT (trios-sdk)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: BR-OUTPUT
- Package: trios-sdk-broutput
- Role: assembly (scaffold — logic migration: TODO)

## What this ring does (target)

Owns assembly logic for trios-sdk after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (assembly)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-sdk/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-sdk-broutput
cargo test  -p trios-sdk-broutput
```
