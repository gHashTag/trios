# RING — GF-00 (trios-golden-float)

## Identity

| Field | Value |
|-------|-------|
| Metal | 🥈 Silver |
| Package | trios-golden-float-gf00 |
| Sealed | No |

## Purpose

phi constants + GF16 newtype. Bottom of the dependency graph.

## API Surface (pub)

| Item | Role |
|------|------|
| `PHI`, `INV_PHI` | phi constants |
| `GF16(pub u16)` | 16-bit phi-anchored newtype |

## Dependencies

None.

## Laws

- R1: No imports from GF-01, BR-OUTPUT
- R9: No sibling imports
- L6: Pure Rust only
- No I/O, no async
- Anchor: `phi^2 + phi^-2 = 3`
