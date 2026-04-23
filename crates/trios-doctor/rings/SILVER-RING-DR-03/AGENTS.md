# AGENTS.md — SILVER-RING-DR-03

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Ring: SILVER-RING-DR-03
- Metal: Silver
- Crate: trios-doctor
- Package: trios-doctor-dr03

## What this ring does

Formats `WorkspaceDiagnosis` into terminal output (text/JSON). No logic, no subprocess calls.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R1: Do NOT import from DR-01 or DR-02 directly
- L6: Pure Rust only
- Output only — no side effects, no file writes

## You MAY

- ✅ Add new output formats (SARIF, GitHub annotations)
- ✅ Add color output via ANSI escape codes (no external crates)
- ✅ Add tests

## You MAY NOT

- ❌ Write to files
- ❌ Import from DR-01 or DR-02
- ❌ Add subprocess calls

## Entry point

- Primary file: `src/lib.rs`
- Build: `cargo build -p trios-doctor-dr03`
- Test: `cargo test -p trios-doctor-dr03`
