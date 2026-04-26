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
    -v $PWD/assertions:/work/assertions \
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
| `needle-v1-mup.toml` | μP transfer variant | Experimental |

## Invariants (INV-1..INV-10)

The trainer enforces:
- **INV-8**: LR in φ-band `[0.001, 0.01]` (proven)
- **INV-2**: ASHA prune threshold `3.5 = φ² + φ⁻² + 0.5`

All emits are triplet-validated: `BPB=<v> @ step=<N> seed=<S> sha=<7c>`.

## Migration Status

| PR | Status | Description | Owner |
|----|--------|-------------|--------|
| PR-1 | ✅ Complete | Skeleton crate (empty) |
| PR-2 | 🟡 In Progress | Migrate model + optimizer + data + tokenizer |
| PR-3 | ⬜ Pending | Migrate JEPA + objective + invariants |
| PR-4 | ⬜ Pending | DELETE dead crates + R1 cleanup |
| PR-5 | ⬜ Pending | Railway publish + 3-seed deploy |

See [ROADMAP.md](./ROADMAP.md) for detailed phase breakdown and known issues.

## Anchor

φ² + φ⁻² = 3 — Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
