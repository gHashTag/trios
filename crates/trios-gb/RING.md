# RING — trios-gb

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

Branch and diff handling for trios-gb (Git-Bee) — scaffolded under L-ARCH-001.

## Ring Structure (L-ARCH-001)

Rings: GB-00 (branches), GB-01 (diff), BR-OUTPUT (output)

```
crates/trios-gb/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── GB-00/  ← branches
    ├── GB-01/  ← diff
    ├── BR-OUTPUT/  ← output
```

## Dependency flow

```
BR-OUTPUT → GB-01 → GB-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
