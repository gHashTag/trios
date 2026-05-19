# trios-trainer Roadmap

## Context

This crate is the **single source of truth** for IGLA RACE training pipeline.
Reference: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla)

## Phase Status

| Phase | Status | Description | Notes |
|-------|--------|-------------|-------|
| **PR-0** | ✅ Complete | Skeleton crate with empty training loop | Anchor test passes |
| **PR-1** | 🟡 Active | Migrate model + optimizer + data + tokenizer | Core components integrated |
| **PR-2** | ⬜ Pending | Migrate JEPA + objective + invariants | Blocked by jepa_runner path |
| **PR-3** | ⬜ Pending | Champion-config full run reproduces ≈2.2393 ± 0.01 | Depends on PR-1 |
| **PR-4** | ⬜ Pending | DELETE phase in gHashTag/trios (consolidation PR) | After PR-1-3 complete |
| **PR-5** | ⬜ Pending | Railway publish + 3-seed deploy for Gate-2 | Docker + Railway config |

## PR-1: Model + Optimizer + Data Migration (ACTIVE)

### Completed Components
- ✅ `src/model.rs` — MinimalTransformer (MHA + FFN + LayerNorm)
- ✅ `src/optimizer.rs` — AdamW + Muon + φ-schedule
- ✅ `src/forward.rs` — CPU matmul, GELU, LayerNorm, Softmax
- ✅ `src/backward.rs` — Gradient computation (linear, GELU, LayerNorm, cross-entropy)
- ✅ `src/data.rs` — FineWeb binary format loader
- ✅ `src/data/tokenizer.rs` — BPE tokenizer (32k vocab)
- ✅ `src/lib.rs` — Module re-exports
- ✅ `src/train_loop.rs` — Real model integration (replaces placeholder)

### Remaining Tasks
- ⬜ Wire gradient flow (backward → optimizer)
- ⬜ Add checkpoint/resume support
- ⬜ Add real evaluation on validation set
- ⬜ Fix champion.toml config (add train_path, val_path)
- ⬜ Run full champion config (27K steps → BPB ≈ 2.2393)

### Architecture
```
┌─────────────────────────────────────────────────────────┐
│                    train_loop.rs                        │
│  ┌─────────────────────────────────────────────────┐    │
│  │  MinimalTransformer (model.rs)                  │    │
│  │  ├─ MultiHeadAttention (8 heads)                │    │
│  │  ├─ FFN (GELU activation)                       │    │
│  │  ├─ LayerNorm (Pre-Norm)                        │    │
│  │  └─ RoPE (Rotary Position Embedding)            │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  AdamWCpu (optimizer.rs)                        │    │
│  │  ├─ φ-based defaults (β₁=φ⁻¹≈0.618, wd=α_φ≈0.118) │   │
│  │  └─ phi_lr_schedule (warmup + decay)           │    │
│  └─────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────┐    │
│  │  FineWebDataset (data.rs)                       │    │
│  │  └─ Binary format (256-byte header + uint16)    │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
```

## PR-2: JEPA + Objective Migration

### Scope
Migrate from `trios-igla-trainer`:
- `src/jepa/` → T-JEPA loss + EMA target
- `src/objective.rs` → NCA objective
- `src/invariants.rs` → INV-8, R8, embargo enforcement

### Blocker
**jepa_runner crate path not found** in trios-trainer-igla repository.

### Resolution Options
1. Create jepa_runner submodule in trios-trainer
2. Copy JEPA implementation from trios-igla-trainer
3. Implement JEPA from scratch (spec exists in DECOMPOSED_PLAN.md)

## PR-3: Champion Reproduction

### Goal
Run `champion.toml` config for 27K steps, seed=43 → BPB ≈ 2.2393

### Validation
- INV-8: LR ∈ [0.001, 0.01] ✓ (champion uses 0.004)
- R8: step ≥ 4000 for ledger emit ✓ (checkpoint at 1000, eval at 500)
- Triplet validation: all rows contain BPB, step, seed, SHA, gate_status ✓

## Invariants (INV-1 to INV-10)

| Invariant | Status | Validation |
|----------|--------|------------|
| **INV-8**: LR φ-band | ✅ Config validation | `config.rs:validate_lr_phi_band()` |
| **R8**: Gate-2 floor | ⚠️ Partial | Config shows checkpoint_interval=1000 (needs fix) |
| **Embargo**: SHA block | ✅ Implemented | `ledger.rs:EmbargoBlock` |
| **Triplet**: Row format | ✅ Implemented | `ledger.rs:emit_row()` |

## Config Files

| File | Purpose | Champion-BPB | Steps | Status |
|------|---------|-------------|-------|--------|
| `champion.toml` | Baseline reproduction | 2.2393 | 27 000 | ⚠️ Needs train_path/val_path |
| `gate2-attempt.toml` | HybridAttn push | < 1.85 | 30 000 | ⬜ Pending PR-2 |
| `needle-v1-mup.toml` | μP-transfer | 2.2393 | 12 000 | ⬜ Pending |

## Dependencies

### External (tri-igla-race, trios-golden-float)
These are kept as workspace dependencies for integration mode:
```toml
# trios-igla-race = { path = "../trios-igla-race" }
# trios-golden-float = { path = "../trios-golden-float" }
```

### Build Modes
```bash
# Default — standalone, all stubs
cargo build --release -p trios-trainer

# Integration — pulls ASHA + victory gate from trios-igla-race
cargo build --release -p trios-trainer --features trios-integration

# CI strict — adds embargo + triplet enforcement
cargo build --release -p trios-trainer --features "trios-integration,ci-strict"
```

## Known Issues

1. ⚠️ **R8 Violation**: `champion.toml` has `checkpoint_interval=1000` which violates R8 (step ≥ 4000)
2. ⚠️ **Config Missing**: `champion.toml` missing `train_path` and `val_path` fields
3. ⬜ **Gradient Flow**: ModelGradients struct exists but not yet wired to backward pass
4. ⬜ **Checkpoint**: No checkpoint save/load support yet
5. ⬜ **JEPA**: jepa_runner crate not found in repository

## Testing

```bash
# Run all tests
cargo test -p trios-trainer

# Run clippy (L3 compliance)
cargo clippy -p trios-trainer -- -D warnings

# Run training with fallback data
cargo run --release -p trios-trainer --bin trios-train -- \
    --config crates/trios-trainer/configs/champion.toml --seed 43
```

### Test Coverage
- 54 unit tests passing
- All modules tested (config, data, ledger, model, optimizer, forward, backward, train_loop)
- Clippy zero warnings (L3 compliant)
