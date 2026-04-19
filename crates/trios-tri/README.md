# trios-tri (∓)

Ternary BitLinear + QAT Engine (Φ3)

Implements ternary {-1, 0, +1} quantization for bulk compute layers with zero-DSP architecture.

## Symbol

∓ — Three-state weights: {-1, 0, +1}

## Phase

Φ3 — Ternary Precision Layer

## Overview

`trios-tri` provides ternary quantization for neural network weights, achieving 20.25× compression vs f32 while using zero DSP resources on FPGA. This makes it ideal for bulk compute layers (FFN gate/up/down) in the TRIOS hybrid precision pipeline.

## Key Features

- **Zero DSP cost** — Uses only LUT (52 per MAC-16 unit vs 71 for GF16)
- **20.25× compression** vs f32 (1.58 bits/parameter)
- **10.13× compression** vs GF16 (16 bits/parameter)
- **QAT foundation** — STE (Straight-Through Estimator) for training-aware quantization
- **Matrix operations** — Full 2D matrix support for FFN layers
- **trios-core integration** — Seamless bridge to SSOT schema

## Installation

Add to `Cargo.toml`:

```toml
[dependencies]
trios-tri = { path = "../crates/trios-tri" }
trios-core = { path = "../crates/trios-core" }
```

## Usage

### Basic Ternary Conversion

```rust
use trios_tri::{Ternary, quantize, dequantize, compute_scale};

let weights = vec![1.0, -0.8, 0.3, 1.5];
let scale = compute_scale(&weights);
let ternary = quantize(&weights, scale);
let recovered = dequantize(&ternary, scale);
```

### Matrix Operations

```rust
use trios_tri::TernaryMatrix;

let data = vec![1.0, -0.5, 0.3, -1.5, 0.8, 0.2];
let matrix = TernaryMatrix::from_f32(&data, 2, 3);
let result = matrix.matmul(&other);
let transposed = matrix.transpose();
```

### Hardware Cost

```rust
use trios_tri::hardware_cost;

let cost = hardware_cost();
assert_eq!(cost.dsp_per_param, 0); // Zero DSP!
assert_eq!(cost.lut_per_param, 52);
```

### Layer-Specific Quantization

```rust
use trios_tri::ffn;

// FFN gate (first linear layer)
let gate_ternary = ffn::quantize_gate(&gate_weights, None);

// FFN up (dimension expansion)
let up_ternary = ffn::quantize_up(&up_weights, None);

// FFN down (projection back)
let down_ternary = ffn::quantize_down(&down_weights, None);

// Memory calculation
let bytes = ffn::ternary_size_bytes(param_count);
```

### QAT (Quantization-Aware Training)

```rust
use trios_tri::qat::{TernarySTE, LearnableScale, QatConfig};

// Create QAT configuration
let mut config = QatConfig::default();

// Forward: scale and ternarize
let ternary = config.forward(0.8);

// Backward: gradient with STE
let grad = config.backward(0.1);

// Update learnable scale
config.update_scale(-0.05, 0.01);
```

### trios-core Integration

```rust
use trios_core::{PrecisionFormat, LayerType};
use trios_tri::{is_ternary_format, supports_ternary, default_precision};

// Check if format is ternary
assert!(is_ternary_format(PrecisionFormat::Ternary158));

// Check if layer supports ternary
assert!(supports_ternary(LayerType::Dense));
assert!(!supports_ternary(LayerType::Embedding));

// Get default precision for a layer
assert_eq!(
    default_precision(LayerType::Dense),
    PrecisionFormat::Ternary158
);
```

## Modules

| Module | Description |
|--------|-------------|
| `arith` | Arithmetic operations (Add, Sub, Mul) and dot product |
| `matrix` | 2D matrix operations for FFN layers |
| `core_compat` | Integration with `trios-core` types |
| `qat` | QAT foundation (STE, learnable scale) |
| `ffn` | Layer-specific quantization (gate, up, down) |

## Hardware Specs (XC7A100T-FGG676)

| Resource | Ternary (∓) | GF16 (φ) | Ratio |
|----------|-------------|----------|-------|
| LUT/MAC-16 | 52 | 71 | 0.73× |
| DSP/MAC-16 | 0 | 16 | 0× |
| FF/MAC-16 | 69 | 266 | 0.26× |
| Cells/MAC-16 | 71 | 549 | 0.13× |

## Compression Ratios

| Format | Bits/Param | vs f32 | vs GF16 |
|--------|------------|--------|---------|
| FP32 | 32 | 1.00× | 0.50× |
| GF16 | 16 | 2.00× | 1.00× |
| Ternary158 | 1.58 | 20.25× | 10.13× |

## Hybrid Pipeline

```
Embedding ──┐
            ├─→ GF16 (φ) ──→ Attention ──┐
Input ──────┘                          ├─→ Output
                                       │
FFN Gate ──→ Ternary (∓) ──→ FFN Up ──┤
            ↑ Zero DSP               ├─→ FFN Down ──→┘
            │ 20.25× compression     │
            └─────────────────────────┘
```

## Brand Kit Compliance

- ✅ Name: `trios-tri`
- ✅ Symbol: ∓ (three-state weights)
- ✅ Phase: Φ3
- ✅ Pattern: `trios-<domain>`
- ✅ Follows `trios-gf` structure

## License

MIT

## Links

- [TRIOS Repository](https://github.com/gHashTag/trios)
- [Brand Kit](./BRAND_KIT.md)
- [trios-core](../trios-core)
- [trios-gf](../trios-golden-float)
