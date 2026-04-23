# RING — SILVER-RING-DR-00

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Crate | trios-doctor |
| Package | trios-doctor-dr00 |
| Sealed | No |

## Purpose

Core types ring. Defines all shared data structures used across trios-doctor rings.
No business logic. No I/O. Pure data definitions.

## API Surface (pub)

- `WorkspaceDiagnosis` — top-level diagnostic report
- `WorkspaceCheck` — single check result
- `CheckStatus` — enum: Green / Yellow / Red

## Dependencies

- None (bottom of dependency graph)

## Provides to

- SILVER-RING-DR-01 (check runner)
- SILVER-RING-DR-02 (heal)
- SILVER-RING-DR-03 (report)
- BRONZE-RING-DR (binary)

## Laws

- R1: No imports from sibling rings
- R2: Separate package in workspace
- R3: This RING.md is required (you are reading it)
- L6: Pure Rust only
