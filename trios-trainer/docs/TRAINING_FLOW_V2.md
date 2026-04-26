# Training Flow v2 — Closing the Gap to BPB < 1.85

## Context

**Target**: Achieve BPB < 1.85 on 3 seeds (43, 44, 45) by 2026-04-30 23:59 UTC.
**Baseline**: 2.2393 BPB (champion config, 27K steps, seed=43).
**Gap**: Need ~40% improvement (2.2393 → < 1.85).

## Pre-Registered Decision Matrix

**Empty by Design**: Filled only by merged PRs (R5/R7).

| Phase | Hypothesis | What we change | Margin | Owner |
|-------|------------|---------------|--------|--------|
| **P0** Audit | champion.toml repro 2.2393 ± 0.01 | tests/champion_reproduction.rs, assertions/champion_lock.txt | 0 (floor) |
| **P1** Optimizer Lab | Muon (η²D=0.0235, η_1D=0.007) > AdamW | new src/optimizer/muon.rs (Newton-Schulz, Polar-Express) | ≥ 0.05 BPB |
| **P2** muP Transfer | LR* with 8M → 70M without re-sweep | new src/mup.rs | <5% degradation |
| **P3** Schedule-Free + WSD | SF/WSD > cosine φ-schedule | src/optimizer.rs::schedule_free / wsd_lr | ≥ 0.04 BPB + anytime |
| **P4** MultiObj + EMA | (w_ce, w_jepa, w_nca) sweep + post-hoc EMA | src/objective.rs, src/checkpoint.rs::ema_average | ≥ 0.03 BPB |
| **P5** Gate-2 Push | 3 seeds < 1.85 on step ≥ 4000 | configs/gate2-final.toml + railway up --confirm | merged victory PR |

## Detailed Phase Breakdown

### P0: Audit Phase

**Goal**: Reproduce champion.toml config exactly: BPB = 2.2393 ± 0.01 at 27K steps, seed=43.

**What we validate**:
1. Config loading (INV-8: lr in φ-band [0.001, 0.01])
2. FineWeb data path resolution
3. Model architecture (d_model=384, n_layers=6, d_ffn=1536, n_heads=8)
4. Optimizer initialization (AdamW with φ-defaults: β₁=φ⁻¹≈0.618, wd=α_φ≈0.11803)
5. Evaluation at correct intervals
6. Ledger emission with triplet format

**Exit Criterion**:
```rust
// Final BPB must be within 2.2293 to 2.2493 (±0.01)
assert!(final_bpb >= 2.2293 && final_bpb <= 2.2493);
```

**Artifacts**:
- `tests/champion_reproduction.rs` — Full run, asserts final BPB
- `assertions/champion_lock.txt` — Expected hash of model weights (for detecting uncommitted changes)

**Owner**: DELTA (trios-train-cpu team)

---

### P1: Optimizer Lab — Muon > AdamW

**Hypothesis**: Muon optimizer (η²D=0.0235, η_1D=0.007) improves over AdamW by ≥ 0.05 BPB.

**What we implement**:
1. `src/optimizer/muon.rs`:
   - Newton-Schulz orthogonalization: `X' = X (XᵀX)ᵀ⁰·⁵ / ‖(XᵀX)‖`
   - Polar-Express parameterization: `X = R ⊙ U`
   - η-schedule: η²D warmup → η_1D plateau

2. Ablation script: `scripts/run_muon_ablation.sh`
   - Same architecture, same data, same seeds
   - Compare: AdamW baseline vs Muon variants
   - Output: `results/muon_vs_adamw.jsonl`

**Exit Criterion**:
```rust
// Muon must beat AdamW by at least 0.05 BPB
assert!(muon_bpb < adamw_bpb - 0.05);
```

**Margin**: ≥ 0.05 BPB improvement over AdamW baseline.

**Owner**: CHARLIE (Optimizer specialist)

---

### P2: muP Transfer — 8M → 70M

**Hypothesis**: Transferring 8M model to 70M parameter space with LR* (multiplied LR per layer) achieves < 5% BPB degradation.

**What we implement**:
1. `src/mup.rs` — Matrix-upwise Parameterization (MuP):
   ```rust
   pub struct MuPModel {
       base_model: MinimalTransformer,  // 8M base
       projection_matrix: Vec<f32>,    // 8M × 70M
       lr_multipliers: Vec<f32>,      // Per-layer lr scaling
   }
   ```

2. Training loop adaptation:
   ```rust
   // Apply LR* per layer
   for (layer_idx, param_range) in layer_ranges.iter().enumerate() {
       let lr = base_lr * lr_multipliers[layer_idx];
       optimizer.step_layer(&mut params[param_range], &gradients[param_range], lr);
   }
   ```

3. Evaluation: Compare 70M-fine-tuned vs 8M-base on same data.

**Exit Criterion**:
```rust
// Degradation must be < 5%
let degradation = (bpb_70m - bpb_8m) / bpb_8m;
assert!(degradation < 0.05);
```

**Margin**: < 5% BPB degradation (i.e., > 95% of base performance).

**Owner**: ECHO (Scale specialist)

---

### P3: Schedule-Free + WSD — No Cosine

**Hypothesis**: SF/WSD learning rate schedule (warmup → constant) achieves ≥ 0.04 BPB improvement over cosine φ-schedule.

**What we implement**:
1. `src/optimizer.rs::schedule_free`:
   ```rust
   pub enum ScheduleType {
       SF,       // Stochastic Flooding
       WSD,      // Weight Standardized Decay
   }

   pub fn schedule_free_lr(step: usize, max_steps: usize, schedule: ScheduleType) -> f64 {
       let warmup_ratio = (step as f64) / (max_steps as f64);
       if warmup_ratio < 0.1 {
           // Warmup phase
           base_lr * warmup_ratio * 10.0  // SF
       } else {
           // Constant phase
           base_lr
       }
   }
   }
   ```

2. Ablation: Same setup, compare SF vs WSD vs φ-cosine.

**Exit Criterion**:
```rust
// Must beat cosine φ-schedule by ≥ 0.04 BPB
assert!(sf_bpb < cosine_bpb - 0.04);
```

**Margin**: ≥ 0.04 BPB improvement over φ-cosine baseline.

**Owner**: BRAVO (Schedule specialist)

---

### P4: MultiObj + EMA — JEPA + NCA

**Hypothesis**: Combined JEPA + NCA objective with post-hoc EMA achieves ≥ 0.03 BPB improvement.

**What we implement**:
1. `src/objective.rs` — Multi-objective:
   ```rust
   pub struct TrainingObjective {
       w_ce: f32,           // Cross-entropy weight
       w_jepa: f32,         // JEPA weight
       w_nca: f32,           // NCA weight
   }

   pub fn compute_loss(&self, logits, targets, jepa_out, nca_out) -> f32 {
       self.w_ce * cross_entropy(logits, targets)
       + self.w_jepa * jepa_loss(jepa_out, targets)
       + self.w_nca * nca_loss(nca_out, targets)
   }
   ```

2. `src/jepa.rs` — JEPA block:
   - T-JEPA loss (future prediction)
   - EMA target network (τ = 0.999)
   - Masked attention

3. `src/checkpoint.rs::ema_average` — EMA of checkpoints:
   ```rust
   pub fn ema_average(checkpoint_dir: &Path, tau: f32) -> ModelWeights {
       // Average last N checkpoints with exponential decay
       let checkpoints = load_last_n_checkpoints(checkpoint_dir, N=5);
       let ema_weights = weighted_average(&checkpoints, tau);
       ema_weights
   }
   ```

4. Hyperparameter sweep: `scripts/run_multiobj_sweep.sh`
   - Sweep w_ce ∈ [1.0, 0.9, 0.8, 0.7]
   - Sweep w_jepa ∈ [0, 0.1, 0.2, 0.3]
   - Sweep w_nca ∈ [0, 0.05, 0.1, 0.15]

**Exit Criterion**:
```rust
// MultiObj must beat pure CE by ≥ 0.03 BPB
assert!(multiobj_bpb < ce_baseline_bpb - 0.03);
```

**Margin**: ≥ 0.03 BPB improvement over pure cross-entropy baseline.

**Owner**: ALFA (JEPA specialist)

---

### P5: Gate-2 Push — 3 Seeds < 1.85

**Hypothesis**: Running same model on 3 seeds (43, 44, 45) achieves < 1.85 BPB on all seeds at step ≥ 4000.

**What we implement**:
1. `configs/gate2-final.toml`:
   ```toml
   [training]
   seeds = [43, 44, 45]
   steps = 5000  # R8 floor
   eval_interval = 500  # Evaluate at 500, 1000, 1500, 2000, 2500, 3000, 3500, 4000, 4500, 5000

   [gate2]
   victory_threshold = 1.85  # Target BPB
   victory_step_floor = 4000  # R8 floor
   ```

2. `scripts/deploy_gate2.sh`:
   ```bash
   #!/bin/bash
   # Create 3 Railway services
   for seed in 43 44 45; do
       railway service create "trios-gate2-seed-$seed"
       railway variables set TRIOS_SEED=$seed --service "trios-gate2-seed-$seed"
       railway up --service "trios-gate2-seed-$seed"
   done

   # Monitor all 3 services
   while true; do
       echo "=== Checking gate status ==="
       check_all_seeds 43 44 45
       sleep 300  # Check every 5 minutes
   done
   ```

3. Gate-2 verdict logic in `src/ledger.rs`:
   ```rust
   pub fn evaluate_gate2(bpb: f32, step: usize) -> Gate2Status {
       if step < 4000 {
           return Gate2Status::BelowFloor;  // R8 violation
       }
       if bpb < 1.85 {
           return Gate2Status::Victory;  // Target met
       } else {
           return Gate2Status::Evidence;  // Still collecting evidence
       }
   }
   ```

**Exit Criterion**:
```rust
// ALL 3 seeds must achieve < 1.85 at step ≥ 4000
for seed in [43, 44, 45] {
    let bpb_at_4k = run_training(seed);
    assert!(bpb_at_4k < 1.85, "Seed {}: BPB {} >= 1.85", seed, bpb_at_4k);
}

// Gate-2 only triggers when ALL 3 seeds pass
assert!(gate2_status == Gate2Status::Victory);
```

**Margin**: ALL 3 seeds < 1.85 at step ≥ 4000.

**Owner**: ZETA (Gate-2 specialist)

---

## Lab vs Ledger Discipline

### Rule

**P1..P4** write ONLY to `assertions/lab/*.jsonl` (R7 triplet, R8 floor, no embargo).

**P0 and P5** write to `assertions/seed_results.jsonl` (full R7 triplet, R9 embargo, step ≥ 4000).

### Enforcement

```rust
// In ledger.rs:emit_row()
pub fn emit_row(path: &Path, row: &LedgerRow, embargo: &EmbargoBlock) -> Result<()> {
    let phase = get_current_training_phase();

    match phase {
        Phase::P0 | Phase::P5 => {
            // Full R7 triplet: BPB, step, seed, SHA, gate_status
            let full_triplet = Triplet::new(row.bpb, row.step, row.seed, row.sha.clone(), Some(row.gate_status.clone()));
            write_to_seed_results(full_triplet)?;
        }
        Phase::P1 | Phase::P2 | Phase::P3 | Phase::P4 => {
            // Lab-only triplet: BPB, step, seed, gate_status=null (no R9)
            let lab_triplet = Triplet::new(row.bpb, row.step, row.seed, row.sha.clone(), None);
            write_to_lab_results(lab_triplet)?;
        }
    }
}
```

### R9 Embargo Check

```rust
// Only P0 and P5 can emit with step < 4000
if step < 4000 && (phase == Phase::P0 || phase == Phase::P5) {
    return Err(anyhow!("R9 violation: step {} < 4000", step));
}
```

---

## Evidence Base (2025)

### IMU-1 (Muon −2.88% vs AdamW)
- Source: Internal trios-bus sweep
- Result: Muon (η²D=0.0235) achieved −2.88% BPB improvement over AdamW
- Cautious Weight Decay: −0.97% over Muon baseline (top of MLCommons 2024 AlgoPerf)

### IMU-2 (Meta Schedule-Free AdamW)
- Source: https://arxiv.org/abs/2402.10554
- Result: SF/WSD schedule defeats cosine φ-schedule on image tasks
- Relevance: Same principle applies to language modeling

### IMU-3 (Cerebras µP)
- Source: https://arxiv.org/abs/2506.02473
- Result: µP-DiT achieves 2.9× faster training with 70M → 8M transfer
- Relevance: Similar to our P2 hypothesis (parameter transfer)

### IMU-4 (µP-DiT DiT-XL)
- Source: https://arxiv.org/abs/2506.02473
- Result: DiT-XL achieves µP-DiT with even stronger performance
- Relevance: Shows parameter transfer works

---

## R5-Honest Status

| Phase | Status | Falsified | Notes |
|-------|--------|-----------|--------|
| P0 | 🟡 In Progress | — | Need audit tests + lock file |
| P1 | ⬜ Pending | — | Muon implementation ready |
| P2 | ⬜ Pending | — | MuP requires matrix ops |
| P3 | ⬜ Pending | — | Schedule-Free straightforward |
| P4 | ⬜ Pending | — | JEPA from trios-igla-trainer |
| P5 | ⬜ Pending | — | Deployment script |

**PR Open**: https://github.com/gHashTag/trios-trainer-igla/pull/21

**CI Status**: https://github.com/gHashTag/trios-trainer-igla/actions

---

## Anchor

φ² + φ⁻² = 3 — Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
