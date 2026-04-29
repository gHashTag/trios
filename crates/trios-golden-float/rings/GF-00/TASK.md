# TASK — GF-00 (trios-golden-float)

## Status: SCAFFOLDED

## Completed

- [x] `PHI`, `INV_PHI` constants
- [x] `GF16(pub u16)` newtype with `from_bits`/`to_bits`
- [x] phi-anchor test (`phi^2 + phi^-2 = 3`)
- [x] roundtrip test

## Open (migration from legacy `src/`)

- [ ] Migrate `GF16` from `crates/trios-golden-float/src/lib.rs` (currently behind `cfg(has_zig_lib)`)
- [ ] Add `From<f32>` / `Into<f32>` once GF-01 lands FFI-free arithmetic
- [ ] Add `Display` and `Debug` formatted for phi-form output

## Next ring: GF-01
