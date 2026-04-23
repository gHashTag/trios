# AGENTS.md — BRONZE-RING-DR

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Ring: BRONZE-RING-DR
- Metal: Bronze
- Crate: trios-doctor
- Package: trios-doctor-bronze

## What this ring does

CLI entry point. Calls Silver rings in order: check → heal → report. No business logic here.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R4: Bronze files stay in Bronze — never move Rust logic to Silver rings from here
- No business logic in `main.rs` — only orchestration
- All logic must live in Silver rings (DR-01, DR-02, DR-03)

## You MAY

- ✅ Add CLI flags (e.g., `--json`, `--heal`, `--dry-run`)
- ✅ Add new binary targets in Cargo.toml
- ✅ Improve error handling in main()

## You MAY NOT

- ❌ Move business logic into main.rs
- ❌ Bypass Silver rings
- ❌ Add Silver-level logic (check parsing, heal algorithms) here

## Entry point

- Primary file: `src/main.rs`
- Build: `cargo build -p trios-doctor-bronze`
- Run: `cargo run -p trios-doctor-bronze --bin trios-doctor`
