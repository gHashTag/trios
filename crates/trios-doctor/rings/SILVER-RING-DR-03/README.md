# RING — SILVER-RING-DR-03

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Crate | trios-doctor |
| Package | trios-doctor-dr03 |
| Sealed | No |

## Purpose

Report ring. Formats `WorkspaceDiagnosis` into human-readable output:
plain text, JSON, or color terminal output.

## API Surface (pub)

- `Reporter` — main struct
- `Reporter::print_text()` — terminal output with colors
- `Reporter::print_json()` — JSON output
- `Reporter::summary_line()` — single line summary

## Dependencies

- SILVER-RING-DR-00 (types)

## Laws

- R1: No imports from DR-01 or DR-02 directly
- L6: Pure Rust only
- No external formatting crates without approval
