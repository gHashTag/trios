# TASK — GF-01 (trios-golden-float)

## Status: SCAFFOLDED

## Completed

- [x] `add_bits` placeholder
- [x] `scale_phi` host-side multiplication
- [x] Tests for both

## Open (migration)

- [ ] Migrate `gf16_add`, `gf16_mul`, `gf16_sub`, `gf16_div` from FFI in legacy `src/ffi.rs`
- [ ] Add phi-conjugate operations
- [ ] Property test: anchor preservation `phi^2 + phi^-2 = 3` after a chain of ops

## Next ring: BR-OUTPUT
