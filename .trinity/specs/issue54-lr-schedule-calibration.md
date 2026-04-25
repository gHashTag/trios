# Issue #54: φ-LR Schedule Calibration

**Status:** Active
**Date:** 2026-04-20
**Authors:** Dmitrii Vasilev
**Issue:** #54
**Depends on:** #32 (CPU training baseline)
**Blocks:** #33 (Empirical validation)

---

## Executive Summary

Calibrate 3 LR schedules to determine optimal gradient decay strategy for IGLA-GF16.

**Why this matters:**
- BPB 1.82 achieved with flat LR 3e-4 (Issue #53 extended)
- Need to test if phi-decay improves final BPB
- Scientific question: Does α_φ work as asymptotic floor in decay schedule?

---

## Scientific Hypothesis

**H0:** Flat LR 3e-4 achieves same final BPB as phi-decay
**H1:** Phi-decay (3e-4 → α_φ) achieves lower final BPB (ΔBPB ≥ 0.1)

**Key insight from Issue #53:**
- α_φ = 0.118034 is NOT a valid initial LR (BPB explodes to 18.60)
- α_φ may serve as ASYMPTOTIC FLOOR in decay schedule

---

## LR Schedule Specifications

### (a) flat_3e4

```rust
fn flat_lr(step: usize) -> f64 {
    3e-4  // Constant
}
```

### (b) cosine_3e4_to_0

```rust
fn cosine_lr(step: usize, max_steps: usize) -> f64 {
    let progress = step as f64 / max_steps as f64;
    3e-4 * (1.0 + (std::f64::consts::PI * progress).cos()) / 2.0
}
```

### (c) phi_decay_3e4_to_alpha_phi (hypothesis)

```rust
fn phi_decay_lr(step: usize, tau: usize) -> f64 {
    let phi = 1.618033988749895_f64;
    let alpha_phi = 1.0 / (phi * phi * phi);  // ≈ 0.11803

    if step < 100 {
        3e-4  // Warmup plateau
    } else {
        let t = (step - 100) as f64 / tau as f64;
        3e-4 * alpha_phi.powf(-t)
    }
}
```

Where τ = max_steps / (φ × 27) ≈ 37 for 1000 steps.

---

## Experimental Design

### Parameters

| Parameter | Value |
|-----------|-------|
| max_steps | 1000 |
| base_lr | 3e-4 |
| batch_size | 4 |
| seq_len | 128 |
| seed | 42 (fixed for calibration) |
| log_every | 100 |

### Conditions

| Schedule | Description |
|----------|-------------|
| (a) flat | Constant LR 3e-4 |
| (b) cosine | Cosine decay 3e-4 → 0 |
| (c) phi_decay | Phi decay 3e-4 → α_φ (τ=37) |

---

## Acceptance Criteria

| # | Criterion | Target |
|---|-----------|--------|
| 1 | flat LR baseline | BPB ≤ 1.90 (verify reproducibility) |
| 2 | cosine LR | BPB ≤ 1.90 |
| 3 | phi-decay LR | BPB ≤ 1.85 (hypothesis) |
| 4 | CSV output | 3 files with step,loss,bpb,lr |
| 5 | Winner selection | Lowest final BPB |
| 6 | Commit | `exp(lr): calibrate 3 LR schedules (flat, cosine, phi-decay) — refs #54` |

---

## Output Structure

```
experiments/lr_calibration/
├── flat.csv          # step,loss,bpb,lr
├── cosine.csv
├── phi_decay.csv
└── results.json      # {flat: {final_bpb: X, time: Y}, cosine: {...}, phi_decay: {...}}
```

---

## Definition of Done

- [ ] 3 LR schedules implemented in `trios-phi-schedule`
- [ ] Calibration experiment runs to completion
- [ ] 3 CSV files generated with loss curves
- [ ] `results.json` with final BPB for each schedule
- [ ] Winner identified and documented in Issue #33
- [ ] Commit with refs #54
- [ ] Experience log entry written

---

## Implementation Plan

1. Update `trios-phi-schedule/src/lib.rs` with 3 schedule types
2. Create `experiments/lr_calibration/run_calibration.rs`
3. Run 1000-step calibration with seed=42
4. Compare final BPB, select winner
5. Commit and update Issue #33

---

**Closes:** #54
