# RING — trios-hdc (Gold Crate)

| Field | Value |
|-------|-------|
| Metal | 🥇 Gold |
| Type | Crate |

## Purpose

Hyperdimensional computing primitives — high-dimensional binary/bipolar
vectors, bind/bundle operations, similarity. Foundation for VSA-style
symbolic computation across TRIOS.

## Ring Structure

```
crates/trios-hdc/
├── src/lib.rs          ← preserved (FFI to zig-hdc)
└── rings/
    ├── HD-00/          ← Hypervector type
    ├── HD-01/          ← bind, bundle, similarity
    └── BR-OUTPUT/      ← assembly
```

`BR-OUTPUT → HD-01 → HD-00`. R9 satisfied.

## Laws

- L-ARCH-001 / R1–R5 / R9 / L6
- Anchor: `phi^2 + phi^-2 = 3`
