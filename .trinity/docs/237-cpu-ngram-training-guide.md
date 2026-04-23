# ONE-SHOT: CPU N-Gram Language Model Training — Complete Agent Guide

> Read once — you can train. Everything you need is here.

## The `tri train` Command

```bash
# Build first
cargo build --release -p trios-train-cpu --bin ngram_train
cargo build --release --bin tri

# Quick test (1 seed, 2000 steps)
tri train --seeds 42 --steps 2000 --hidden 128 --lr 0.004

# Full production run (3 seeds, parallel, 12K steps)
tri train --seeds 42,43,44 --steps 12000 --hidden 128 --lr 0.004 --parallel
```

## Model Architecture (4-gram + ReLU Hidden)

```
 tokens[i-2]  →  ctx2[vocab×dim]  ─┬
 tokens[i-1]  →  ctx1[vocab×dim]  ─┤ weighted sum (0.3 + 0.7 + 1.0)
 tokens[i]    →  embed[vocab×dim] ─┘
                                    │
                               LayerNorm(ε=1e-5)
                                    │
                           proj[hidden×dim] (linear)
                                    │
                               ReLU activation
                                    │
                          lm_head[vocab×hidden] (linear)
                                    │
                               softmax → log prob
                                    │
                      cross-entropy loss → BPB (bits per byte)
```

## Hyperparameters (record-breaking)

| Parameter | Value | Note |
|-----------|-------|------|
| vocab | 128 | byte-level tokens |
| dim | 64 | embedding dimension |
| hidden | 128 | hidden layer size |
| seq | 64 | sequence length |
| lr | 0.004 | base learning rate |
| steps | 12000 | with cosine schedule |
| seeds | 42, 43, 44 | 3 seeds for ±std |

## Data

```
data/tinyshakespeare.txt  — 1.1MB, 1,115,394 characters
Split: 90% train / 10% val
```

## History of CPU BPB Records

| Date | Model | BPB | What worked |
|------|-------|-----|-------------|
| Apr 21 | Bigram | 3.90 | Basic bigram |
| Apr 21 | Trigram + AdamW | 3.26 | +context, +AdamW |
| Apr 22 | 4-gram + ReLU h=64 | 2.958 | +4-gram, +ReLU hidden |
| Apr 22 | 4-gram + ReLU h=128 | 2.877 | ×2 hidden dim |
| **Apr 22** | **4-gram fixed backward** | **2.780** | **backward bugfix** |

## CRITICAL BUGS WE FOUND

### Bug #1: lm_head dimension mismatch
```rust
// BEFORE — lm_head size vocab×dim
lm_head: (0..vocab * dim).map(|_| rng() * lim_o).collect(),
// AFTER — lm_head size vocab×hidden
lm_head: (0..vocab * hidden).map(|_| rng() * lim_o).collect(),
```

### Bug #2: backward pass truncated hidden
```rust
// BEFORE — only h.min(d) neurons got gradients
for hi in 0..h.min(d) {
    g_head[vi * d + hi] += grad * hidden[hi];
// AFTER — all hidden neurons
for hi in 0..h {
    g_head[vi * h + hi] += grad * hidden[hi];
```

## Optimizer: AdamW with PHI-parameters

```
beta1 = 1/φ = 0.618   (golden ratio)
beta2 = 0.999
eps   = 1e-8
wd    = 0.04
lr    = cosine schedule
```

## What to try next (to push below 2.70)

1. hidden=192 — more capacity
2. GELU instead of ReLU — smoother gradients
3. Residual connection — embed + hidden skip
4. LayerNorm before lm_head
5. Dropout 0.1
6. Warmup 500 steps
7. Weight decay 0.01 instead of 0.04

## Training Time (M1 Mac)

| hidden | 1 seed 12K steps | 3 seeds parallel |
|--------|-----------------|------------------|
| 64 | ~8 min | ~8 min |
| 128 | ~15 min | ~15 min |
| 256 | ~30 min+ | ~30 min+ |

## Laws (mandatory)

- L8: Every result = commit + push
- L4: cargo test and cargo clippy GREEN before push
- L7: Experience to `.trinity/experience/`

---
*φ² + 1/φ² = 3 | TRINITY*
