# jepa_t_ingest

**Wave-14a L-S50** — Plaintext → ternary-quantized triplet streaming pipeline for JEPA-T training on Trinity silicon.

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021--edition-orange.svg)](https://www.rust-lang.org/)

## Overview

`jepa_t_ingest` converts raw UTF-8 text corpora into binary streams of ternary triplets
(`anchor`, `positive`, `negative`) suitable for Joint Embedding Predictive Architecture
(JEPA-T) contrastive pretraining on Trinity ternary silicon.

### Ternary Anchor

- **Alphabet**: {−1, 0, +1}
- **Threshold**: φ⁻² in Q1.15 fixed-point = **12533** (0x30F4)
- **Identity**: φ² + φ⁻² = 3
- **DOI**: [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

### Quantizer — Wave-9b RTL Byte-for-Byte Match

The core `quantize_phi_prior` function matches `phi_prior_quantizer.v` from Wave-9b exactly:

```
if fp_q15 >= +12533  →  +1
if fp_q15 <= −12533  →  −1
else                 →   0
```

## API

### `quantize_phi_prior(fp_q15: i16) -> i8`

Ternary quantizer with Wave-9b RTL parity.

```rust
use jepa_t_ingest::quantize_phi_prior;

assert_eq!(quantize_phi_prior(12533),  1);   // at positive threshold
assert_eq!(quantize_phi_prior(-12533), -1);  // at negative threshold
assert_eq!(quantize_phi_prior(12532),  0);   // below threshold
assert_eq!(quantize_phi_prior(-12532), 0);   // above -threshold
assert_eq!(quantize_phi_prior(0),      0);   // zero
```

### `ingest_text(input: &str, cfg: &IngestConfig) -> Vec<Triplet>`

Streams a plaintext string into a sequence of ternary triplets.

```rust
use jepa_t_ingest::{ingest_text, IngestConfig};

let cfg = IngestConfig { window_size: 64, stride: 32 };
let triplets = ingest_text("your corpus text here ...", &cfg);
println!("{} triplets produced", triplets.len());
```

### `Triplet`

```rust
pub struct Triplet {
    pub anchor:   [i8; 64],   // anchor context window
    pub positive: [i8; 64],   // adjacent / overlapping window
    pub negative: [i8; 64],   // non-overlapping window (hard negative)
}
```

Each element is in {−1, 0, +1}. Serialise to binary with `triplet.to_bytes()` (192 bytes).

### `IngestConfig`

```rust
pub struct IngestConfig {
    pub window_size: usize,   // tokens per window (max 64)
    pub stride:      usize,   // step between anchor windows
}
```

## CLI Binary

```
jepa_t_ingest --input corpus.txt --output triplets.bin [--window-size 64] [--stride 32]
```

### Output Format

Raw binary stream of packed 192-byte triplet records:

| Bytes | Content |
|-------|---------|
| 0–63  | anchor (64 × i8) |
| 64–127 | positive (64 × i8) |
| 128–191 | negative (64 × i8) |

## Tests

```bash
# Run all tests (quantizer boundary + ingest golden integration)
cargo test -p jepa_t_ingest

# Build release binary
cargo build --release --bin jepa_t_ingest
```

### Quantizer Boundary Tests (`tests/quantize.rs`)

| Input | Expected | Notes |
|-------|----------|-------|
| +12532 | 0 | one below threshold |
| +12533 | +1 | at threshold (φ⁻²) |
| −12532 | 0 | one above −threshold |
| −12533 | −1 | at −threshold |
| 0 | 0 | zero |
| +0x7FFF | +1 | i16::MAX |
| −0x8000 | −1 | i16::MIN |

The exhaustive test `output_always_ternary_for_all_i16` checks all 65536 possible i16 inputs.

### Integration Test (`tests/ingest.rs`)

Uses a fixed 13-token golden corpus:

```
"the quick brown fox jumps over the lazy dog and a ternary world"
```

With `window_size=4, stride=2` this produces **3 triplets** (5 windows).
Token hashes and ternary values are byte-compared against pre-computed golden values.

## R1 CROWN Compliance

This crate is **Rust ONLY** — no Python, no shell scripts, no foreign-language source files.
The quantizer is a single `#[inline]` function with no dependencies beyond `core`.

## License

Apache-2.0 — Copyright 2024 Dmitrii Vasilev &lt;admin@t27.ai&gt;
