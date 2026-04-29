# RING — SILVER-RING-DR-01

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Crate | trios-doctor |
| Package | trios-doctor-dr01 |
| Sealed | No |

## Purpose

Check runner. Executes `cargo check`, `cargo clippy`, `cargo test` against
the workspace and returns structured `WorkspaceCheck` results.

## API Surface (pub)

- `Doctor` — main struct, constructed with workspace root path
- `Doctor::run_all()` — runs all checks, returns `WorkspaceDiagnosis`
- `Doctor::count_crates()` — returns number of crates in workspace

## Dependencies

- SILVER-RING-DR-00 (types)

## Provides to

- BRONZE-RING-DR (binary entry point)

## Laws

- R1: No imports from DR-02, DR-03 directly
- L6: Pure Rust only
- All subprocess calls use `std::process::Command`
