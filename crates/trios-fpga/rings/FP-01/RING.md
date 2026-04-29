# RING — FP-01 (trios-fpga)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-fpga-fp01 |
| Sealed | No |

## Purpose

`synthesis` ring for `trios-fpga`. Scaffolded as part of issue #238 to bring
this crate under the ring-isolation architecture (L-ARCH-001).

## Ring scope

This ring will eventually own the `synthesis` concern of `trios-fpga`.
The current scaffold is a placeholder; logic remains in the parent crate's
`src/` until migrated incrementally.

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change
- L6: Pure Rust
