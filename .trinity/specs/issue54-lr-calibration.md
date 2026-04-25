# Issue #54: φ-LR schedule calibration (Issue #53 extended)

**Status:** Active
**Date:** 2026-04-20
**Authors:** Dmitrii Vasilev
**Issue:** #54
**Depends on:** #53 (H3 LR fix confirmed)
**Blocks:** #33 (empirical validation)

---

## Executive Summary

Calibrate 3 LR schedules to determine optimal training strategy for φ-init.

**Why this matters:**
- Issue #53 confirmed: α_φ = 0.118034 as raw LR → BPB 18.60 (EXPLOSION)
- LR 3e-4 → BPB 1.82 (VALID)
- But we don't know: is flat LR best? cosine? or phi-decay to α_φ floor?

**Scientific hypothesis:**
α_φ serves as ASYMPTOTIC FLOOR in phi-decay schedule, NOT as initial gradient step.

---

## Background: The Scientific Discovery

From Issue #53 session 2026-04-20:

```
Hypothesis FAILED: α_φ = 0.118034 ≡ LR
  Result: BPB 18.60 (explosion)

Hypothesis PASSED: LR = 3e-4 (standard)
  Result: BPB 1.82 (near SOTA 1.1228)

Gap to SOTA: 0.70 BPB → requires better schedule, not just constant LR
```

**Scientific correction for PhD:**
```
OLD CLAIM: "α_φ is universal coupling → optimal LR"
NEW CLAIM: "α_φ = stable point of spectrum, NOT optimal gradient step.
            Two different physical roles require RG-flow interpretation."
```

This is HONEST science — we're documenting boundary of applicability.

---

## Experimental Design

### Three LR Schedules (pre-registered)

| Schedule | Formula | Parameters | Hypothesis |
|----------|---------|------------|------------|
| (a) flat | LR = 3e-4 | constant | Baseline |
| (b) cosine | LR = 3e-4 * 0.5 * (1 + cos(π * t / T)) | T=1000 | Standard decay |
| (c) phi-decay | LR = 3e-4 * φ^(-t/τ) → α_φ floor | τ=200 | α_φ as asymptote |

**Key hypothesis (c):** α_φ = 0.118034 serves as NATURAL FLOOR in phi-decay,
not as INITIAL value. This preserves physical meaning while fixing the training.

### Experimental Protocol

```rust
pub struct CalibConfig {
    pub max_steps: usize,       // 1000 (quick calibration)
    pub batch_size: usize,       // 8
    pub seq_len: usize,          // 128
    pub seed: u64,               // 42 (single seed for calibration)
    pub val_every: usize,        // 10
    pub output_dir: String,      // "experiments/lr_calibration"
}
```

### Expected Outcomes

| Winner | Scientific Implication | PhD Chapter 7 |
|--------|----------------------|---------------|
| flat | No decay needed for this scale | Simple LR sufficient |
| cosine | Standard decay works well | φ-init still beneficial |
| phi-decay | α_φ has physical role as floor | **Trinity strengthened** |

---

## Implementation Plan

### Phase 1: Extend trios-phi-schedule crate

Add 3 new functions to `crates/trios-phi-schedule/src/lib.rs`:

```rust
/// Flat LR schedule — constant 3e-4
pub fn flat_lr(_step: usize) -> f32 { 3e-4 }

/// Cosine decay from 3e-4 to 0
pub fn cosine_lr(step: usize, max_steps: usize) -> f32 {
    let progress = (step as f32) / (max_steps as f32);
    3e-4 * 0.5 * (1.0 + (std::f32::consts::PI * progress).cos())
}

/// Phi decay from 3e-4 with α_φ as asymptotic floor
pub fn phi_decay_lr(step: usize, tau: usize) -> f32 {
    const ALPHA_PHI: f32 = 0.118034;
    const PHI: f32 = 1.618034;
    let decay = PHI.powf(-(step as f32 / tau as f32));
    (3e-4 * decay).max(ALPHA_PHI)  // Floor at α_φ
}
```

### Phase 2: Create calibration binary

`crates/trios-experiments/src/bin/lr_calibration.rs`:

```rust
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(short, long, default_value = "1000")]
    steps: usize,

    #[arg(short, long, default_value = "42")]
    seed: u64,

    #[arg(short, long, default_value = "experiments/lr_calibration")]
    output: String,
}

fn main() {
    // Run 3 schedules in parallel
    // Save results to {output}/{schedule}.csv
    // Compare final BPB
}
```

### Phase 3: Run calibration

```bash
cargo run --bin lr_calibration -- --steps 1000 --seed 42
```

Output structure:
```
experiments/lr_calibration/
├── flat.csv           # step,loss,bpb
├── cosine.csv
├── phi_decay.csv
└── summary.json       # {flat: X, cosine: Y, phi_decay: Z, winner: "..."}
```

---

## Acceptance Criteria

| # | Criterion | Target |
|---|-----------|--------|
| 1 | 3 LR schedules implemented | All compile and test |
| 2 | Calibration binary runs | No panics, produces 3 CSVs |
| 3 | Each schedule completes 1000 steps | All have 100 rows |
| 4 | Final BPB recorded | `summary.json` exists |
| 5 | Winner identified | Clear best BPB or tie reported |
| 6 | Commit message | `exp(lr): calibrate 3 LR schedules — refs #54` |

---

## Definition of Done

- [ ] `trios-phi-schedule/src/lib.rs` has 3 new LR functions
- [ ] `trios-experiments/src/bin/lr_calibration.rs` implemented
- [ ] 3 CSV files generated (flat, cosine, phi_decay)
- [ ] `summary.json` identifies winner
- [ ] Commit: `exp(lr): calibrate 3 LR schedules — refs #54`
- [ ] Issue #33 updated with winning schedule

---

## Next Actions

1. [ ] Implement 3 LR schedules in trios-phi-schedule
2. [ ] Create lr_calibration binary
3. [ ] Run 1000-step calibration
4. [ ] Compare final BPB
5. [ ] Update Issue #33 with winner
6. [ ] Proceed to 3-seed validation

---

**Closes:** #54
