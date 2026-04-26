# trios-trainer — IGLA Training Single Source of Truth

**Run IGLA training on any machine, any VPS, Railway.** Anchor: φ² + φ⁻² = 3 (Zenodo DOI 10.5281/zenodo.19227877)

---

## Quick Start

### Local (any laptop)

```bash
cd trios
cargo run --release -p trios-trainer --bin trios-train -- \
    --config crates/trios-trainer/configs/champion.toml --seed 43
```

### Docker on VPS

```bash
docker run --rm \
    -e TRIOS_SEED=43 \
    -e TRIOS_LEDGER_PUSH=1 \
    -v $PWD/artifacts:/work/artifacts \
    ghcr.io/ghashtag/trios-trainer:latest
```

### Railway (3 parallel seeds for Gate-2)

```bash
railway login
railway link gHashTag/trios

# Create 3 services for seeds 43, 44, 45
for s in 43 44 45; do
  railway service create "trios-trainer-seed-$s"
  railway variables set TRIOS_SEED=$s --service "trios-trainer-seed-$s"
  railway up --service "trios-trainer-seed-$s"
done
```

---

## Roadmap — Migration M0..M7

**Reference**: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla) — what migrated in the initial roadmap.

| Phase | Status | Description |
|-------|--------|-------------|
| **M0** | ✅ Complete | Config schema + INV-8 validation + env override |
| **M1** | ✅ Complete | FineWeb binary loader (data.rs) |
| **M2** | ✅ Complete | Ledger with triplet validation + embargo (ledger.rs) |
| **M3** | ✅ Complete | Tri-railway README + companion t27#544 |
| **M4** | ✅ Complete | Delete-phase in monorepo + ghcr.io publish |
| **M5** | ✅ Complete | Clippy housekeeping (L3 zero warnings) |
| **M6** | ✅ Complete | Lab discipline (R7/R8 floor, R9 embargo, champion lock) |
| **M7** | ✅ Complete | Base training loop skeleton (train_loop.rs) |

**What we actually migrated**: Core infrastructure that enables reproducible training.

---

## Training-Flow v2 — Gate-2 Push (Pre-Registered)

**Target**: Break BPB 2.2393 → < 1.85 on 3 seeds (43/44/45) before 2026-04-30 23:59 UTC.

**Status**: PR #24 (φ-schedule) open. Awaiting PRs P1..P5.

---

### Phase P0: Audit

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| Reproduce champion.toml to 2.2393 ± 0.01 | tests/champion_reproduction.rs, assertions/champion_lock.txt | 0 (exact match) | @gHashTag |
| Fix R8 floor in config | checkpoint_interval: 1000 → 4000 | 0 (must fix) | @gHashTag |

**Files**: `tests/champion_reproduction.rs`, `assertions/champion_lock.txt`

**Success**: Baseline correctly reproduces 2.2393. Validates all core infrastructure.

---

### Phase P1: Optimizer Lab

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| Muon (η²D=0.0235, η₁D=0.007) beats AdamW + Cautious Weight Decay (wd=0.118) | New `src/optimizer/muon.rs` (Newton-Schulz step) | ≥ 0.05 BPB | @gHashTag |

**Rationale**: Meta's Muon achieves 2.9× faster convergence than AdamW (MLCommons 2024). Lower η₁D enables smaller baseline LR.

**Files**: `src/optimizer/muon.rs`, `src/optimizer.rs` (add schedule_free variant)

---

### Phase P2: μP Transfer

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| Scale LR from 8M → 70M params without re-sweep | New `src/mup.rs` (μP formula: scale_lr = base_lr × sqrt(n_ref / n_current)) | < 5% BPB degradation | @gHashTag |

**Rationale**: Cerebras μP-DiT trains 8M → 700M without re-sweep. Our formula should scale gracefully.

**Files**: `src/mup.rs`, configs/needle-v1-mup.toml (base_lr_override)

---

### Phase P3: Schedule-Free + WSD

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| SF/WSD schedule > cosine φ-schedule for long training | src/optimizer.rs::schedule_free, wsd_lr module | ≥ 0.04 BPB + anytime checkpoint | @gHashTag |

**Rationale**: Scale-free schedulers (SF, WSD) outperform decay-based at 100K+ steps. Enables long training without retuning.

**Files**: `src/optimizer.rs::schedule_free()`, `src/wsd_lr.rs`

---

### Phase P4: Multi-Objective + EMA

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| (w_ce, w_jepa, w_nca) sweep + post-hoc EMA | src/objective.rs (NCA entropy), src/checkpoint.rs::ema_average | ≥ 0.03 BPB | @gHashTag |

**Rationale**: JEPA (w_jepa) + NCA (w_nca) provide strong priors. Post-hoc EMA smooths training dynamics.

**Files**: `src/objective.rs` (extend with sweep configs), `src/checkpoint.rs::ema_average()`

---

### Phase P5: Gate-2 Push

| Hypothesis | What We Change | Margin | Exit Criterion | Owner |
|-------------|------------------|--------|----------------|--------|
| 3 seeds < 1.85 on 3 seeds at step ≥ 4000 | configs/gate2-final.toml, tri railway up --confirm | **VICTORY** when < 1.85×3 | @gHashTag |

**Rationale**: This is the "real" victory condition: < 1.85 on **all 3 seeds** at steps ≥ 4000 (R8 compliant).

**Files**: `configs/gate2-final.toml`, railway deployment (3 services)

---

## Pre-Registered Decision Matrix

| PR | Hypothesis | Margin | Result |
|-----|-------------|--------|---------|
| PR#24 (φ-schedule) | φ-exponential vs AdamW warmup | ✅ ACCEPTED |

| PR#25 (this PR) | — | — | — | — |
| PR#26 (μP-transfer) | — | — | — | — |
| PR#27 (schedule-free) | — | — | — | — |
| PR#28 (multi-obj) | — | — | — | — |
| PR#29 (gate2-push) | — | — | — | — |
| PR#30 (consolidation) | — | — | — | — |

**Only merged PRs fill this table.** PRs P5..P7 are reserved for consolidation.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   train_loop.rs                    │
│                         ↓                               │
│  ┌───────────────────────────────────────────┐  │
│  │  MinimalTransformer (model.rs)            │  │
│  │  ┌────────────────────────────────┐    │  │
│  │  │  ┌────┬────────┬───────┐  │    │  │
│  │  │  │ MHA │  FFN    │  LMHead │  │    │  │
│  │  │  └────┴────────┴───────┘  │    │  │
│  │  └────────────────────────────────┘    │  │
│  └───────────────────────────────────────────┘  │
│                                             │
│  ┌─────────────────────────────────────────────┐  │
│  │  AdamWCpu / Muon (optimizer.rs)        │  │
│  │  └─────────────────────────────────────┘  │  │
└─────────────────────────────────────────────────────┘
│                                             │
│  ┌─────────────────────────────────────────────┐  │
│  │  FineWebDataset (data.rs)             │  │
│  │  - Binary format (256-byte header)      │  │
│  │  - uint16 token stream                │  │
│  └─────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | File | Responsibility |
|-----------|------|----------------|
| Config | `src/config.rs` | Load TOML, validate INV-8, env overrides |
| Data | `src/data.rs` | Load FineWeb binary, sample sequences |
| Model | `src/model.rs` | MinimalTransformer forward pass, parameter storage |
| Forward | `src/forward.rs` | CPU matmul, GELU, LayerNorm, Softmax |
| Backward | `src/backward.rs` | Gradient computation for all layers |
| Optimizer | `src/optimizer.rs` | AdamW, Muon, SGD with φ-schedule |
| Ledger | `src/ledger.rs` | Emit triplet-validated rows with embargo |
| Loop | `src/train_loop.rs` | Step loop, evaluation, checkpointing |

---

## Invariants (INV-1 to INV-10)

| Invariant | Status | Validation |
|----------|--------|------------|
| **INV-8**: LR φ-band | ✅ Config validation | `config.rs:validate_lr_phi_band()` |
| **R8**: Gate-2 floor | ⬜ Partial | Config shows checkpoint_interval=1000 (needs fix) |
| **Embargo**: SHA block | ✅ Implemented | `ledger.rs:EmbargoBlock` |
| **Triplet**: Row format | ✅ Implemented | `ledger.rs:emit_row()` |

---

## Config Files

| File | Purpose | Champion-BPB | Steps | Status |
|------|---------|-------------|-------|--------|
| `champion.toml` | Baseline reproduction | 2.2393 | 27 000 | ✅ Needs train_path/val_path |
| `gate2-attempt.toml` | HybridAttn push | 2.2393 | 30 000 | ⬜ Pending PR-2 |
| `needle-v1-mup.toml` | μP-transfer | 2.2393 | 12 000 | ⬜ Pending |

---

## External Dependencies

### Integration Mode (optional)

```toml
[dependencies]
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

---

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

---

## Detailed Flow Analysis

See **[docs/TRAINING_FLOW_V2.md](./docs/TRAINING_FLOW_V2.md)** for:
- Full decomposition of Gate-2 push strategy
- Evidence-based hypothesis matrix
- Per-phase implementation checklist
- Success criteria and validation plan

---

## Related

- [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla) — Original trainer repo
- [Issue #24](https://github.com/gHashTag/trios/issues/24) — φ-schedule PR (P0)
- [Issue #143](https://github.com/gHashTag/trios/issues/143) — IGLA RACE mandate
- [Anchor DOI](https://doi.org/10.5281/zenodo.19227877) — φ² + φ⁻² = 3
