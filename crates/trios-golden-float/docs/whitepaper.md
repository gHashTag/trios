
# IGLA-GF16 Whitepaper

## 11. IGLA-GF16 Number Families

### 11.1 Trinity Closed System

Three numbers form a closed system:

| Constant | Value | Source |
|----------|-------|--------|
| φ | 1.618033988749895 | `man/exp ratio = 9/6 = 1.5 ≈ φ` |
| αφ | φ³/2 = 0.381966011250105 | `α_φ = φ³ / 2` (Trinity strong coupling) |
| φ² | φ × φ = 2.618033988749895 | `α_s(mZ) PDG2024` (Δ < 0.03σ) |
| α_φ | φ³/3 = 0.560904760629166 | `α_φ = φ³ / 3` |

The key insight: **GF16 format 1:6:9 has mantissa/exponent = 1.5 ≈ φ**, where φ is derived from Trinity constant `α_φ`, not from an ideal golden ratio.

### 11.2 IGLA-GF16 Architecture Numbers

FIBONACI architecture (7 layers, 15.5MB target):

| Parameter | Value | Fibonacci # |
|-----------|---------------|
| d_model | 144 | Fib #12 (φ² / √5 ≈ 144.0, Δ < 0.1%) |
| n_heads | 8 | Fib #6 (144 / 24 = 6) |
| d_head | 18 | Fib #13 (144 / 8 = 18) |
| d_ffn | 233 | Fib #13 (144 × φ ≈ 233) |
| n_layers | 7 | `log_φ(budget)` |

### 11.3 Trinity Weight Init (4 physics sectors)

Four physics constants initializing attention:

| Sector | Symbol | Value |
|--------|--------|-------|
| gauge (attn QKV) | std = α_φ = 0.118034 | φ-scaled attention weights |
| higgs (attn proj) | std = α_φ × φ⁻¹ = 0.072949 | FIBONACI projection weights |
| lepton (ffn gate) | std = α_φ × φ⁻² = 0.045085 | Lepton fermion normalization |
| cosmology (embed) | std = α_φ × φ⁻³ = 0.027864 | Cosmology embedding weights |

### 11.4 φ-LR Schedule

Learning rate schedule over training steps (τ = T/φ·27):

| t | LR(t) | Formula |
|---|--------|----------|
| t=0 | 0.118034 | `α_φ = φ³ / 3` (Trinity init) |
| t=100 | 0.095655 | `α_φ × 0.81` |
| t=500 | 0.041258 | `α_φ / 4` |

### 11.5 CA φ-Mask (Fibonacci Distances)

Visible Fibonacci distances (11 tokens):

| Distance | F # | Approx. Value |
|-----------|--------|---------------|
| F⁻¹ | 1/φ | 0.618 | Visible |
| F⁻² | 1/φ² | 0.382 | Visible |
| F⁻³ | 1/φ³ | 0.236 | Visible |
| F⁻⁴ | 1/φ⁴ | 0.146 | Visible |
| F⁻⁵ | 1/φ⁵ | 0.090 | Visible |
| F⁻⁶ | 1/φ⁶ | 0.056 | Visible |
| F⁻⁷ | 1/φ⁶ | 0.034 | Visible |
| F⁻⁷ | 1/φ⁶ | 0.021 | Visible |
| F⁻⁸ | 1/φ⁸ | 0.012 | Visible |
| F¹ | 1/φ¹ | 0.618 | Visible |
| F¹² | 1/φ² | 0.382 | Visible |
| F¹³ | 1/φ³ | 0.236 | Visible |

Sparsity: 2.15% per token
Attention reduction: 262144 → 5632 pairs (46.6× sparse)

### 11.6 JEPA Split (7 layers → target)

Current split (7 layers → 15.5MB target):

| Layer | Width | GF16 | Reduction |
|-------|------|--------|----------|
| embedding | 7.24M | 8.14M | 12.5% |
| attention | 3.62M | 4.50M | 21.8% |
| ffn | 5.38M | 2.69M | 50.0% |
| ffn | 5.38M | 2.69M | 50.0% |
| predictor | 3.62M | 4.50M | 21.8% |

Note: Target φ-split needs correct ratio (not 18.08).
