# trios-train-cpu

CPU N-gram language model training for IGLA RACE.

## Migration Status

| Phase | Component | Status |
|-------|-----------|--------|
| M0 | Crate scaffolding | Partial (no Cargo.toml, no lib.rs) |
| M1 | muP scaling (`src/mup.rs`) | Done (10 tests) |
| M2 | LR schedules (`src/schedule.rs`) | Done (6 tests) |
| M3 | Training loop | Not started |
| M4 | Data pipeline | Not started |
| M5 | Tokenizer | Not started |
| M6 | Checkpoint/eval | Not started |
| M7 | Gate-2 push (3 seeds < 1.85 BPB) | Not started |

## Training-Flow V2 Plan

| Phase | Goal | Falsifiable Hypothesis |
|-------|------|----------------------|
| P0 | Audit: reproduce champion BPB 2.2393 | Exact reproduction on seed 43 |
| P1 | Optimizer Lab: Muon vs AdamW | Muon beats AdamW by >5% on 8M |
| P2 | muP Transfer: 8M -> 70M | LR transfer reduces search by 3x |
| P3 | Schedule-Free + WSD vs cosine | SF/WSD beats cosine by >3% |
| P4 | Multi-Objective + EMA (JEPA + NCA) | Joint loss < single-task |
| P5 | Gate-2 Push: 3 seeds < 1.85 | 3 independent seeds under threshold |

## Architecture

```
trios-train-cpu/
  src/
    mup.rs        -- Maximal Update Parametrization scaling
    schedule.rs   -- LR schedules (Cosine, Schedule-Free, WSD)
  configs/lab/
    p2-proxy-8m.toml   -- 8M proxy (d=256, 2L, 4H)
    p2-proxy-24m.toml  -- 24M transfer (d=384, 3L, 6H)
    p2-target-70m.toml -- 70M Gate-2 target (d=384, 4L, 6H)
  assertions/lab/
    p2_transfer.jsonl  -- muP transfer results (empty)
```

## Key Types

- `MupConfig` — muP scaling configuration (ref/target width, per-group LR multipliers)
- `ModelDims` — model dimensions (d_model, n_heads, d_ffn)
- `ParamGroup` — parameter group enum (Embedding, Output, Attention, FFN, LayerNorm)
- `ScheduleType` — LR schedule enum (Cosine, ScheduleFree, Wsd)
- `CosineSchedule` / `WsdSchedule` / `ScheduleFreeState` — schedule implementations

## Configuration

All configs use the Muon optimizer with:
- `lr = 0.0039`, `warmup = 500 steps`, `max_steps = 12000`
- `batch_size = 32`, `grad_clip = 1.0`
- `eta_2d = 0.0235`, `eta_1d = 0.007`, `momentum = 0.95`

## References

- Yang et al. 2022 — Maximal Update Parametrization (muP)
- Defazio 2024 — Schedule-Free learning
- Wen 2024 — WSD (Warmup-Stable-Decay) schedule

> phi^2 + phi^-2 = 3 · TRINITY · IGLA-RACE
