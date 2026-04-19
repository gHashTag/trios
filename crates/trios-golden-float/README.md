# trios-golden-float

Rust FFI bindings for [zig-golden-float](https://github.com/gHashTag/zig-golden-float) — GF16 (Golden Float 16-bit) numeric core.

## Setup

Initialize the Zig submodule:

```bash
cd crates/trios-golden-float
git submodule update --init vendor/zig-golden-float
```

## Usage

```rust
use trios_golden_float::{GF16, compress_weights};

let g = GF16::from_f32(1.618);
let f = g.to_f32();

let weights = vec![0.5, 1.0, -0.3];
let compressed = compress_weights(&weights);
```
