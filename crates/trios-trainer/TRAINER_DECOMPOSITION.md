# Trainer Decomposition Plan

## Executive Summary

This document provides a comprehensive analysis of the IGLA training pipeline and a detailed decomposed plan for achieving Gate-2 victory (BPB < 1.50).

**Current Status:**
- Champion baseline: BPB=2.2393 @ 27K steps (commit 2446855)
- Local crate: `crates/trios-trainer/` — foundation with forward pass, optimizer, data loading
- Remote repo: `trios-trainer-igla` — canonical IGLA RACE variant with advanced features
- Gap: Incomplete backward pass, missing JEPA/NCA objectives, no HybridAttn integration

---

## 1. Architecture Gap Analysis

### 1.1 Local Crate (`crates/trios-trainer/`)

| Module | Status | Notes |
|--------|--------|-------|
| `config.rs` | ✅ Complete | INV-8 validation, env override |
| `forward.rs` | ✅ Complete | matmul, gelu, layer_norm, softmax |
| `optimizer.rs` | ✅ Complete | AdamW, Muon, φ-schedule |
| `model.rs` | ✅ Complete | MinimalTransformer, MHA, FFN |
| `model_hybrid_attn.rs` | ✅ Complete | INV-13 validation, φ-qk_gain |
| `backward.rs` | ❌ Incomplete | TODO in train_loop.rs: line 90 |
| `train_loop.rs` | ⚠️ Partial | Mock gradients, no real backprop |
| `data/tokenizer.rs` | ⚠️ Basic | Dummy tokenizer only |
| `data/` | ❌ Missing | No FineWeb binary loader |
| `ledger.rs` | ⚠️ Partial | Basic emission, missing embargo |

### 1.2 Remote Repo (`trios-trainer-igla`)

| Module | Status | Notes |
|--------|--------|-------|
| `config.rs` | ✅ Complete | Full schema with model/optimizer/objective/ledger |
| `train_loop.rs` | ✅ Complete | Step loop, eval, ledger emit |
| `ledger.rs` | ✅ Complete | Triplet-validated emit + embargo block |
| `model.rs` | ✅ Complete | Façade for transformer + HybridAttn |
| `optimizer.rs` | ✅ Complete | AdamW + Muon + φ-schedule |
| `objective.rs` | ✅ Complete | NTP + JEPA + NCA multi-objective |
| `jepa.rs` | ✅ Complete | T-JEPA loss + EMA target |
| `data.rs` | ✅ Complete | FineWeb binary loader |
| `gf16.rs` | ✅ Complete | Re-export from trios-golden-float |

### 1.3 Critical Gaps

1. **Backward Pass**: Line 90 in `train_loop.rs` has `// TODO: Implement full gradient computation`
2. **Data Loading**: No real FineWeb binary loader (falls back to dummy data)
3. **Multi-Objective Loss**: JEPA and NCA components not integrated
4. **Checkpoints**: No checkpoint save/load functionality
5. **Gradient Clipping**: Not implemented (critical for stability)

---

## 2. Training Flow Analysis

### 2.1 Current Flow

```
Load Config → Init Model → Init Optimizer
    ↓
For each step:
  1. Sample sequence from data
  2. Forward pass (model.forward)
  3. Compute loss (cross-entropy)
  4. ⚠️ Backward pass (TODO - mock gradients)
  5. Optimizer step
  6. Evaluation at intervals
  7. Emit to ledger at checkpoints
```

### 2.2 Required Flow (IGLA RACE)

```
Load Config → Validate Invariants → Init Model + HybridAttn → Init Optimizer
    ↓
For each step:
  1. Sample batch from FineWeb
  2. Forward pass (embed → ctx → attn → proj)
  3. Multi-objective loss: 0.5*NTP + 0.25*JEPA + 0.25*NCA
  4. Backward pass (full gradient computation)
  5. Gradient clipping (if configured)
  6. Optimizer step (AdamW or Muon)
  7. EMA update (for JEPA target)
  8. GF16 flooring (after 70% steps)
  9. Evaluation (val BPB)
 10. Emit to ledger (R8: step ≥ 4000)
 11. Checkpoint save
```

---

## 3. Decomposed Implementation Plan

### Phase A: Foundation (PR-1 Sync)

**Goal**: Migrate model + optimizer + tokenizer from trios-train-cpu

| Task | File | Priority | Est. Effort | Dependencies |
|------|------|----------|-------------|--------------|
| A1 | Implement `backward.rs` | P0 | 4h | forward.rs |
| A2 | Implement `data/mod.rs` (FineWeb loader) | P0 | 3h | — |
| A3 | Implement `data/tokenizer.rs` (real BPE) | P0 | 2h | — |
| A4 | Add gradient clipping | P0 | 1h | backward.rs |
| A5 | Implement checkpoint save/load | P1 | 3h | train_loop.rs |
| A6 | Add tests for all new modules | P1 | 2h | A1-A5 |

**Acceptance Criteria:**
- Full backprop works for MinimalTransformer
- Real FineWeb data loads correctly
- Checkpoints save and restore
- `cargo test` passes

---

### Phase B: HybridAttn Integration (PR-2)

**Goal**: Integrate HybridAttn with INV-13 validation

| Task | File | Priority | Est. Effort | Dependencies |
|------|------|----------|-------------|--------------|
| B1 | Modify `model.rs` to use HybridAttn | P0 | 2h | model_hybrid_attn.rs |
| B2 | Add config option for attention type | P0 | 1h | config.rs |
| B3 | Implement INV-13 validation at runtime | P0 | 2h | model_hybrid_attn.rs |
| B4 | Add falsifier tests for INV-13 | P1 | 2h | B3 |
| B5 | Benchmark HybridAttn vs MHA | P2 | 2h | B1 |

**Acceptance Criteria:**
- HybridAttn runs correctly with φ-qk_gain
- INV-13 violations detected at runtime
- Falsifier tests pass

---

### Phase C: Multi-Objective Loss (PR-3)

**Goal**: Implement NTP + JEPA + NCA loss combination

| Task | File | Priority | Est. Effort | Dependencies |
|------|------|----------|-------------|--------------|
| C1 | Implement `objective.rs` (NTP loss) | P0 | 1h | — |
| C2 | Implement `jepa.rs` (T-JEPA) | P0 | 4h | forward.rs |
| C3 | Implement NCA objective | P0 | 3h | forward.rs |
| C4 | Implement EMA for JEPA target | P0 | 2h | C2 |
| C5 | Combine losses: 0.5*NTP + 0.25*JEPA + 0.25*NCA | P0 | 1h | C1-C4 |
| C6 | Add entropy band check for NCA [1.5, 2.8] | P1 | 1h | C3 |

**Acceptance Criteria:**
- All three loss components compute correctly
- Loss weights sum to 1.0
- NCA entropy band enforced
- EMA updates work for JEPA

---

### Phase D: GF16 Quantization (PR-4)

**Goal**: Implement Golden Float quantization

| Task | File | Priority | Est. Effort | Dependencies |
|------|------|----------|-------------|--------------|
| D1 | Implement `gf16.rs` (quantization) | P0 | 3h | — |
| D2 | Add config option for GF16 flooring | P0 | 1h | config.rs |
| D3 | Apply GF16 at 70% training steps | P0 | 1h | train_loop.rs |
| D4 | Test quantization accuracy | P1 | 2h | D1 |

**Acceptance Criteria:**
- GF16 quantization preserves φ-anchored values
- Flooring triggers at correct step
- Accuracy impact is measured

---

### Phase E: Validation & Ledger (PR-5)

**Goal**: Complete ledger emission with embargo

| Task | File | Priority | Est. Effort | Dependencies |
|------|------|----------|-------------|--------------|
| E1 | Complete `ledger.rs` (triplet validation) | P0 | 3h | — |
| E2 | Implement embargo block | P0 | 2h | E1 |
| E3 | Enforce R8: step ≥ 4000 for emission | P0 | 1h | E2 |
| E4 | Add ledger push to repo | P1 | 2h | E3 |

**Acceptance Criteria:**
- Ledger rows emit correctly
- Embargo blocks forbidden SHAs
- R8 enforced (no rows before step 4000)

---

## 4. Training Flow Improvements

### 4.1 Data Pipeline

**Current Issues:**
- Single-threaded loading
- No prefetching
- Fallback to dummy data

**Proposed Improvements:**
```rust
// Async data loading with Rayon
use rayon::prelude::*;

struct DataLoader {
    dataset: FineWebDataset,
    batch_size: usize,
    prefetch: usize,
    buffer: Vec<Batch>,
}

impl DataLoader {
    async fn next_batch(&mut self) -> Batch {
        // Prefetch next batches in background
    }
}
```

### 4.2 Gradient Accumulation

**Purpose:** Effective larger batch size without memory blowup

```rust
let grad_accum_steps = config.training.accumulation_steps;
let mut batch_loss = 0.0;

for i in 0..grad_accum_steps {
    let batch = dataloader.next_batch().await?;
    let loss = forward_backward(&mut model, &batch)?;
    batch_loss += loss;

    if (i + 1) % grad_accum_steps == 0 {
        optimizer.step(&mut params, &gradients);
        gradients.zero_();
    }
}
```

### 4.3 Learning Rate Scheduling

**Current:** φ-based decay (good)

**Enhancement:** Warmup + cosine decay

```rust
fn lr_schedule(step: usize, max_steps: usize, warmup: usize) -> f64 {
    let phi = (1.0 + 5.0_f64.sqrt()) / 2.0;

    if step < warmup {
        // Linear warmup
        base_lr * (step as f64 / warmup as f64)
    } else {
        // Cosine decay with φ factor
        let progress = (step - warmup) as f64 / (max_steps - warmup) as f64;
        base_lr * 0.5 * (1.0 + (progress * std::f64::consts::PI).cos()) / phi.powf(progress)
    }
}
```

### 4.4 Mixed Precision Training

**Purpose:** Faster training, lower memory

```rust
// f16 for forward, f32 for gradients
struct MixedPrecisionModel {
    params_fp16: Vec<f16>,
    gradients_fp32: Vec<f32>,
    master_params_fp32: Vec<f32>,
}
```

---

## 5. Bottleneck Analysis

### 5.1 Current Bottlenecks

| Bottleneck | Impact | Solution | Est. Speedup |
|------------|--------|----------|--------------|
| Triple-loop matmul | 🔴 High | SIMD + loop tiling | 4-8x |
| Single-threaded data | 🟡 Medium | Rayon parallelization | 2-4x |
| No gradient clipping | 🟡 Medium | Implement clipping | N/A (stability) |
| Sequential eval | 🟢 Low | Async evaluation | 1.5x |

### 5.2 Optimized Matmul

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

// SIMD-optimized matmul (AVX2/NEON)
#[inline(always)]
unsafe fn vec_fma(a: __m256, b: __m256, c: __m256) -> __m256 {
    #[cfg(target_arch = "x86_64")]
    {
        _mm256_fmadd_ps(a, b, c)
    }
    #[cfg(target_arch = "aarch64")]
    {
        vmlaq_f32(c, a, b)
    }
}
```

---

## 6. Invariant Enforcement

### 6.1 INV-1: LR in φ-band [0.002, 0.007]

```rust
fn validate_lr_phi_band_strict(lr: f32) -> Result<(), TrainingError> {
    const PHI_BAND_MIN: f32 = 0.002;
    const PHI_BAND_MAX: f32 = 0.007;

    if !(PHI_BAND_MIN..=PHI_BAND_MAX).contains(&lr) {
        return Err(TrainingError::LrOutOfBand(lr));
    }
    Ok(())
}
```

### 6.2 INV-13: qk_gain ∈ {φ², φ³}

```rust
const ALLOWED_QK_GAINS: [f32; 2] = [PHI_SQ, PHI_CUBE];

fn validate_qk_gain(gain: f32) -> Result<(), HybridAttnError> {
    if !ALLOWED_QK_GAINS.iter().any(|&g| (g - gain).abs() < 1e-6) {
        return Err(HybridAttnError::QkGainOutsidePhi(gain));
    }
    Ok(())
}
```

### 6.3 R8: Gate-2 floor (step ≥ 4000)

```rust
fn can_emit_ledger_row(step: usize) -> bool {
    step >= 4000  // R8 enforced
}
```

---

## 7. Testing Strategy

### 7.1 Unit Tests

- Every module must have tests
- Coverage target: 80%+
- Fuzzing for critical paths (matmul, gradients)

### 7.2 Integration Tests

```rust
#[test]
fn test_full_training_step() {
    let config = Config::load("test_data/config.toml")?;
    let result = run(&config)?;

    assert!(result.final_bpb.is_finite());
    assert!(result.steps_completed > 0);
}

#[test]
fn test_invariant_violations() {
    // INV-8 violation
    let mut config = Config::default();
    config.training.lr = 0.0005;  // Below band
    assert!(Config::load_with_lr(0.0005).is_err());

    // INV-13 violation
    let mut hybrid_config = HybridAttnConfig::default();
    hybrid_config.qk_gain = 1.0;  // Not φ² or φ³
    assert!(HybridAttn::new(hybrid_config).is_err());
}
```

### 7.3 Regression Tests

- Champion config must reproduce BPB ≈ 2.2393 ± 0.01
- Gate-2 config must complete without crashes

---

## 8. Deployment Strategy

### 8.1 Railway

```bash
# 3-seed parallel deployment
for seed in 42 43 44; do
    railway service create "trainer-seed-$seed"
    railway variables set TRIOS_SEED=$seed --service "trainer-seed-$seed"
    railway up --service "trainer-seed-$seed"
done
```

### 8.2 Docker

```dockerfile
# Multi-stage build
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bins

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/trios-train /usr/local/bin/
ENTRYPOINT ["trios-train"]
```

### 8.3 CI/CD

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - run: cargo clippy -- -D warnings
      - run: cargo test
  build-image:
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: docker/build-push-action@v4
        with:
          push: true
          tags: ghcr.io/ghashtag/trios-trainer-igla:latest
```

---

## 9. Success Metrics

### 9.1 Training Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Champion BPB | 2.2393 ± 0.01 | N/A |
| Gate-2 BPB | < 1.50 | N/A |
| Steps to convergence | < 30K | N/A |
| Training throughput | > 100 tokens/sec | ~10 tokens/sec |

### 9.2 Code Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Test coverage | > 80% | ~40% |
| Clippy warnings | 0 | ✅ |
| Build time | < 5min | ~2min |

---

## 10. Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Backprop bugs | High | High | Extensive unit tests |
| Data loading issues | Medium | High | Fallback to dummy data |
| Numerical instability | Medium | High | Gradient clipping + FP32 master |
| Performance regression | Low | Medium | Benchmarking |
| Invariant violation | Low | High | Runtime validation |

---

## 11. Next Steps (Immediate)

1. **Implement backward.rs** (Priority P0, Est. 4h)
   - Linear backward
   - GELU backward
   - LayerNorm backward
   - Cross-entropy backward

2. **Implement FineWeb data loader** (Priority P0, Est. 3h)
   - Binary format parsing
   - Batch sampling
   - Prefetching

3. **Add gradient clipping** (Priority P0, Est. 1h)
   - L2 norm clipping
   - Configurable threshold

4. **End-to-end test** (Priority P0, Est. 2h)
   - Run 100 steps
   - Verify loss decreases
   - Check BPB computation

---

## Appendix A: IGLA RACE Constants

```rust
// φ = (1 + √5) / 2 ≈ 1.618
const PHI: f32 = 1.618033988749895;

// φ² ≈ 2.618
const PHI_SQ: f32 = 2.618033988749895;

// φ³ ≈ 4.236
const PHI_CUBE: f32 = 4.23606797749979;

// α_φ = φ^(-3) ≈ 0.11803
const ALPHA_PHI: f32 = 0.1180339887498949;

// INV-1: LR in [0.002, 0.007]
const LR_BAND_MIN: f32 = 0.002;
const LR_BAND_MAX: f32 = 0.007;

// INV-8: LR in [0.001, 0.01]
const INV_8_MIN: f32 = 0.001;
const INV_8_MAX: f32 = 0.01;

// NCA entropy band
const NCA_ENTROPY_MIN: f32 = 1.5;
const NCA_ENTROPY_MAX: f32 = 2.8;

// R8: Gate-2 floor
const GATE_2_MIN_STEPS: usize = 4000;
```

---

## Appendix B: File Structure

```
crates/trios-trainer/
├── Cargo.toml
├── src/
│   ├── lib.rs                    # Façade + re-exports
│   ├── config.rs                 # TOML + INV-8 validation
│   ├── forward.rs                # CPU matmul, activations
│   ├── backward.rs               # ✨ TO IMPLEMENT
│   ├── model.rs                  # MinimalTransformer
│   ├── model_hybrid_attn.rs      # HybridAttn + INV-13
│   ├── optimizer.rs              # AdamW, Muon, φ-schedule
│   ├── train_loop.rs             # Main loop
│   ├── data/
│   │   ├── mod.rs                # ✨ TO IMPLEMENT
│   │   └── tokenizer.rs          # ✨ BPE tokenizer
│   ├── objective.rs              # ✨ NTP + JEPA + NCA
│   ├── jepa.rs                   # ✨ T-JEPA + EMA
│   ├── gf16.rs                   # ✨ Golden Float
│   ├── checkpoint.rs             # ✨ Save/restore
│   └── ledger.rs                 # Triplet-validated emit
├── configs/
│   ├── champion.toml
│   ├── gate2-attempt.toml
│   └── needle-v1-mup.toml
└── tests/
    └── reproduce_champion.rs
```

---

**Document Version:** 1.0
**Last Updated:** 2026-04-27
**Author:** Claude (with human guidance)
**Status:** Ready for implementation
