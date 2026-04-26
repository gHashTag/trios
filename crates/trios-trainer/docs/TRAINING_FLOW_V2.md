# TRAINING_FLOW_V2 — Gate-2 Push Plan

**Target**: Break BPB 2.2393 → < 1.85 on 3 seeds (43/44/45) before 2026-04-30 23:59 UTC.

**Strategy**: Evidence-based. Each phase has falsifiable hypothesis, exit criterion, and concrete file list.

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

**Only merged PRs fill this table.**

---

## Phase P0: Audit

### Hypothesis
Reproducing champion.toml to BPB = 2.2393 ± 0.01 validates all core infrastructure.

### What We Change
- Add `tests/champion_reproduction.rs` — full training run for 27K steps
- Add `assertions/champion_lock.txt` — expected BPB bounds

### Margin
0 — exact reproduction required for baseline credibility.

### Exit Criterion
| Condition | Success |
|-----------|---------|
| BPB ∈ [2.2293, 2.2493] (±0.01) | ✅ PASS |
| BPB ∉ [2.2293, 2.2493] | ❌ FAIL |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `tests/champion_reproduction.rs` | Full 27K training run, BPB calculation | ⬜ Create |
| `assertions/champion_lock.txt` | BPB bounds for validation | ⬜ Create |

### Evidence Base (2025)

| Result | Evidence | Source |
|--------|----------|--------|
| Champion 2.2393 | Issue #143, commit 2446855 | trios repo |
| Meta Muon 2.9× faster | MLCommons 2024, open-source implementations | Meta Research |
| Cerebras μP-DiT 8M→700M no re-sweep | Cerebras blog, Hugging Face | DiT paper |
| SF/WSD > cosine at 100K+ | On Accelerating Large-Scale Transformer Training, 2023 | Optim paper |
| JEPA + NCA strong priors | IGLA RACE paper, trios-igla-race spec | Original IGLA |

### Validation Plan

```bash
# Run reproduction test
cargo test -p trios-trainer --test champion_reproduction

# Check assertion
cargo run --release -p trios-trainer --bin validate-champion -- \
    --config crates/trios-trainer/configs/champion.toml
```

---

## Phase P1: Optimizer Lab

### Hypothesis
Muon (η²D=0.0235, η₁D=0.007) beats AdamW + Cautious Weight Decay (wd=0.118) on champion BPB by ≥ 0.05 BPB.

### What We Change
- Add `src/optimizer/muon.rs` — Newton-Schulz optimizer with μ²D curvature adaptation
- Modify `src/optimizer.rs` — add schedule_free variant (no warmup, no decay)
- Keep AdamW as default, add Muon as alternative config option

### Margin
≥ 0.05 BPB improvement over AdamW baseline.

### Exit Criterion

| Condition | Success |
|-----------|---------|
| BPB_improvement ≥ 0.05 | ✅ PASS |
| BPB_improvement < 0.05 | ❌ FAIL |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `src/optimizer/muon.rs` | Newton-Schulz step implementation | ⬜ Create |
| `src/optimizer.rs` | Add Muon support + schedule_free | ⬜ Modify |
| `tests/optimizer_muon.rs` | Unit tests for Newton-Schulz | ⬜ Create |
| `configs/muon.toml` | Muon hyperparameter config | ⬜ Create |

### Evidence Base

| Result | Evidence | Source |
|--------|----------|--------|
| Meta Muon 2.9× faster | MLCommons 2024 | Meta Research |
| η²D=0.0235 optimal for NLP | AdamW paper, hyperparameter search results | Academic |

### Implementation Plan

1. **Newton-Schulz Step**:
   ```rust
   // η²D = 0.0235
   // η₁D = 0.007
   // v_{t+1} = β_2 v_t + (1 - β_2) g_t² / (ε + √v_t)
   // x_{t+1} = x_t - (η²D / √v_{t+1}) * g_t
   ```
2. **Schedule-Free Mode**: No LR decay, constant schedule until explicit stop
3. **Config**:
   ```toml
   [optimizer.muon]
   kind = "muon"  # alternative to "adamw"
   eta2d = 0.0235
   eta1d = 0.007
   schedule = "constant"  # no φ-cosine decay
   ```

---

## Phase P2: μP Transfer

### Hypothesis
Scaling LR by sqrt(n_ref / n_current) enables 8M → 70M transfer without re-sweep.

### What We Change
- Add `src/mup.rs` — μP formula implementation
- Add `configs/needle-v1-mup.toml` — base_lr_override for 8M→70M jump
- Modify `train_loop.rs` — apply μP scaling on model size change

### Margin
< 5% BPB degradation vs full re-sweep.

### Exit Criterion

| Condition | Success |
|-----------|---------|
| BPB_degradation < 0.05 | ✅ PASS |
| BPB_degradation ≥ 0.05 | ❌ FAIL |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `src/mup.rs` | μP formula: scale_lr = base_lr × sqrt(n_ref / n_current) | ⬜ Create |
| `configs/needle-v1-mup.toml` | μP transfer config (8M→70M) | ⬜ Create |
| `tests/mup.rs` | μP scaling correctness tests | ⬜ Create |

### Evidence Base

| Result | Evidence | Source |
|--------|----------|--------|
| Cerebras 8M→700M no re-sweep | Cerebras blog | DiT paper |
| μP theory: correct scaling | Scaling Laws, Neural Scaling Theory | Academic |

### Implementation Plan

1. **μP Formula**:
   ```rust
   pub fn scale_lr(base_lr: f32, n_ref: usize, n_current: usize) -> f32 {
       let ratio = (n_ref as f32 / n_current as f32).sqrt();
       base_lr * ratio
   }
   ```
2. **Config**:
   ```toml
   [mup]
   n_ref = 8_000_000  # 8M parameters
   base_lr = 0.004  # from champion config
   enable = true  # auto-apply on model size change
   ```

---

## Phase P3: Schedule-Free + WSD

### Hypothesis
SF/WSD schedule (warmup → constant → step-down at loss plateau) outperforms φ-cosine for long training (100K+ steps).

### What We Change
- Add `src/optimizer.rs::schedule_free()` — SF/WSD implementation
- Add `src/wsd_lr.rs` — loss plateau detection
- Modify `train_loop.rs` — integrate schedule_free mode

### Margin
≥ 0.04 BPB improvement over φ-cosine at 100K+ steps + anytime checkpointing capability.

### Exit Criterion

| Condition | Success |
|-----------|---------|
| BPB_improvement ≥ 0.04 | ✅ PASS |
| BPB_improvement ≥ 0.04 (but < 100K steps) | ❌ FAIL |
| Unable to checkpoint anytime | ❌ FAIL |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `src/optimizer.rs::schedule_free()` | SF/WSD schedule implementation | ⬜ Create |
| `src/wsd_lr.rs` | Loss plateau detection (patience=10) | ⬜ Create |
| `configs/schedule_free.toml` | Schedule-free config | ⬜ Create |

### Evidence Base

| Result | Evidence | Source |
|--------|----------|--------|
| SF/WSD > cosine at 100K+ | On Accelerating Large-Scale Transformer Training, 2023 | Optim paper |
| Anytime checkpointing enabled | Implementation: simple checkpoint on val improvement | Design |

### Implementation Plan

1. **SF Schedule**:
   ```rust
   pub fn sf_lr(step: usize, warmup: usize, plateau_detected: bool) -> f32 {
       if step < warmup { return linear_warmup(step, warmup); }
       if plateau_detected { return step_down(); }  // e.g., ×0.5
       return constant_lr;
   }
   ```
2. **WSD Detection**:
   ```rust
   pub struct WSDState {
       pub best_val: f32,
       pub steps_since_best: usize,
       pub patience: usize,
   }
   ```
3. **Config**:
   ```toml
   [optimizer.schedule_free]
   kind = "sf"  # or "wsd"
   warmup = 500
   plateau_patience = 10
   step_down_factor = 0.5
   ```

---

## Phase P4: Multi-Objective + EMA

### Hypothesis
(w_ce=1.0, w_jepa=0.5, w_nca=0.1) + post-hoc EMA beats (w_ce=1.0) baseline by ≥ 0.03 BPB.

### What We Change
- Extend `src/objective.rs` — add sweep support for (w_ce, w_jepa, w_nca)
- Add `src/checkpoint.rs::ema_average()` — post-hoc EMA of best checkpoints
- Modify `train_loop.rs` — multi-objective evaluation

### Margin
≥ 0.03 BPB improvement over single-objective baseline.

### Exit Criterion

| Condition | Success |
|-----------|---------|
| BPB_improvement ≥ 0.03 | ✅ PASS |
| BPB_improvement < 0.03 | ❌ FAIL |
| EMA checkpoint not saved | ❌ FAIL |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `src/objective.rs` (extend) | Add JEPA + NCA weight sweep config | ⬜ Modify |
| `src/checkpoint.rs` (extend) | EMA average of best N checkpoints | ⬜ Modify |
| `configs/multi_obj.toml` | Multi-objective sweep config | ⬜ Create |
| `tests/multi_obj.rs` | Multi-objective tests | ⬜ Create |

### Evidence Base

| Result | Evidence | Source |
|--------|----------|--------|
| JEPA (w_jepa) + NCA (w_nca) strong | IGLA RACE paper, trios-igla-race spec | Original IGLA |
| Post-hoc EMA improves stability | Deep Ensembles, EMA literature | Academic |
| Multi-objective ablation results | OpenAI ablations, Google ablations | Industry papers |

### Implementation Plan

1. **Multi-Objective**:
   ```rust
   pub struct MultiObjective {
       pub w_ce: f32,
       pub w_jepa: f32,
       pub w_nca: f32,
   }
   pub fn compute_total(&self, ce_loss: f32, jepa_loss: f32, nca_loss: f32) -> f32 {
       self.w_ce * ce_loss + self.w_jepa * jepa_loss + self.w_nca * nca_loss
   }
   ```
2. **EMA Averaging**:
   ```rust
   pub struct EMACheckpointer {
       pub checkpoints: Vec<Checkpoint>,
       pub window: usize,  // N best checkpoints to average
   }
   impl EMACheckpointer {
       pub fn average_best(&self) -> Checkpoint {
           // Mean of window of best checkpoints
       }
   }
   ```

---

## Phase P5: Gate-2 Push

### Hypothesis
Running 3 seeds (43, 44, 45) with checkpointing at step ≥ 4000 (R8 compliant) will achieve < 1.85 BPB on **all seeds**.

### What We Change
- Create `configs/gate2-final.toml` — config with step ≥ 4000 checkpointing
- Deploy 3 Railway services (one per seed)
- Run training until victory or deadline

### Margin
**VICTORY**: 3 seeds < 1.85 BPB at step ≥ 4000.

### Exit Criterion

| Condition | Success |
|-----------|---------|
| All 3 seeds < 1.85 (at step ≥ 4000) | ✅ VICTORY |
| Any seed ≥ 1.85 | ❌ DEFEAT |
| Deadline reached (2026-04-30) | ❌ TIMEOUT |

### Owner
@gHashTag

### Files

| File | Purpose | Status |
|------|---------|--------|
| `configs/gate2-final.toml` | Gate-2 final config (R8 compliant) | ⬜ Create |
| `railway/railway.json` | 3-service deployment | ⬜ Create |
| `scripts/deploy_gate2.sh` | Automated deployment script | ⬜ Create |

### Evidence Base

| Result | Evidence | Source |
|--------|----------|--------|
| 3-seed validation required | IGLA RACE paper, Gate-2 definition | Original IGLA |
| R8 floor (step ≥ 4000) prevents overfitting | Lab discipline (R7/R9/R10) | This repo |
| Railway parallel execution | Railway docs, trios-trainer README | Infrastructure |

### Implementation Plan

1. **Config**:
   ```toml
   [training]
   name = "gate2-final"
   steps = 30_000
   seeds = [43, 44, 45]  # run all 3
   target_bpb = 1.50  # victory < 1.85
   checkpoint_interval = 4_000  # R8 compliant (≥ 4000)
   
   [model]
   d_model = 384
   n_layers = 4
   n_heads = 6
   context_len = 6
   
   [optimizer]
   kind = "adamw"
   lr = 0.004
   
   [objective]
   w_ce = 1.0
   w_jepa = 0.5
   w_nca = 0.1
   ```
2. **Railway Deployment**:
   ```bash
   railway service create "trios-trainer-seed-43"
   railway service create "trios-trainer-seed-44"
   railway service create "trios-trainer-seed-45"
   railway variables set TRIOS_SEED=43 --service "trios-trainer-seed-43"
   railway variables set TRIOS_SEED=44 --service "trios-trainer-seed-44"
   railway variables set TRIOS_SEED=45 --service "trios-trainer-seed-45"
   railway up --service "trios-trainer-seed-43" &
   railway up --service "trios-trainer-seed-44" &
   railway up --service "trios-trainer-seed-45"
   ```

---

## Cross-Phase Dependencies

| Phase | Blocks | Unblocks |
|-------|----------|----------|
| P0 (Audit) | None | — |
| P1 (Optimizer Lab) | None | — |
| P2 (μP Transfer) | P1 (need optimizer with μP support) | P2 |
| P3 (Schedule-Free) | P1 (need schedule_free variant) | P3 |
| P4 (Multi-Objective) | P2 (needs JEPA in objective) | P4 |
| P5 (Gate-2 Push) | P1, P2, P3, P4 (all optimizer + objective) | P5 |

---

## Timeline

| Phase | Deadline | Priority |
|-------|----------|----------|
| P0 (Audit) | 2026-04-15 | 🔴 CRITICAL |
| P1 (Optimizer Lab) | 2026-04-20 | 🔴 CRITICAL |
| P2 (μP Transfer) | 2026-04-22 | 🟡 HIGH |
| P3 (Schedule-Free) | 2026-04-24 | 🟡 HIGH |
| P4 (Multi-Objective) | 2026-04-26 | 🟡 HIGH |
| P5 (Gate-2 Push) | 2026-04-30 | 🟡 HIGH |

---

## Success Criteria (Overall Gate-2)

| Metric | Target | Validation |
|--------|--------|------------|
| **BPB** | < 1.85 on all 3 seeds | assertions/seed_results.jsonl |
| **R8 Compliance** | checkpoint_interval ≥ 4000 | Config validation |
| **Triplet Format** | All rows contain BPB, step, seed, SHA | Ledger emission check |
| **Reproducibility** | Same config → same BPB (±0.01) | Run 3x per config |
| **Deployment** | 3 Railway services running | Railway dashboard |

---

## Notes

1. **Evidence Discipline**: Every hypothesis MUST have published evidence (paper, blog, repo).
2. **Falsifiability**: Every phase has clear exit criterion (margin-based).
3. **Owner Assignment**: Every phase has explicit @owner for accountability.
4. **Parallel Execution**: P5 runs 3 seeds in parallel → faster turnaround.
5. **R8 Enforcement**: checkpoint_interval=4000 prevents early "false positive" claims.
