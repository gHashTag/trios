# AGENTS.md — SILVER-RING-DR-02

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Ring: SILVER-RING-DR-02
- Metal: Silver
- Crate: trios-doctor
- Package: trios-doctor-dr02

## What this ring does

Auto-repair logic: takes a `WorkspaceDiagnosis` and attempts to fix issues via `cargo fix`, formatting, missing files.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R1: Do NOT import from DR-01 or DR-03 directly
- L6: Pure Rust only
- **Destructive operations require explicit confirmation flag**
- Never auto-delete files — only create or fix

## You MAY

- ✅ Implement `cargo fix --allow-dirty` for auto-fixable warnings
- ✅ Implement `cargo fmt` auto-format
- ✅ Create missing `RING.md`, `AGENTS.md`, `TASK.md` in rings
- ✅ Add tests

## You MAY NOT

- ❌ Delete any files automatically
- ❌ Import from DR-01 or DR-03
- ❌ Run destructive commands without `dry_run: false` flag

## Entry point

- Primary file: `src/lib.rs`
- Build: `cargo build -p trios-doctor-dr02`
- Test: `cargo test -p trios-doctor-dr02`
