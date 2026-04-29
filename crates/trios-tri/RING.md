# RING — trios-tri

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

TRI language parser, compiler, codegen and binary output rings — scaffolded under L-ARCH-001.

## Ring Structure (L-ARCH-001)

Rings: TI-00 (parser), TI-01 (compiler), TI-02 (codegen), BR-OUTPUT (output)

```
crates/trios-tri/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── TI-00/  ← parser
    ├── TI-01/  ← compiler
    ├── TI-02/  ← codegen
    ├── BR-OUTPUT/  ← output
```

## Dependency flow

```
BR-OUTPUT → TI-02 → TI-01 → TI-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
