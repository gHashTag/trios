# AGENTS.md — trios-data

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Crate: trios-data
- Tier: 2 (SILVER)
- Status: Scaffolded (logic migration: TODO)
- Repo: gHashTag/trios

## What this crate does

Existing logic in `src/`. The `rings/` directory contains scaffold packages
for the future ring-isolation migration per issue #238.

## Ring map

| Ring | Package | Role | Sealed |
|------|---------|------|--------|
| DT-00 | trios-data-dt-00 | store | No |
| DT-01 | trios-data-dt-01 | query | No |
| DT-02 | trios-data-dt-02 | sync | No |
| BR-OUTPUT | trios-data-br-output | assembly | No |

## Rules (ABSOLUTE)

- Read repo `LAWS.md` and `CLAUDE.md` before any action
- L-ARCH-001: After migration, logic lives in `rings/` — `src/lib.rs` becomes RE-EXPORT
- R1–R5: Ring Isolation
- R9: No cross-ring imports except via Cargo.toml
- L6: Pure Rust only
- L7: Experience line per merge

## Migration plan

1. Stub rings exist with passing tests (this PR)
2. Future PR: extract types/logic from `src/` into appropriate rings
3. `src/lib.rs` becomes thin re-export facade

## You MAY NOT (until migration complete)

- ❌ Add new logic to `rings/<ring>/src/lib.rs` beyond stub types
- ❌ Cross-import rings without Cargo.toml declaration
