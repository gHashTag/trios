# RING — trios-igla-race

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

Orchestration and telemetry rings for trios-igla-race (active IGLA #143/#264 orchestrator) — scaffolded under L-ARCH-001 (Tier 4).

## Ring Structure (L-ARCH-001)

Rings: IR-00 (orchestration), IR-01 (telemetry), BR-OUTPUT (output)

```
crates/trios-igla-race/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── IR-00/  ← orchestration
    ├── IR-01/  ← telemetry
    ├── BR-OUTPUT/  ← output
```

## Dependency flow

```
BR-OUTPUT → IR-01 → IR-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
