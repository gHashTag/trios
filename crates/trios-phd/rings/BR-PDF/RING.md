# RING — BR-PDF (trios-phd)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-phd-brpdf |
| Sealed | No |

## Purpose

`pdf-artifact` ring for `trios-phd`. Scaffolded as part of issue #238 to bring
this crate under the ring-isolation architecture (L-ARCH-001).

## Ring scope

This ring will eventually own the `pdf-artifact` concern of `trios-phd`.
The current scaffold is a placeholder; logic remains in the parent crate's
`src/` until migrated incrementally.

## Laws

- R1 / R5 / R9: Ring isolation
- L7: Additive scaffold only — no behavior change
- L6: Pure Rust
