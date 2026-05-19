# RING — trios-ca-mask

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

Cellular automaton mask construction and application rings for trios-ca-mask — scaffolded under L-ARCH-001.

## Ring Structure (L-ARCH-001)

Rings: CM-00 (mask), CM-01 (apply), BR-OUTPUT (output)

```
crates/trios-ca-mask/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── CM-00/  ← mask
    ├── CM-01/  ← apply
    ├── BR-OUTPUT/  ← output
```

## Dependency flow

```
BR-OUTPUT → CM-01 → CM-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
