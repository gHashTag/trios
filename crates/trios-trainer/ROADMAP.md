# trios-trainer Roadmap

## Context

This crate is the **single source of truth** for IGLA RACE training pipeline.
Reference: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla)

## Phase Status

| Phase | Status | Description | Owner |
|-------|--------|-------------|--------|
| **PR-0** | ✅ Complete | Skeleton crate with empty training loop |
| **PR-1** | 🟡 In Progress | Migrate model + optimizer + data + tokenizer |
| **PR-2** | ⬜ Pending | Migrate JEPA + objective + invariants |
| **PR-3** | ⬜ Pending | Champion-config full run reproduces ≈2.2393 ± 0.01 |
| **PR-4** | ⬜ Pending | DELETE phase in gHashTag/trios (consolidation PR) |
| **PR-5** | ⬜ Pending | Railway publish + 3-seed deploy for Gate-2 |

## PR-1: Model + Optimizer + Data Migration

### Scope
Migrate from `trios-train-cpu` crate:
- `transformer.rs` → `model.rs` (façade pattern)
- `optimizer.rs` (AdamW + Muon + φ-schedule)
- `data.rs` + tokenizer.rs
- Config schema extensions

### Source Files (trios-train-cpu)
- `src/transformer.rs` (~15K lines) → split
- `src/optimizer.rs` (~22K lines)
- `src/data.rs` → FineWeb binary format
- `src/tokenizer.rs` → byte-level encoding

### Target Files (trios-trainer)
- `src/model.rs` → placeholder
- `src/optimizer.rs` → placeholder
- `src/data.rs` → partial (only token sampling)
- `src/data/tokenizer.rs` → to create

## PR-2: JEPA + Objective Migration

### Scope
Migrate from `trios-igla-trainer`:
- `src/jepa/` → T-JEPA loss + EMA target
- `src/objective.rs` → NCA objective
- `src/invariants.rs` → INV-8, R8, embargo enforcement

### Source Files (trios-igla-trainer)
- `src/jepa_runner.rs` → main JEPA training logic
- `src/objective.rs` → NCA + JEPA combination

### Target Files (trios-trainer)
- `src/jepa/` → directory (empty)
- `src/objective.rs` → placeholder
- `src/invariants.rs` → to create

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
| **INV-8**: LR φ-band | ⬜ Config validation only, not yet enforced in training loop |
| **R8**: Gate-2 floor | ⬜ Config shows checkpoint_interval=1000 (violates R8) |
| **Embargo**: SHA block | ✅ Implemented in `ledger.rs` |
| **Triplet**: Row format | ✅ Implemented in `ledger.rs` |

## Config Files

| File | Purpose | Champion-BPB | Steps | Status |
|------|---------|-------------|-------|--------|
| `champion.toml` | Baseline reproduction | 2.2393 | 27 000 | ✅ Validated |
| `gate2-attempt.toml` | HybridAttn push | 2.2393 | 30 000 | ⬜ Pending PR-2 |
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

1. **R8 Violation**: `champion.toml` has `checkpoint_interval=1000` which violates R8 (step ≥ 4000)
2. **Mock Training**: Current `train_loop.rs` uses dummy evaluation, not real model
3. **Missing Model**: `src/model.rs` is empty, `src/forward.rs`, `src/backward.rs` are new files
