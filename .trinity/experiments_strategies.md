# Open Golf: CPU N-Gram Language Model Training — Complete Strategy Guide
## Issue: #237 — Target: < 1.5 BPB on TinyShakespeare

**Current Best: 2.5329 BPB** — 6-gram h=384 lr=0.004 wd=0.01 seed=43
**Gap to Target: 1.0329 BPB (≈41% improvement needed)**

---

## Dataset & Baselines

| Metric | Value |
|--------|-------|
| Dataset | TinyShakespeare (1.1MB, 1,115,394 characters) |
| Split | 90% train / 10% val |
| Vocab | 128 (byte-level tokens) |
| Dim | 64 (embedding dimension) |
| Seq | 64 (sequence length) |
| Default Optimizer | AdamW (β1=0.618, β2=0.999, ε=1e-8, wd=0.04) |
| LR Schedule | Cosine with warmup |

---

## Complete History of BPB Records (Chronological)

| Date | Model | Config | BPB | Δ | Notes |
|------|-------|--------|-----|---|-------|
| Apr 21 | Bigram | baseline | 3.90 | — | Starting point |
| Apr 21 | Trigram + AdamW | — | 3.26 | -0.64 | +context, +AdamW |
| Apr 22 | 4-gram + ReLU h=64 | h=64 | 2.958 | -0.30 | +4-gram, +ReLU hidden |
| Apr 22 | 4-gram + ReLU h=128 | h=128 | 2.877 | -0.08 | ×2 hidden dim |
| Apr 22 | 4-gram fixed backward | h=128, lr=0.004 | 2.780 | -0.10 | backward bugfix (3-seed avg) |
| Apr 23 | GELU h=192 | h=192, gelu | 2.6942 | -0.09 | smoother activation |
| Apr 23 | GELU h=256 | h=256, gelu | 2.7265 | +0.03 | diminishing returns |
| Apr 23 | 4-gram h=192 | h=192, relu, wd=0.01 | 2.7184 | -0.06 | weight decay tuning |
| Apr 23 | 4-gram h=256 | h=256, relu, wd=0.01 | 2.6964 | -0.02 | hidden size increase |
| Apr 23 | 5-gram (ctx3) | ctx3, h=256, lr=0.006, wd=0.01 | 2.6005 | -0.10 | +5th context token |
| Apr 23 | 5-gram h=384 | ctx3, h=384, lr=0.006, wd=0.01 | 2.5771 | -0.02 | more capacity |
| Apr 23 | 5-gram h=384 (3-seed) | ctx3, h=384, lr=0.006, wd=0.01 | 2.5719±0.004 | — | stable across seeds |
| Apr 23 | 6-gram (ctx4) | ctx4, h=384 | 2.5678 | -0.00 | +6th context token |
| Apr 23 | 6-gram + label smoothing | ctx4, h=384, ls=0.1 | 2.5654 | -0.00 | minimal impact |
| Apr 24 | 6-gram lr=0.004 | ctx4, h=384, lr=0.004, wd=0.01 | **2.5329** | -0.03 | **CURRENT RECORD** |
| Apr 24 | 6-gram lr=0.005 | ctx4, h=384, lr=0.005, wd=0.01 | 2.5500 | +0.02 | higher LR worse |
| Apr 24 | 6-gram (3-seed avg) | ctx4, h=384, lr=0.004, wd=0.01 | 2.5431±0.01 | — | verified stability |

---

## Strategy Matrix: All Experiments Attempted

### 1. Model Architecture Strategies

| Strategy | Variants Tested | Best Result | Status |
|----------|----------------|-------------|--------|
| **N-gram Context** | Bigram → Trigram → 4-gram → 5-gram → 6-gram | 2.5329 (6-gram) | ✅ Diminishing returns after 5-gram |
| **Hidden Size** | 64 → 128 → 192 → 256 → 384 | 2.5329 (h=384) | ✅ Larger helps, but costs |
| **Activation** | ReLU, GELU | 2.6942 (GELU h=192) | ❌ GELU slightly better, but not decisive |
| **Residual Connections** | Yes/No flag | 2.743 (with res) | ❌ No improvement observed |
| **LayerNorm** | Input only, Input+Output | 2.6964 (ln) | ❌ Minimal impact |
| **Multi-layer FFN** | 1, 2, 3 layers | 3.21 (2 layers) | ❌ Made things worse |

### 2. Training Hyperparameters

| Strategy | Range Tested | Best Result | Status |
|----------|--------------|-------------|--------|
| **Learning Rate** | 0.001 → 0.008 | 0.004 (optimal) | ✅ 0.004 is sweet spot |
| **Weight Decay** | 0.0, 0.01, 0.04 | 0.01 (best) | ✅ 0.01 > 0.04 for this task |
| **Warmup** | 0, 100, 500 | 0 (no warmup) | ❌ Warmup not beneficial |
| **Dropout** | 0.0, 0.1, 0.2 | 0.0 (no dropout) | ❌ Dropout hurts performance |
| **Steps** | 2000, 5000, 10000, 12000 | 12000 | ✅ More steps = better |

### 3. Data & Optimization Strategies

| Strategy | Implementation | Best Result | Status |
|----------|----------------|-------------|--------|
| **Cosine LR Schedule** | Implemented | Standard | ✅ Baseline |
| **Label Smoothing** | 0.05, 0.1 | 2.5654 | ❌ Minimal benefit |
| **Gradient Clipping** | Implemented | Standard | ✅ Baseline |
| **AdamW Optimizer** | φ-optimized β1=0.618 | Standard | ✅ Baseline |
| **Parallel Seeds** | 3 seeds (42,43,44) | 2.5431±0.01 | ✅ Validation |

### 4. Experimental / Failed Strategies

| Strategy | Why It Failed |
|----------|---------------|
| GF16 (GoldenFloat16) | Numerical instability, worse BPB (3.2182) |
| Multi-layer FFN | Increased depth hurt performance (3.21) |
| Residual connections | No improvement (2.743) |
| Dropout | Regularization hurts on small dataset |
| Warmup | Not needed for cosine schedule |
| GELU > ReLU | GELU slightly better but not decisive |

---

## SOTA Benchmarks (External Reference)

| Model | Architecture | BPB | Platform | Notes |
|-------|-------------|-----|----------|-------|
| **Baseline (ALiBi + LoRA)** | Transformer | 2.1536 | T4 GPU | LinkedIn benchmark |
| **HierAttn v3 (residual)** | Hierarchical Attention | **1.2150** | T4 GPU | **43% improvement** |
| **Our Current Best** | 6-gram N-gram | **2.5329** | CPU | Gap: 1.32 BPB |

**Key Insight:** Hierarchical Attention achieves 1.2150 BPB, well below our 1.5 target. This suggests:
- Architecture matters more than hyperparameters
- Hierarchical/self-attention patterns are superior to n-gram
- We may need to explore transformer-based approaches

---

## Recommended Next Steps (Priority Order)

### 🔴 HIGH PRIORITY: Architectural Changes

| # | Strategy | Expected Impact | Effort |
|---|----------|----------------|--------|
| 1 | **Implement Attention Mechanism** | -0.5 to -1.0 BPB | High |
| 2 | **Multi-Head Self-Attention (MHSA)** | -0.3 to -0.8 BPB | High |
| 3 | **Positional Embeddings** | -0.1 to -0.3 BPB | Medium |
| 4 | **Transformer Block** | -0.5 to -1.2 BPB | Very High |

### 🟡 MEDIUM PRIORITY: Hyperparameter Tuning

| # | Strategy | Expected Impact | Effort |
|---|----------|----------------|--------|
| 5 | **Increase hidden to 512+** | -0.05 to -0.1 BPB | Low |
| 6 | **Try 7-gram, 8-gram context** | -0.01 to -0.03 BPB | Low |
| 7 | **Learning rate sweep [0.003, 0.005]** | ±0.02 BPB | Low |
| 8 | **Batch size variation** | -0.01 to -0.02 BPB | Medium |

### 🟢 LOW PRIORITY: Training Tricks

| # | Strategy | Expected Impact | Effort |
|---|----------|----------------|--------|
| 9 | **Ensemble of seeds** | -0.01 to -0.05 BPB | Low |
| 10 | **Gradient accumulation** | 0 BPB (same) | Low |
| 11 | **Mixed precision (f16)** | 0 BPB (same, faster) | Medium |

---

## Critical Architectural Insights

### Why N-gram Hits a Ceiling (~2.5 BPB)

1. **No Long-Range Dependencies**: 6-gram only sees 6 previous tokens
2. **No Shared Computation**: Each context position has separate embeddings
3. **No Parameter Sharing**: ctx1, ctx2, embed are separate matrices
4. **No Hierarchical Features**: Single hidden layer, no depth

### What Transformer Provides

1. **Self-Attention**: Can attend to all positions in sequence
2. **Parameter Sharing**: Same attention/head used across all positions
3. **Multi-Scale Features**: Multiple heads capture different patterns
4. **Deep Hierarchy**: Stacked layers learn increasingly abstract features

### Minimal Transformer for CPU

```
Embedding (128×64) → Positional Encoding (64) →
Multi-Head Attention (4 heads, 16-dim each) →
LayerNorm → FeedForward (64→256→64) →
LayerNorm → LM Head (128×64)
```

Estimated parameters: ~50K (manageable for CPU)

---

## Training Commands Reference

### Current Best (6-gram)
```bash
cargo build --release -p trios-train-cpu --bin ngram_train
./target/release/ngram_train --seed=43 --steps=12000 --hidden=384 --lr=0.004 --wd=0.01
```

### Quick Test (2K steps)
```bash
./target/release/ngram_train --seed=42 --steps=2000 --hidden=128 --lr=0.004
```

### 3-Seed Parallel
```bash
tri train --seeds 42,43,44 --steps=12000 --hidden=384 --lr=0.004 --parallel
```

---

## Laws (Mandatory!)

- **L8**: Every result = commit + push
- **L4**: `cargo test` and `cargo clippy` GREEN before push
- **L7**: Experience to `.trinity/experience/`
- **L2**: Every PR closes an issue
- **L3**: clippy zero warnings

---

## Conclusion

**Current Status**: We've exhausted n-gram architecture improvements (2.5329 BPB).

**Path to <1.5 BPB**: Requires architectural shift from n-gram to attention-based models.

**Recommended Action**: Begin implementing a minimal transformer architecture for CPU.

---

*Last Updated: 2026-04-24*
*φ² + 1/φ² = 3 | TRINITY*
