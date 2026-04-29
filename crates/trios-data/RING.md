# RING — trios-data

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver (Tier 2) |
| Type | Crate (scaffold — logic migration: TODO) |
| Sealed | No |

## Purpose

Scaffold for ring-isolation architecture (issue #238).
Current logic lives in `src/` — rings are organizational placeholders for future migration.

## Ring Structure (L-ARCH-001)

```
crates/trios-data/
├── src/                ← existing logic (untouched)
├── rings/
│   ├── DT-00/             ← store
│   ├── DT-01/             ← query
│   ├── DT-02/             ← sync
│   ├── BR-OUTPUT/             ← assembly
└── RING.md, AGENTS.md
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  DT-00   DT-01   DT-02 
```

No ring imports a sibling at the same level. Logic-bearing rings (non-BR) are independent.

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- L-ARCH-001: Only `rings/` contains logic (after migration)
- R1–R5: Ring Isolation
- L6: Pure Rust only
- R9: Ring isolation (no cross-ring imports except via Cargo.toml)
