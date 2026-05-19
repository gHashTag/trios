# RING — trios-git

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

Git operations, hooks and binary output rings for trios-git — scaffolded under L-ARCH-001.

## Ring Structure (L-ARCH-001)

Rings: GT-00 (ops), GT-01 (hooks), BR-OUTPUT (output)

```
crates/trios-git/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── GT-00/  ← ops
    ├── GT-01/  ← hooks
    ├── BR-OUTPUT/  ← output
```

## Dependency flow

```
BR-OUTPUT → GT-01 → GT-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
