# trinity_loss

φ-prior-aware ternary contrastive loss (Trinity Loss) for JEPA-T.

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

## Overview

Trinity Loss is a triplet-margin loss tailored for ternary neural networks
inspired by the golden ratio φ. It penalises both poor positive/negative
separation and excess sparsity in ternary representations.

## Formula

```
sim(a, b)     = dot_ternary(a, b) / 64
L_triplet     = max(0, margin + sim(a, n) - sim(a, p))
L_phi_prior   = φ⁻² · (||a||₀ + ||p||₀ + ||n||₀) / 192
L_total       = L_triplet + λ · L_phi_prior
```

| Constant    | Value  | Meaning                               |
|-------------|--------|---------------------------------------|
| φ⁻²         | 0.382  | Golden-ratio inverse square (≈ 1/φ²)  |
| margin      | 0.5    | Triplet loss margin                   |
| λ (lambda)  | 0.1    | φ-prior weight                        |

where `||x||₀` denotes the number of **zero** entries in the ternary vector x,
and the denominator 192 = 3 × 64 normalises across the full triplet.

## Anchor

- φ² + φ⁻² = 3
- DOI: 10.5281/zenodo.19227877

## Public API

```rust
use trinity_loss::{dot_ternary, sim, zero_count, phi_prior_term, trinity_loss,
                   DEFAULT_MARGIN, DEFAULT_LAMBDA};

let a = [1i8; 64];
let p = [1i8; 64];
let n = [-1i8; 64];

// Individual components
let dp = dot_ternary(&a, &p);          // → 64  (i32)
let s  = sim(&a, &p);                  // → 1.0 (f32)
let z  = zero_count(&a);               // → 0   (u32)
let lp = phi_prior_term(&a, &p, &n);   // → 0.0 (f32)

// Full loss (margin=0.5, λ=0.1)
let loss = trinity_loss(&a, &p, &n, DEFAULT_MARGIN, DEFAULT_LAMBDA);
```

## Properties

- **Deterministic**: identical inputs always produce identical outputs.
- **Allocation-free**: no heap allocation; all computation is in-register.
- **No `std::time`**: safe for `#![no_std]`-adjacent usage.
- **R1 CROWN**: only Rust is compiled; `python_ref/` is reference docs only.

## Tests

```bash
cargo test -p trinity_loss
```

Runs:
- 10 deterministic hand-computed triplets (±1e-4 tolerance)
- 50 LFSR-random stability tests (determinism + non-negativity + finiteness)

## Python Reference

`python_ref/trinity_loss_ref.py` provides a NumPy implementation of the same
formula for independent verification. It is **not** part of `cargo build` or
`cargo test`.

```bash
python3 python_ref/trinity_loss_ref.py   # prints PASS/FAIL for all 10 cases
```

## License

Apache-2.0 — see [LICENSE](../../LICENSE).
