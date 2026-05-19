# RING — trios-sacred (Gold Crate)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate |
| Sealed | No |

## Purpose

Sacred geometry primitives — geometric shapes anchored on phi, and ratio
helpers (golden ratio, sqrt(2), sqrt(3), pi). Foundational for
trinity-aligned spatial reasoning.

## Ring Structure (L-ARCH-001)

```
crates/trios-sacred/
├── src/lib.rs          ← preserved (FFI to zig-sacred-geometry)
└── rings/
    ├── SC-00/          ← geometry primitives (Vec2, Triangle, Circle)
    ├── SC-01/          ← sacred ratios (phi, sqrt2, sqrt3, pi)
    └── BR-OUTPUT/      ← assembly + router
```

## Dependency Flow

```
BR-OUTPUT
    ↓
  SC-01 → SC-00
```

R9: rings cannot import siblings.

## Laws

- L-ARCH-001: Future logic in `rings/`
- R1–R5: Ring Isolation
- R9: No sibling imports
- L6: Pure Rust only
- Anchor: `phi^2 + phi^-2 = 3`
