# BR-OUTPUT — trios-golden-float assembly

Top of the ring graph for trios-golden-float. Re-exports public API
from GF-00 and GF-01.

## API

| Item | Role |
|------|------|
| `GF16`, `PHI`, `INV_PHI` | re-exports from GF-00 |
| `add_bits`, `scale_phi` | re-exports from GF-01 |
| `GoldenFloat::anchor()` | returns `3.0` (phi anchor witness) |

## Dependencies

- GF-00, GF-01
