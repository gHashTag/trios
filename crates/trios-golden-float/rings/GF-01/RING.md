# RING — GF-01 (trios-golden-float)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-golden-float-gf01 |
| Sealed | No |

## Purpose

Arithmetic operations on GF16. Phi-anchored.

## API Surface (pub)

| Item | Role |
|------|------|
| `add_bits` | Bit-level addition |
| `scale_phi` | Phi multiplication |

## Dependencies

- GF-00

## Laws

- R1: No imports from BR-OUTPUT
- R9: No sibling imports
- May import shallower rings (GF-00) only
- L6: Pure Rust only
- Anchor: `phi^2 + phi^-2 = 3`
