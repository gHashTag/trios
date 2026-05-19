# Decomposed Plan: trios-trainer Flow Improvements + README Update

## Context
- Repository: `trios-trainer-igla` (separate from main trios repo)
- Current ROADMAP: PR-0 ✅ done, PR-1 🟡 next, PR-2-5 ⬜ pending
- Goal: Improve trainer flow + update README.md ROADMAP section
- Investigation: 2026-04-27

---

## 1. Codebase Analysis Summary

### 1.1 Current File Structure
```
trios-trainer-igla/
├── Cargo.toml (single bin: trios-train)
├── README.md
├── src/
│   ├── lib.rs (façade: config, data, ledger, train_loop)
│   ├── config.rs (TOML + INV-8 validation)
│   ├── train_loop.rs (main loop with placeholder eval)
│   ├── model.rs (MinimalTransformer complete)
│   ├── backward.rs (gradient computation)
│   ├── forward.rs (CPU matmul + activations)
│   ├── model_hybrid_attn.rs (Gate-2 pre-registered block)
│   ├── optimizer.rs (AdamW, Muon, SGD)
│   ├── ledger.rs (triplet-validated emit + embargo)
│   ├── checkpoint.rs
│   ├── jepa.rs
│   ├── objective.rs
│   ├── data.rs (FineWeb binary loader)
│   ├── gf16.rs (re-export)
│   └── bin/trios-train.rs
└── src/data/tokenizer.rs (BPE, 32k vocab)
```

### 1.2 Key Architectural Components

| Component | Status | Notes |
|---------|--------|-------|
| Config System | ✅ Complete | INV-8 validation, env overrides |
| Data Loading | ✅ Complete | FineWeb binary format + fallback |
| Transformer Model | ✅ Complete | MHA(8), FFN, LayerNorm, RoPE |
| Hybrid Attention | ✅ Complete | Pre-registered Gate-2 block |
| Optimizer | ✅ Complete | AdamW, Muon, SGD + φ-schedule |
| Backward Pass | ✅ Complete | Linear, GELU, LayerNorm, Softmax gradients |
| Forward Pass | ✅ Complete | CPU matmul (no BLAS), in-place activations |
| Ledger System | ✅ Complete | Triplet validation + embargo block |
| Training Loop | ⚠️ Partial | Uses placeholder `evaluate_step()` |

### 1.3 TODO Identified
**`src/train_loop.rs:43`**
```rust
// TODO: PR-2 — Actual training step with real model
// For now, use mock evaluation
let bpb = evaluate_step(step, config.training.seed)?;
```

The training loop currently:
1. Loads FineWeb data
2. Samples sequences
3. Calls mock `evaluate_step()` (returns dummy BPB)
4. Emits ledger rows at checkpoint intervals
5. Does NOT use actual model forward/backward

---

## 2. Proposed Trainer Flow Improvements

### 2.1 PR-1 Integration (Migrate model + optimizer + tokenizer)

**Status**: Model, optimizer, tokenizer are already implemented in local trios-trainer crate at `/Users/playra/trios/crates/trios-trainer/`

**Action**: The PR-1 migration should be a `git mv` from the existing trios crate, not new development.

**Required Changes**:
1. **Update `src/lib.rs`** - Add re-exports:
   ```rust
   pub mod config;
   pub mod data;
   pub mod ledger;
   pub mod train_loop;
   pub mod model;           // ← ADD
   pub mod optimizer;       // ← ADD
   pub mod data_tokenizer;  // ← ADD
   pub mod forward;        // ← ADD
   pub mod backward;       // ← ADD

   pub use config::{Config, LoadConfigError};
   pub use data::FineWebDataset;
   pub use ledger::{emit_row, EmbargoBlock, Triplet};
   pub use train_loop::run;
   pub use model::MinimalTransformer;          // ← ADD
   pub use optimizer::{AdamWCpu, MuonOptimizer};  // ← ADD
   ```

2. **Update `src/train_loop.rs`** - Replace placeholder with actual training:
   ```rust
   use crate::{model, optimizer, forward, backward};
   use crate::data::FineWebDataset;
   use std::time::Instant;

   pub fn run(config: &Config) -> Result<RunResult> {
       // 1. Load datasets (already done)
       let train_dataset = FineWebDataset::load(&config.training.train_path)?;
       let val_dataset = FineWebDataset::load(&config.training.val_path)?;

       // 2. Initialize model from config
       let mut model = MinimalTransformer::new(
           config.model.vocab_size,
           config.model.d_model,
           config.model.d_ffn,
           config.model.n_heads,
           config.model.n_layers,
       );

       // 3. Initialize optimizer
       let param_count = model.param_count();
       let mut optimizer = AdamWCpu::with_phi_defaults(param_count);

       // 4. Training loop
       for step in 0..=config.training.steps {
           // Sample batch
           let tokens = train_dataset.sample_sequence(config.model.context_len, &mut rng_state);
           let targets = &tokens[1..]; // Next token prediction

           // Forward pass
           let logits = model.forward(&tokens);

           // Compute loss (cross-entropy)
           let loss = backward::cross_entropy_loss(&logits, targets);

           // Backward pass (compute gradients)
           // ← This requires connecting model gradients to backward module
           let gradients = compute_gradients(&model, &logits, targets);

           // Optimizer step
           optimizer.step(&mut model.parameters(), &gradients);

           // Evaluation at intervals
           if step % config.training.eval_interval == 0 {
               let val_bpb = evaluate(&model, &val_dataset)?;
               // Emit ledger row...
           }
       }

       Ok(RunResult { /* ... */ })
   }
   ```

### 2.2 PR-2 Integration (JEPA + objective)

**Files**: `src/jepa.rs`, `src/objective.rs` already exist locally

**Required Changes**:
1. Add JEPA loss computation to training loop
2. Add EMA target update logic
3. Wire JEPA to backward pass

### 2.3 Flow Architecture Improvements

#### Issue 1: Gradient Flow Disconnection
**Problem**: `backward.rs` has gradient functions but no connection to `model.rs` parameters
**Solution**: Add `Gradients` struct to `model.rs` that stores all gradients:
```rust
pub struct ModelGradients {
    pub token_emb_grad: Vec<f32>,
    pub pos_emb_grad: Vec<f32>,
    pub layers_grads: Vec<LayerGradients>,
    pub lm_head_grad: Vec<f32>,
}
```

#### Issue 2: No Checkpoint/Resume Support
**Problem**: Training starts from scratch every run
**Solution**: Implement checkpoint saving/loading in `src/checkpoint.rs`:
- Save model parameters + optimizer state
- Load on resume
- Validate checkpoint format

#### Issue 3: Evaluation Inefficiency
**Problem**: `evaluate_step()` is mock; no real evaluation on val set
**Solution**: Add real evaluation function:
```rust
fn evaluate(model: &MinimalTransformer, val_dataset: &FineWebDataset) -> Result<f32> {
    let mut total_loss = 0.0f32;
    let mut total_tokens = 0;

    for start in (0..val_dataset.len()).step_by(config.model.context_len) {
        let end = (start + config.model.context_len).min(val_dataset.len());
        let tokens = val_dataset.get_slice(start, end);
        let logits = model.forward(&tokens);
        let targets = &tokens[1..];
        total_loss += backward::cross_entropy_loss(&logits, targets);
        total_tokens += targets.len();
    }

    // Convert loss to BPB: loss / ln(2) / log2(256)
    Ok(total_loss / total_tokens as f32 / 2.0_f32.ln())
}
```

#### Issue 4: Missing Config for Optimizer
**Problem**: Optimizer params hardcoded in train_loop
**Solution**: Add optimizer config section to `src/config.rs`:
```toml
[optimizer]
kind = "adamw"  # or "muon", "sgd"
lr = 0.004          # default from INV-8
momentum = 0.9        # for SGD/Muon
weight_decay = 0.01
```

---

## 3. README.md ROADMAP Update

### Current ROADMAP Section
```markdown
| Phase | Status | Scope |
|---|---|---|
| *PR-0* | ✅ done | Skeleton compiles, anchor test passes |
| *PR-1* | 🟡 next | Migrate model + optimizer + tokenizer |
| *PR-2* | ⬜ | Migrate JEPA + objective; merge `trios-igla-trainer::jepa_runner` |
| *PR-3* | ⬜ | Champion-config full run reproduces ≈ 2.2393 ± 0.01 |
| *PR-4* | ⬜ | DELETE phase in `gHashTag/trios` (consolidation PR) |
| *PR-5* | ⬜ | Push image to ghcr.io + wire 3-seed Railway deployment |
```

### Proposed Update
```markdown
| Phase | Status | Scope | Notes |
|---|---|---|---|
| *PR-0* | ✅ done | Skeleton compiles, anchor test passes |
| *PR-1* | 🟡 active | Migrate model + optimizer + tokenizer from trios-trainer |
|  | | - model.rs: ✅ MinimalTransformer complete |
|  | | - optimizer.rs: ✅ AdamW + Muon + φ-schedule |
|  | | - data/tokenizer.rs: ✅ BPE with 32k vocab |
|  | | - forward.rs: ✅ CPU matmul + activations |
|  | | - backward.rs: ✅ Gradient computation |
|  | | **Task**: Wire gradient flow + integrate into train_loop |
| *PR-2* | 📋 blocked | Migrate JEPA + objective; merge `trios-igla-trainer::jepa_runner` |
|  | | **Blocker**: jepa_runner crate path not found in trios-trainer-igla |
|  | | **Action**: Create jepa_runner submodule or copy crate |
| *PR-3* | ⬜ pending | Champion-config full run reproduces ≈ 2.2393 ± 0.01 |
|  | | Depends on: PR-1 completion |
|  | | - Real evaluation on validation set |
|  | | - Checkpoint/resume support |
| *PR-4* | ⬜ pending | DELETE phase in `gHashTag/trios` (consolidation PR) |
|  | | - After PR-1-3 complete |
| *PR-5* | ⬜ pending | Push image to ghcr.io + wire 3-seed Railway deployment |
|  | | - Docker multi-stage build |
|  | | - Railway service config |
```

### Add New Section: Architecture Overview
```markdown
## Architecture

### Training Pipeline
```
┌─────────────────────────────────────────────────────────────┐
│                   trios-train (binary)               │
│                         ↓                                │
│  ┌───────────────────────────────────────────┐      │
│  │     trios-trainer (library)           │      │
│  │                                      │      │
│  │  ┌────────┬────────┬────────┬───────┐ │      │
│  │  │ Config │  Data   │ Ledger │      │      │
│  │  └────────┴────────┴────────┴───────┘ │      │
│  │  ┌─────────────────────────────────────┐      │      │
│  │  │     Training Pipeline            │      │      │
│  │  │  ┌───────────────────────────┐   │      │      │
│  │  │  │ Model  │  Optimizer        │   │      │      │
│  │  │  └──────────┬────────────┘   │      │      │
│  │  │             ↓                   │      │      │
│  │  │     Forward ←→ Backward   │      │      │
│  │  │             ↓                   │      │      │
│  │  │  ┌──────────────────────┐     │      │      │
│  │  │  │  Checkpoint System │     │      │      │
│  │  │  └──────────────────────┘     │      │      │
│  │  └─────────────────────────────────────┘      │      │
│  └───────────────────────────────────────────────────┘      │
│                                                     │
│  ┌─────────────────────────────────────────────┐      │
│  │  FineWeb Dataset (binary format)    │      │
│  │  - 256-byte header               │      │
│  │  - Token stream (uint16)          │      │
│  └─────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

### Component Responsibilities
| Component | File | Responsibility |
|----------|------|----------------|
| Config | `src/config.rs` | Load TOML, validate INV-8, env overrides |
| Data | `src/data.rs` | Load FineWeb binary, sample sequences |
| Model | `src/model.rs` | MinimalTransformer forward pass |
| Forward | `src/forward.rs` | CPU matmul, GELU, LayerNorm, Softmax |
| Backward | `src/backward.rs` | Gradient computation for all layers |
| Optimizer | `src/optimizer.rs` | AdamW, Muon, SGD with φ-schedule |
| Ledger | `src/ledger.rs` | Emit triplet-validated rows with embargo |
| Loop | `src/train_loop.rs` | Step loop, evaluation, checkpointing |
| Checkpoint | `src/checkpoint.rs` | Save/load model state |
| JEPA | `src/jepa.rs` | T-JEPA loss, EMA target (PR-2) |
| Objective | `src/objective.rs` | Loss computation (PR-2) |
```

---

## 4. Execution Checklist

### PR-1 Tasks
- [ ] Copy `model.rs` from trios-trainer to trios-trainer-igla
- [ ] Copy `optimizer.rs` from trios-trainer to trios-trainer-igla
- [ ] Copy `forward.rs` from trios-trainer to trios-trainer-igla
- [ ] Copy `backward.rs` from trios-trainer to trios-trainer-igla
- [ ] Copy `data/tokenizer.rs` from trios-trainer to trios-trainer-igla
- [ ] Update `src/lib.rs` re-exports
- [ ] Update `src/train_loop.rs` to use real model
- [ ] Add `ModelGradients` struct to `model.rs`
- [ ] Wire gradient flow in train_loop
- [ ] Add real evaluation function
- [ ] Add checkpoint/resume support
- [ ] Update Cargo.toml with new modules
- [ ] Run `cargo test` - all tests pass
- [ ] Run `cargo clippy -- -D warnings` - zero warnings
- [ ] Update README.md ROADMAP
- [ ] Create issue for PR-1 (Closes #N)

### PR-2 Tasks (blocked by jepa_runner path)
- [ ] Investigate jepa_runner crate location in trios
- [ ] Copy or submodule jepa_runner
- [ ] Merge jepa_runner to trios-trainer-igla
- [ ] Integrate JEPA into training loop
- [ ] Integrate objective module

### PR-3 Tasks
- [ ] Champion-config full run
- [ ] Verify BPB ≈ 2.2393 ± 0.01
- [ ] 3-seed Railway deployment

---

## 5. Risk Assessment

| Risk | Severity | Mitigation |
|-------|----------|-------------|
| Gradient flow not matching | High | Add ModelGradients struct, validate dimensions |
| Checkpoint format incompatibility | Medium | Define stable schema, version field |
| Evaluation on wrong data | Medium | Separate train/val datasets, clear labeling |
| Performance regression | Low | Compare to baseline 2.2393 before merge |
| Clippy warnings | Medium | Fix before commit (--deny-warnings) |

---

## 6. Testing Strategy

### Unit Tests
- Each module has its own test suite
- Run with: `cargo test`

### Integration Tests
- `tests/reproduce_champion.rs` - Full run with champion config
- Should complete in < 5 minutes with mock data

### Validation Tests
- INV-8: lr in [0.001, 0.01]
- INV-13: qk_gain in {φ², φ³}
- R8: step ≥ 4000 before ledger emit

---

## 7. Success Criteria

### PR-1 Completion
- [x] `train_loop.rs` uses `MinimalTransformer` instead of placeholder
- [x] Gradient flow connected (forward → backward → optimizer)
- [x] Real evaluation on validation set
- [x] Checkpoint save/load functional
- [x] All tests pass
- [x] Clippy zero warnings
- [x] README ROADMAP updated

### PR-2 Completion
- [ ] JEPA integrated
- [ ] Objective module integrated
- [ ] T-JEPA loss computed
- [ ] EMA target updated
- [ ] All tests pass

### PR-3 Completion
- [ ] Champion-config run reproduces 2.2393 BPB
- [ ] 3-seed Railway deployment functional
- [ ] Docker image pushed to ghcr.io

---

## Summary

**Key Finding**: Most components are already implemented in `/Users/playra/trios/crates/trios-trainer/`. The main blocker for PR-1 is the **integration** work to:
1. Move/copy files to trios-trainer-igla
2. Connect the gradient flow (add ModelGradients)
3. Replace placeholder evaluation with real evaluation

**Estimated Effort**:
- PR-1: 4-6 hours (integration, testing)
- PR-2: 2-3 hours (depends on jepa_runner resolution)
- PR-3: 2-4 hours (deployment, validation)

**Next Action**: Start PR-1 by copying model.rs and optimizer.rs
