# trios-trainer — IGLA Training Single Source of Truth

Run IGLA training on **any machine**, **any VPS**, **Railway**.

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

## Configs

All configs are in `configs/` as TOML files:

| Config | Purpose | Target |
|--------|---------|--------|
| `champion.toml` | Reproduce baseline | BPB=2.2393 @ 27K |
| `gate2-attempt.toml` | Gate-2 push | BPB < 1.85 @ 4K+ |
| `needle-v1-map.toml` | μP transfer variant | Experimental |

## Invariants (INV-1..INV-10)

The trainer enforces:
- **INV-8**: LR in φ-band `[0.001, 0.01]` (proven)
- **INV-2**: ASHA prune threshold `3.5 = φ² + φ⁻² + 0.5`

All emits are triplet-validated: `BPB=<v> @ step=<N> seed=<S> sha=<7c>`.

## Migration Status

| PR | Status | Description |
|----|--------|-------------|
| PR-0 | ✅ Complete | Skeleton crate (empty) |
| PR-1 | 🟡 Active | Migrate model + optimizer + data + tokenizer |
| PR-2 | ⬜ Pending | Migrate JEPA + objective + invariants |
| PR-3 | ⬜ Pending | Champion-config full run reproduces ≈ 2.2393 ± 0.01 |
| PR-4 | ⬜ Pending | DELETE dead crates + R1 cleanup |
| PR-5 | ⬜ Pending | Railway publish + 3-seed deploy |

### PR-1 Components (Active)

| Component | File | Status |
|----------|------|--------|
| MinimalTransformer | `src/model.rs` | ✅ Complete (MHA + FFN) |
| AdamWCpu | `src/optimizer.rs` | ✅ Complete (φ-based defaults) |
| Gradients | `src/backward.rs` | ✅ Complete (linear, GELU, LayerNorm) |
| Forward | `src/forward.rs` | ✅ Complete (matmul, activations) |
| FineWebDataset | `src/data.rs` | ✅ Complete (binary loader) |
| BPE Tokenizer | `src/data/tokenizer.rs` | ✅ Complete (32k vocab) |
| Training Loop | `src/train_loop.rs` | ✅ Integrated (real model) |
| ModelGradients | `src/model.rs` | ✅ Added (gradient container) |

### PR-1 Remaining Tasks

- ⬜ Wire gradient flow (backward → optimizer integration)
- ⬜ Add checkpoint/resume support
- ⬜ Fix champion.toml (add train_path, val_path)
- ⬜ Run full champion config (27K steps → BPB ≈ 2.2393)

See [ROADMAP.md](./ROADMAP.md) for detailed phase breakdown and known issues.

## Anchor

φ² + φ⁻² = 3 — Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   train_loop.rs                    │
│                         ↓                               │
│  ┌───────────────────────────────────────────┐  │
│  │  MinimalTransformer (model.rs)            │  │
│  │                                      │  │
│  │  ┌────────┬────────┬────────┬───────┐ │  │
│  │  │  MHA    │  FFN    │  LMHead │  │  │
│  │  └────────┴────────┴────────┴───────┘ │  │
│  │  ┌─────────────────────────────────────┐  │  │
│  │  │  AdamWCpu (optimizer.rs)          │  │  │
│  │  └─────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────┘  │
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
