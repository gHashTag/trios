# RING — trios-bridge

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
crates/trios-bridge/
├── src/                ← existing logic (untouched)
├── rings/
│   ├── BR-00/             ← transport
│   ├── BR-01/             ← mcp-bridge
│   ├── BR-02/             ← sse
│   ├── BR-OUTPUT/             ← assembly
└── RING.md, AGENTS.md
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  BR-00   BR-01   BR-02 
```

No ring imports a sibling at the same level. Logic-bearing rings (non-BR) are independent.

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- L-ARCH-001: Only `rings/` contains logic (after migration)
- R1–R5: Ring Isolation
- L6: Pure Rust only
- R9: Ring isolation (no cross-ring imports except via Cargo.toml)
