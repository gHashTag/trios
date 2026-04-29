# RING — VM-01 (trios-vm)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥉 Bronze |
| Package | trios-vm-vm01 |
| Sealed | No |

## Purpose

`executor` ring for `trios-vm`. Scaffolded as part of issue #238
to bring this crate under the ring-isolation architecture (L-ARCH-001).

## Ring scope

This ring will eventually own the `executor` concern of `trios-vm`.
The current scaffold is a placeholder; logic remains in the parent crate's
`src/` until migrated incrementally.

## Laws

- R1 / R5 / R9: Ring isolation, no sibling imports, parent re-exports only
- L7: Additive scaffold only — no behavior change
- L6: Pure Rust
