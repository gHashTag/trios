# AGENTS.md — SILVER-RING-DR-01

> AAIF-compliant (Linux Foundation Agentic AI Foundation)
> MCP-compatible agent instructions

## Identity

- Ring: SILVER-RING-DR-01
- Metal: Silver
- Crate: trios-doctor
- Package: trios-doctor-dr01

## What this ring does

Runs `cargo check`, `cargo clippy`, `cargo test` on the workspace and returns structured results.

## Rules (ABSOLUTE)

- Read `LAWS.md` before ANY action
- R1: Do NOT import from DR-02 or DR-03 directly
- R2: Separate package — do not merge
- L6: Pure Rust only
- All subprocesses via `std::process::Command` only
- No async runtime — synchronous blocking calls

## You MAY

- ✅ Add new check methods to `Doctor` (e.g., `check_fmt`, `check_audit`)
- ✅ Improve error parsing in `extract_failed_crates`
- ✅ Add tests

## You MAY NOT

- ❌ Add I/O other than subprocess calls
- ❌ Import from DR-02 or DR-03
- ❌ Add async/tokio runtime
- ❌ Modify SILVER-RING-DR-00 types from here

## Entry point

- Primary file: `src/lib.rs`
- Build: `cargo build -p trios-doctor-dr01`
- Test: `cargo test -p trios-doctor-dr01`
