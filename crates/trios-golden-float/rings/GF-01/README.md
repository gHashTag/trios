# GF-01 — arithmetic operations on GF16

Phi-anchored arithmetic on top of GF-00.

## API

| Item | Role |
|------|------|
| `add_bits(a, b)` | Bit-level addition (placeholder) |
| `scale_phi(x)` | Multiply by PHI (host f64 arithmetic) |

## Dependencies

- `trios-golden-float-gf00` (GF-00)

## Migration note

FFI-backed arithmetic from legacy `crates/trios-golden-float/src/` will
migrate here. See `TASK.md`.
