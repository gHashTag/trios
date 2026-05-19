# RING — SILVER-RING-DR-02

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Crate | trios-doctor |
| Package | trios-doctor-dr02 |
| Sealed | No |

## Purpose

Heal ring. Takes a `WorkspaceDiagnosis` from DR-01 and attempts
auto-repair: `cargo fix`, format corrections, missing file creation.

## API Surface (pub)

- `Healer` — main struct
- `Healer::heal()` — attempt auto-fix based on diagnosis
- `HealResult` — result of heal attempt

## Dependencies

- SILVER-RING-DR-00 (types)

## Laws

- R1: No imports from DR-01 or DR-03 directly
- L6: Pure Rust only
