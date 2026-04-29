# RING — trios-cli

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

TRIOS CLI command parsing, REPL, output formatting and binary entry rings — scaffolded under L-ARCH-001.

## Ring Structure (L-ARCH-001)

Rings: CI-00 (commands), CI-01 (repl), CI-02 (output), BR-BIN (binary)

```
crates/trios-cli/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── CI-00/  ← commands
    ├── CI-01/  ← repl
    ├── CI-02/  ← output
    ├── BR-BIN/  ← binary
```

## Dependency flow

```
BR-BIN → CI-02 → CI-01 → CI-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
