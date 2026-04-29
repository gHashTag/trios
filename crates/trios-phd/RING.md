# RING — trios-phd

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Type  | Crate (rings scaffolded for issue #238) |
| Sealed | No |

## Purpose

LaTeX pipeline and chapter-management rings, with BR-PDF artifact ring, for trios-phd — scaffolded under L-ARCH-001 (Tier 4).

## Ring Structure (L-ARCH-001)

Rings: PD-00 (latex), PD-01 (chapters), BR-PDF (pdf-artifact)

```
crates/trios-phd/
├── src/                 ← existing logic (untouched, re-export facade)
└── rings/               ← scaffolded for issue #238 (additive)
    ├── PD-00/  ← latex
    ├── PD-01/  ← chapters
    ├── BR-PDF/  ← pdf-artifact
```

## Dependency flow

```
BR-PDF → PD-01 → PD-00
```

## Anchor

`phi^2 + phi^-2 = 3 · TRINITY · NEVER STOP`

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change to parent `src/`
- L-ARCH-001: `rings/` is the future home of all logic
