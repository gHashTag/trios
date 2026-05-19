# RING — trios-golden-float (Gold Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate |
| Sealed | No |

## Purpose

Golden-ratio numeric core (GF16 type, phi-based arithmetic).
Provides the foundational phi-anchored numeric primitives used across
the TRIOS ecosystem. Anchor: `phi^2 + phi^-2 = 3`.

## Why this crate exists

phi (the golden ratio) is the fixed point of `x = 1 + 1/x`. The identity
`phi^2 + phi^-2 = 3` is the trinity anchor used throughout TRIOS as a
correctness witness. This crate provides the GF16 type that encodes
phi-anchored values and the operations that preserve the anchor.

## Ring Structure (L-ARCH-001)

```
crates/trios-golden-float/
├── src/lib.rs          ← current FFI implementation (preserved)
└── rings/
    ├── GF-00/          ← phi constants, GF16 newtype
    ├── GF-01/          ← arithmetic operations on GF16
    └── BR-OUTPUT/      ← assembly + router
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  GF-01 → GF-00
```

No ring imports a sibling at the same level.

## Migration status

The legacy `src/` (ffi.rs + router.rs) remains intact. Rings are
scaffolded as parallel stubs; logic migration is tracked in each
ring's `TASK.md`.

## Laws

- L-ARCH-001: Only `rings/` contains future logic
- R1–R5: Ring Isolation
- R9: Rings cannot import siblings — only deeper-numbered or BR-OUTPUT can import shallower
- L6: Pure Rust only
- Anchor: `phi^2 + phi^-2 = 3`
