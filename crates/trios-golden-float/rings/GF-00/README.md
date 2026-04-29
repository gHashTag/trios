# GF-00 — phi constants and GF16 type

Bottom of the dependency graph for trios-golden-float.

## API

| Item | Role |
|------|------|
| `PHI: f64` | Golden ratio (1 + sqrt(5)) / 2 |
| `INV_PHI: f64` | 1/phi |
| `GF16(pub u16)` | 16-bit phi-anchored numeric newtype |

## Invariant

`phi^2 + phi^-2 = 3` — verified in `tests`.

## Example

```rust
use trios_golden_float_gf00::{PHI, GF16};

let g = GF16::from_bits(0x4248);
assert_eq!(g.to_bits(), 0x4248);
```

## Dependencies

None. GF-00 is the root of the ring graph.
