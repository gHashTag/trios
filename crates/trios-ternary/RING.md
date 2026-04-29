# RING — trios-ternary (Gold Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate |
| Sealed | No |

## Purpose

Balanced ternary numeric types (-1, 0, +1) and operations.
Foundational for trinity-aligned computation across TRIOS.

## Ring Structure (L-ARCH-001)

```
crates/trios-ternary/
├── src/lib.rs          ← preserved (legacy)
└── rings/
    ├── TR-00/          ← Trit, Tryte, balanced-ternary types
    ├── TR-01/          ← arithmetic ops (add, mul, neg)
    └── BR-OUTPUT/      ← assembly + router
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  TR-01 → TR-00
```

R9: rings cannot import siblings at the same level.

## Laws

- L-ARCH-001: Future logic lives in `rings/`
- R1–R5: Ring Isolation
- R9: No sibling imports
- L6: Pure Rust only
- Anchor: `phi^2 + phi^-2 = 3`
