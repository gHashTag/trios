# AGENTS.md — SD-01 (trios-sdk)

> AAIF-compliant | MCP-compatible

## Identity

- Ring: SD-01
- Package: trios-sdk-sd01
- Role: client (scaffold — logic migration: TODO)

## What this ring does (target)

Owns client logic for trios-sdk after migration.
Currently a stub with a marker type and metadata constants.

## Rules (ABSOLUTE)

- R1: Sibling rings may not be imported without Cargo.toml declaration
- R9: Ring isolation
- L6: Pure Rust only

## You MAY

- ✅ Add types/methods within this ring's scope (client)
- ✅ Add unit tests
- ✅ Migrate code from `crates/trios-sdk/src/` matching this ring's scope

## You MAY NOT

- ❌ Import sibling rings without Cargo.toml dep
- ❌ Add I/O or async without explicit approval (check parent crate's policy)

## Build

```bash
cargo check -p trios-sdk-sd01
cargo test  -p trios-sdk-sd01
```
