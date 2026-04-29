# trios-train-cpu

CPU-first language model training for the IGLA RACE. Transformer, JEPA, and GF16 training pipelines with phi-orthogonal initialization.

## Architecture

```
trios-train-cpu
├── src/
│   ├── model.rs              — Transformer model (phi-init, residual mix)
│   ├── transformer.rs        — Multi-head attention + feed-forward blocks
│   ├── forward.rs            — Forward pass
│   ├── backward.rs           — Backpropagation (manual autograd)
│   ├── optimizer.rs          — SGD, AdamW, Muon optimizers
│   ├── tokenizer.rs          — Byte-level tokenizer
│   ├── data.rs               — Dataloader (FineWeb, synthetic)
│   ├── objective.rs          — ASHA rung schedule + BPB objective
│   ├── pipeline.rs           — Full training loop orchestration
│   ├── gf16.rs               — GF16 arithmetic for quantized training
│   ├── phi_ortho_init.rs     — Phi-orthogonal weight initialization
│   ├── swa_phi.rs            — Stochastic Weight Averaging (phi-schedule)
│   ├── sliding_eval.rs       — Sliding window evaluation
│   ├── invariants.rs         — L5 phi-identity runtime checks
│   └── jepa/                 — JEPA predictor module
├── examples/
│   ├── phase_a_warmup.rs     — Phase A: warmup training
│   ├── phase_b_fine.rs       — Phase B: fine-tuning
│   ├── phase_b_fine_v2.rs    — Phase B v2: improved schedule
│   ├── run_igla_phase_ab.rs  — Combined A+B runner
│   ├── smart_phase_b.rs      — Adaptive phase B
│   ├── trinity_sweep.rs      — Hyperparameter sweep
│   ├── gf16_test.rs          — GF16 training test
│   └── decision_summary.rs   — Sweep decision summary
└── benches/
    └── bench.rs              — Criterion benchmarks
```

## Migration Status (M0–M7)

| Phase | Description | Status |
|-------|-------------|--------|
| M0 | Import from trios-trainer-igla | Done |
| M1 | Model + forward pass | Done |
| M2 | Backward pass + gradients | Done |
| M3 | Optimizer (SGD/AdamW/Muon) | Done |
| M4 | Data pipeline + tokenizer | Done |
| M5 | ASHA objective + rung schedule | Done |
| M6 | GF16 quantized training | In progress |
| M7 | JEPA predictor | In progress |

## Training-Flow V2 (Gate-2 Push)

Target: **BPB < 1.85 on 3 seeds** (Gate-2 criterion from `trios-igla-race::victory`).

| Phase | Experiment | Hypothesis | Status |
|-------|-----------|------------|--------|
| P0 | Audit champion reproduction | Reproduce BPB=2.2393 @ 27K | Done |
| P1 | Optimizer Lab (Muon vs AdamW) | Muon > AdamW by >0.05 BPB | In progress |
| P2 | μP Transfer (8M → 70M) | μP scales loss monotonically | Planned |
| P3 | Schedule-Free + WSD | SF/WSD beats cosine by >0.03 BPB | Planned |
| P4 | Multi-Objective + EMA (JEPA + NCA) | Joint objective < single-task | Planned |
| P5 | Gate-2 Push (3 seeds < 1.85) | Quorum 3/3 passes victory check | Planned |

## Quick Start

```bash
# Build
cargo build --release -p trios-train-cpu

# Phase A warmup (single seed, 2000 steps)
cargo run --release -p trios-train-cpu --example phase_a_warmup

# Full A+B training
cargo run --release -p trios-train-cpu --example run_igla_phase_ab

# Benchmark
cargo bench -p trios-train-cpu
```

## Dependencies

- `trios-core` — shared types and traits
- `trios-physics` — phi-identity invariants
- `trios-phi-schedule` — phi-based learning rate schedule
- `trios-data` — dataset loading
- `trios-ternary` — ternary weight quantization

## Constitutional Compliance

- **L5 (PHI-IDENTITY):** `invariants.rs` checks phi^2 + phi^-2 = 3 at runtime
- **L4 (TESTABILITY):** ASHA rung schedule validated in `objective.rs`
- **L7 (UNITY):** No shell scripts; all training via Rust binaries
