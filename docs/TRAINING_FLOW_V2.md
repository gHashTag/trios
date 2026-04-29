# Training Flow V2 — Phased Development Plan

## Overview

Training Flow V2 replaces the monolithic training pipeline with a phased approach tied to falsifiable hypotheses and pre-registered exit criteria.

## Phases

| Phase | Name | Hypothesis | Exit Criteria |
|-------|------|------------|---------------|
| P0 | Data pipeline | Raw text can be tokenized at 10K tok/s | Throughput benchmark passes |
| P1 | Baseline model | 2-layer MLP achieves BPB < 1.5 on TinyStories | Loss convergence within 1K steps |
| P2 | Ternary quantization | {-1,0,+1} weights maintain >95% FP32 accuracy | Accuracy gap < 5% on validation |
| P3 | GF16 encoding | GF16 format reduces model size 2x with <0.5% accuracy loss | Roundtrip error < 1e-6 |
| P4 | phi-Schedule | phi-adaptive LR converges in 1/phi steps vs cosine | Wall-clock speedup > 1.3x |
| P5 | Full integration | End-to-end pipeline produces sub-16MB model | All P0-P4 criteria met |

## Decision Matrix

| Metric | Threshold | Action if Failed |
|--------|-----------|-----------------|
| BPB (bits-per-byte) | < 1.5 | Revert to wider hidden dim |
| Ternary accuracy gap | < 5% | Add Straight-Through Estimator |
| GF16 roundtrip error | < 1e-6 | Debug encode/decode path |
| Model size | < 16MB | Increase compression ratio |
| Training stability | No NaN in 10K steps | Reduce LR by phi factor |

## References

- EXP-001: Trinity 3^3 physics
- EXP-010: Ternary block alignment
- Issue #321: train-cpu crate scaffolding
- Issue #327: README + Training-Flow V2
