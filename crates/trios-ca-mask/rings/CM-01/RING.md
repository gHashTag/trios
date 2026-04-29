# RING — CM-01 (trios-ca-mask)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-ca-mask-cm01 |
| Sealed | No |

## Purpose

`apply` ring for `trios-ca-mask`. Scaffolded as part of issue #238 to bring
this crate under the ring-isolation architecture (L-ARCH-001).

## Ring scope

This ring will eventually own the `apply` concern of `trios-ca-mask`.
The current scaffold is a placeholder; logic remains in the parent crate's
`src/` until migrated incrementally.

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change
- L6: Pure Rust
