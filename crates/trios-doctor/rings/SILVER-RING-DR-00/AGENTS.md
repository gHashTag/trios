# AGENTS.md — SILVER-RING-DR-00

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Ring: SILVER-RING-DR-00
- Metal: Silver
- Crate: trios-doctor
- Package: trios-doctor-dr00

## What this ring does

Defines core types: `WorkspaceDiagnosis`, `WorkspaceCheck`, `CheckStatus`. No logic, no I/O.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R1: Do NOT import from sibling rings directly — only via Cargo.toml
- R2: This is a separate package — do not merge with other rings
- R4: Silver files stay in Silver ring — never move to BRONZE-RING-DR
- L6: Pure Rust only — no TypeScript, no Python
- **No business logic here** — types only

## You MAY

- ✅ Add new fields to existing structs (non-breaking)
- ✅ Add new variants to enums
- ✅ Add `impl` blocks for derived traits
- ✅ Write tests in `src/lib.rs`

## You MAY NOT

- ❌ Add I/O, filesystem, or process calls
- ❌ Import from SILVER-RING-DR-01, DR-02, DR-03
- ❌ Create `src/` at crate root level
- ❌ Move files outside `rings/`

## Entry point

- Primary file: `src/lib.rs`
- Build: `cargo build -p trios-doctor-dr00`
- Test: `cargo test -p trios-doctor-dr00`
