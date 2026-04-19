# trios-physics

Rust FFI bindings for [zig-physics](https://github.com/gHashTag/zig-physics) — quantum mechanics, QCD, and gravity simulations.

## Setup

```bash
cd crates/trios-physics
git submodule update --init vendor/zig-physics
```

## Usage

```rust
use trios_physics::{chsh_bell, gf_constants};

let result = chsh_bell(0.0, PI/4.0, PI/8.0, 3.0*PI/8.0);
println!("CHSH S = {}, violated: {}", result.s_value, result.violated);

let c = gf_constants();
println!("φ = {}", c.phi);
```
