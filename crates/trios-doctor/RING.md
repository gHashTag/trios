# RING — trios-doctor (Gold Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate (workspace member) |
| Sealed | No |

## Purpose

Leukocyte agent: autonomous diagnostics, healing, and quick-win repair
for the trios workspace. Runs `cargo check / clippy / test`, reports results,
and optionally auto-fixes Yellow-level issues.

## Ring Structure (L-ARCH-001)

```
crates/trios-doctor/
└── rings/
    ├── SILVER-RING-DR-00/   ← types: WorkspaceDiagnosis, CheckStatus
    ├── SILVER-RING-DR-01/   ← check runner: cargo check/clippy/test
    ├── SILVER-RING-DR-02/   ← heal: auto-fix logic
    ├── SILVER-RING-DR-03/   ← report: output formatting
    └── BRONZE-RING-DR/      ← binary output: CLI entry points
```

**NO `src/` at crate root. NO files outside `rings/`.**

## Laws

- L-ARCH-001: Only `rings/` inside crate
- R1–R5: Ring Isolation
- L6: Pure Rust only
