# RING — trios-store (Silver Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Silver |
| Type | Crate (workspace member) |
| Role | Unified persistence for the consolidated Rust backend |
| Sealed | No |

## Purpose

Single source of persistence for the whole trios backend. Mirrors the
drizzle SQLite schema (`agent_definitions`, `oauth_tokens`,
`produced_files`) 1:1 so Rust reads/writes the SAME database file as the
legacy TS backend during migration.

## Ring Structure (L-ARCH-001)

```
crates/trios-store/
├── src/lib.rs          ← re-export facade (NOT business logic)
└── rings/
    ├── ST-00/          ← row types + enums (Adapter, DetectedBy) — pure data
    ├── ST-01/          ← SeaORM SQLite repository (Store)
    ├── ST-02/          ← DDL migrations (mirror drizzle)
    └── BR-OUTPUT/      ← open_and_migrate (assembles all rings)
```

## Dependency Flow

```
BR-OUTPUT → ST-02 → ST-01 → ST-00
```
No ring imports a sibling at the same level.

## Laws

- L-ARCH-001: Only `rings/` contains logic
- R1–R5: Ring Isolation
- L6: Pure Rust only
- Schema parity: DDL in ST-02 must stay 1:1 with the drizzle schema until TS is retired.
