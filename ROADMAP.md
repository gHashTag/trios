# IGLA Training Roadmap

## Current Status: PR-1 COMPLETE

### Phase Overview

| Phase | Status | Description | Target |
|-------|--------|-------------|--------|
| PR-0 | ✅ DONE | Skeleton crate compiles | - |
| PR-1 | ✅ DONE | Model/Optimizer migration | Full training stack |
| PR-2 | 🚧 NEXT | JEPA integration | Champion reproduction |
| PR-3 | ⏳ Planned | Champion reproduction | BPB ≤ 2.24 @ 27K |
| PR-4 | ⏳ Planned | DELETE phase | Cleanup |
| PR-5 | ⏳ Planned | Docker/Railway | Distributed training |

---

## PR-0: Skeleton Foundation ✅

**Status**: COMPLETE

**Deliverables**:
- ✅ `trios-trainer` crate with 4-module facade
- ✅ Config loading with INV-8 validation (LR φ-band)
- ✅ FineWeb dataset loader with fallback
- ✅ Ledger emission with embargo block
- ✅ Mock training loop (TODO: PR-2)

**Files**:
- `src/lib.rs` - Public API
- `src/config.rs` - TOML schema + INV-8
- `src/data.rs` - FineWeb loader
- `src/ledger.rs` - Triplet validation
- `src/train_loop.rs` - Mock loop

---

## PR-1: Model & Optimizer Migration ✅

**Status**: COMPLETE (untracked files)

**Deliverables**:
- ✅ `forward.rs` - CPU matmul, GELU, LayerNorm, softmax
- ✅ `backward.rs` - Gradients, cross-entropy, clipping
- ✅ `model.rs` - MinimalTransformer (MHA + FFN)
- ✅ `model_hybrid_attn.rs` - HybridAttn with φ-qk_gain
- ✅ `optimizer.rs` - AdamW, Muon, φ-schedule
- ✅ `data/tokenizer.rs` - BPE tokenizer (32k)

**Key Features**:
- φ-based constants: β₁=φ⁻¹≈0.618, α_φ=φ⁻³≈0.118
- Muon optimizer with NS5 orthogonalization
- INV-13: qk_gain ∈ {φ², φ³}
- GF16 quantization support

---

## PR-2: Real Training Loop 🚧 NEXT

**Status**: IN PROGRESS

**Objectives**:
1. Replace mock `train_loop.rs` with real training
2. Integrate forward/backward pass
3. Wire up optimizer (AdamW/Muon)
4. Implement checkpoint saving
5. Add validation evaluation

**Tasks**:
```rust
// train_loop.rs upgrade path
pub fn run(config: &Config) -> Result<RunResult> {
    // 1. Load FineWeb data
    let train_data = FineWebDataset::load(&config.training.train_path)?;
    let val_data = FineWebDataset::load(&config.training.val_path)?;

    // 2. Initialize model
    let mut model = MinimalTransformer::new(
        config.model.vocab_size,
        config.model.d_model,
        config.model.d_model * config.model.ff_mult,
        config.model.n_heads,
        config.model.n_layers,
    );

    // 3. Initialize optimizer
    let mut optimizer = AdamWCpu::with_phi_defaults(model.param_count());

    // 4. Training loop
    for step in 0..=config.training.steps {
        // Forward
        let tokens = train_data.sample_sequence(seq_len, &mut rng);
        let logits = model.forward(&tokens);

        // Loss
        let loss = cross_entropy_loss(&logits, &targets);

        // Backward
        let mut grads = vec![0.0f32; model.param_count()];
        backward_pass(&model, &logits, &targets, &mut grads);

        // Optimizer step
        optimizer.step(&mut model.parameters, &grads);

        // Evaluation
        if step % eval_interval == 0 {
            let val_bpb = evaluate(&model, &val_data);
            emit_row(&config.ledger.path, &row, &embargo)?;
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Real training steps complete (no mock BPB)
- [ ] Checkpoint saving works
- [ ] Validation BPB computed
- [ ] Ledger rows emitted at checkpoints
- [ ] Champion config trains to convergence

---

## PR-3: Champion Reproduction

**Status**: PLANNED

**Objective**: Replicate champion baseline BPB=2.2393 @ 27K steps, seed=43

**Config** (`configs/champion.toml`):
```toml
name = "champion"
steps = 27_000
seed = 43
d_model = 256
n_layers = 2
n_heads = 4
lr = 0.004  # INV-8 validated
hybrid_attn = false
w_ce = 1.0
w_jepa = 0.0
w_nca = 0.0
```

**Gate-2 Target**: BPB ≤ 2.24

**Gate-final Target**: BPB ≤ 1.50 (30% above N-gram baseline 2.53)

---

## PR-4: DELETE Phase

**Status**: PLANNED

**Objective**: Clean up after successful reproduction

**Tasks**:
1. Remove mock evaluation code
2. Consolidate duplicate implementations
3. Finalize module structure
4. Update documentation

---

## PR-5: Docker & Railway Deployment

**Status**: PLANNED

**Objective**: Train on any VPS, Railway, or local machine

**Deliverables**:
- Dockerfile with CUDA support
- Railway service template
- Distributed training orchestration
- Artifact logging

---

## Technical Invariants

| Invariant | Description | Status |
|-----------|-------------|--------|
| INV-8 | LR ∈ [0.001, 0.01] (φ-band) | ✅ Enforced |
| INV-13 | qk_gain ∈ {φ², φ³} | ✅ Enforced |
| R8 | step ≥ 4000 to emit ledger row | ⏳ TODO |

---

## References

- **IGLA RACE**: https://github.com/gHashTag/trios-trainer-igla
- **Issue #32**: CPU training configuration
- **Issue #67**: LR fix for tied embeddings
- **Coq proofs**: `trinity-clara/proofs/igla/`
