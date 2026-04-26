# IGLA Training Flow — Detailed Decomposition Plan

## Executive Summary

This plan decomposes the IGLA training pipeline into 5 tracks with 23 actionable tasks. The goal is to achieve **Gate-2 victory** (BPB ≤ 1.50) and **Champion reproduction** (BPB ≤ 2.24 @ 27K).

---

## Track 1: Core Training Loop (Priority: CRITICAL)

### T1.1 Integrate Forward/Backward Pass
**File**: `src/train_loop.rs`

**Current State**: Mock `evaluate_step()` returns dummy BPB

**Required Changes**:
```rust
// BEFORE: Mock evaluation
fn evaluate_step(step: usize, seed: u64) -> Result<f32> {
    let base_bpb = 3.0;
    let progress = (step as f32) / 27000.0;
    Ok(base_bpb - (progress * 0.5) + noise)
}

// AFTER: Real forward pass
pub struct TrainingState {
    pub model: MinimalTransformer,
    pub optimizer: AdamWCpu,
    pub step: usize,
    pub best_bpb: f32,
}

fn training_step(state: &mut TrainingState, batch: &Batch) -> Result<f32> {
    // 1. Forward pass
    let logits = state.model.forward(&batch.tokens)?;

    // 2. Compute loss
    let loss = cross_entropy_loss(&logits, &batch.targets);

    // 3. Backward pass
    let mut grads = vec![0.0f32; state.model.param_count()];
    backward_pass(&state.model, &logits, &batch.targets, &mut grads);

    // 4. Optimizer step
    state.optimizer.step(&mut state.model.parameters, &grads);

    Ok(loss)
}
```

**Dependencies**:
- `forward.rs` (✅ exists)
- `backward.rs` (✅ exists)
- `model.rs` (✅ exists)

**Estimated Effort**: 4 hours

---

### T1.2 Wire Up Optimizer with LR Schedule
**File**: `src/train_loop.rs`

**Required Changes**:
```rust
use crate::optimizer::{AdamWCpu, phi_lr_schedule, lr_schedule_54_f64};
use trios_phi_schedule::LrScheduleType;

pub struct TrainingConfig {
    pub base_lr: f64,
    pub warmup_steps: usize,
    pub schedule_type: LrScheduleType,
}

impl TrainingState {
    pub fn new(cfg: &Config) -> Self {
        let model = MinimalTransformer::new(...);
        let mut optimizer = AdamWCpu::with_phi_defaults(model.param_count());
        optimizer.lr = cfg.training.lr as f64;

        Self { model, optimizer, step: 0, best_bpb: f32::MAX }
    }

    pub fn get_lr(&self, max_steps: usize) -> f64 {
        lr_schedule_54_f64(
            self.schedule_type,
            self.step,
            max_steps
        )
    }
}
```

**Dependencies**:
- `optimizer.rs` (✅ exists)
- `trios-phi-schedule` crate (⚠️ needs dependency)

**Estimated Effort**: 2 hours

---

### T1.3 Batch Sampling & Data Pipeline
**File**: `src/train_loop.rs`

**Required Changes**:
```rust
pub struct Batch {
    pub tokens: Vec<usize>,      // [batch_size * seq_len]
    pub targets: Vec<usize>,     // [batch_size * seq_len]
}

pub struct DataLoader {
    dataset: FineWebDataset,
    seq_len: usize,
    batch_size: usize,
    rng_state: u64,
}

impl DataLoader {
    pub fn next_batch(&mut self) -> Batch {
        let mut tokens = Vec::with_capacity(self.batch_size * (self.seq_len + 1));
        for _ in 0..self.batch_size {
            let seq = self.dataset.sample_sequence(self.seq_len + 1, &mut self.rng_state);
            tokens.extend(seq);
        }

        // Shift for next-token prediction
        let inputs: Vec<usize> = tokens.iter()
            .step_by(self.seq_len + 1)
            .take(self.batch_size * self.seq_len)
            .copied()
            .collect();

        let targets: Vec<usize> = tokens.iter()
            .skip(1)
            .step_by(self.seq_len + 1)
            .take(self.batch_size * self.seq_len)
            .copied()
            .collect();

        Batch { tokens: inputs, targets }
    }
}
```

**Estimated Effort**: 2 hours

---

### T1.4 Evaluation Loop
**File**: `src/train_loop.rs`

**Required Changes**:
```rust
pub fn evaluate(model: &MinimalTransformer, val_data: &FineWebDataset) -> f32 {
    const EVAL_BATCHES: usize = 10;
    const EVAL_TOKENS: usize = 10_000;

    let mut total_loss = 0.0f32;
    let mut total_tokens = 0usize;

    for _ in 0..EVAL_BATCHES {
        let tokens = val_data.get_eval_batch(EVAL_TOKENS);
        let inputs: Vec<_> = tokens[..EVAL_TOKENS-1].iter().map(|&t| t as usize).collect();
        let targets: Vec<_> = tokens[1..EVAL_TOKENS].iter().map(|&t| t as usize).collect();

        let logits = model.forward(&inputs);
        let loss = cross_entropy_loss(
            &logits.concat(),
            &targets
        );

        total_loss += loss * targets.len() as f32;
        total_tokens += targets.len();
    }

    // Convert loss to BPB: BPB = loss / ln(2)
    total_loss / total_tokens as f32 / std::f32::consts::LN_2
}
```

**Estimated Effort**: 2 hours

---

## Track 2: Model Architecture Refinement (Priority: HIGH)

### T2.1 Model Parameter Access
**File**: `src/model.rs`

**Required Changes**:
```rust
impl MinimalTransformer {
    // Add parameter access for optimizer
    pub fn parameters(&self) -> Vec<f32> {
        let mut params = Vec::new();
        params.extend(self.token_embedding.clone());
        params.extend(self.pos_embedding.clone());
        for layer in &self.layers {
            params.extend(layer.attention.w_q.clone());
            params.extend(layer.attention.w_k.clone());
            params.extend(layer.attention.w_v.clone());
            params.extend(layer.attention.w_o.clone());
            params.extend(layer.ffn.w1.clone());
            params.extend(layer.ffn.w2.clone());
        }
        params.extend(self.lm_head.clone());
        params
    }

    pub fn param_count(&self) -> usize {
        self.parameters().len()
    }
}
```

**Estimated Effort**: 1 hour

---

### T2.2 Gradient Accumulation Support
**File**: `src/train_loop.rs`

**Required Changes**:
```rust
pub struct TrainingConfig {
    pub accum_steps: usize,  // Gradient accumulation
}

pub fn train_step_with_accum(
    state: &mut TrainingState,
    loader: &mut DataLoader,
    cfg: &TrainingConfig,
) -> Result<f32> {
    let mut accum_grads = vec![0.0f32; state.model.param_count()];
    let mut total_loss = 0.0f32;

    for _ in 0..cfg.accum_steps {
        let batch = loader.next_batch();
        let loss = training_step_with_grad_buffer(
            state,
            &batch,
            &mut accum_grads
        )?;
        total_loss += loss;
    }

    // Average gradients and apply
    for g in accum_grads.iter_mut() {
        *g /= cfg.accum_steps as f32;
    }
    state.optimizer.step(&mut state.model.parameters, &accum_grads);

    Ok(total_loss / cfg.accum_steps as f32)
}
```

**Estimated Effort**: 2 hours

---

### T2.3 Tied Embeddings Option
**File**: `src/model.rs`

**Rationale**: Issue #67 showed LR=0.1 is correct for tied embeddings

**Required Changes**:
```rust
pub struct MinimalTransformer {
    // ... existing fields ...
    pub tied_embeddings: bool,
}

impl MinimalTransformer {
    pub fn with_tied_embeddings(mut self, tied: bool) -> Self {
        self.tied_embeddings = tied;
        if tied {
            // Use token embedding as LM head
            self.lm_head = vec![];  // Will reference token_emb
        }
        self
    }

    pub fn forward(&self, tokens: &[usize]) -> Vec<Vec<f32>> {
        // ... existing code ...
        let logits = if self.tied_embeddings {
            // Compute logits as token_emb @ x
            self.compute_logits_tied(&x)
        } else {
            // Use lm_head matrix
            self.compute_logits_full(&x)
        };
        logits
    }
}
```

**Estimated Effort**: 3 hours

---

## Track 3: JEPA & NCA Integration (Priority: MEDIUM)

### T3.1 JEPA Objective Module
**File**: `src/jepa.rs` (NEW)

**Required Changes**:
```rust
//! T-JEPA (Temporal Joint Embedding Predictive Architecture)
//!
//! Predicts future embeddings from current context using embedding space alignment.

use crate::model::MinimalTransformer;

pub struct JepaConfig {
    pub mask_ratio: f32,      // Token masking ratio (0.0-1.0)
    pub ema_decay: f32,       // EMA decay for target encoder
}

pub struct JepaObjective {
    pub config: JepaConfig,
    pub target_encoder: MinimalTransformer,  // EMA'd target
}

impl JepaObjective {
    pub fn compute_loss(
        &self,
        predictions: &[f32],
        targets: &[f32],
    ) -> f32 {
        // Cosine similarity loss in embedding space
        let mut loss = 0.0f32;
        let n = predictions.len() / targets.len();

        for i in 0..n {
            let pred = &predictions[i * 384..(i + 1) * 384];
            let target = &targets[i * 384..(i + 1) * 384];

            // Cosine similarity
            let dot: f32 = pred.iter().zip(target.iter()).map(|(p, t)| p * t).sum();
            let pred_norm: f32 = pred.iter().map(|p| p * p).sum::<f32>().sqrt();
            let target_norm: f32 = target.iter().map(|t| t * t).sum::<f32>().sqrt();

            let cosine = dot / (pred_norm * target_norm + 1e-8);
            loss -= cosine;  // Maximize similarity = minimize negative
        }

        loss / n as f32
    }

    pub fn update_target_encoder(&mut self, online: &MinimalTransformer) {
        // EMA update: target = decay * target + (1 - decay) * online
        let online_params = online.parameters();
        let target_params = self.target_encoder.parameters();

        for (t, o) in target_params.iter_mut().zip(online_params.iter()) {
            *t = self.config.ema_decay * *t + (1.0 - self.config.ema_decay) * o;
        }
    }
}
```

**Estimated Effort**: 4 hours

---

### T3.2 NCA (Neural Collapse Auxiliary)
**File**: `src/nca.rs` (NEW)

**Required Changes**:
```rust
//! NCA (Neural Collapse Auxiliary) Objective
//!
//! Encourages class embeddings to converge to a simplex equiangular tight frame.

pub struct NcaObjective {
    pub num_classes: usize,
    pub target_norm: f32,  // Target norm for class embeddings
}

impl NcaObjective {
    pub fn compute_loss(&self, embeddings: &[f32], targets: &[usize]) -> f32 {
        // Compute class means
        let mut class_means = vec![vec![0.0f32; embeddings.len() / targets.len()]; self.num_classes];
        let mut class_counts = vec![0usize; self.num_classes];

        for (emb, &t) in embeddings.chunks(384).zip(targets.iter()) {
            for (i, &e) in emb.iter().enumerate() {
                class_means[t][i] += e;
            }
            class_counts[t] += 1;
        }

        // Normalize class means
        for (mean, &count) in class_means.iter_mut().zip(class_counts.iter()) {
            if count > 0 {
                for m in mean.iter_mut() {
                    *m /= count as f32;
                }
            }
        }

        // Penalize deviation from simplex ETF
        // ETF: class_means are equiangular (60° apart) and equal norm
        let mut loss = 0.0f32;
        for i in 0..self.num_classes {
            for j in (i + 1)..self.num_classes {
                let dot: f32 = class_means[i].iter()
                    .zip(class_means[j].iter())
                    .map(|(a, b)| a * b)
                    .sum();

                // Target: dot = -1 / (num_classes - 1) for ETF
                let target = -1.0 / (self.num_classes - 1) as f32;
                loss += (dot - target).powi(2);
            }
        }

        loss
    }
}
```

**Estimated Effort**: 3 hours

---

### T3.3 Multi-Objective Training
**File**: `src/train_loop.rs`

**Required Changes**:
```rust
pub struct MultiObjectiveConfig {
    pub w_ce: f32,    // Cross-entropy weight
    pub w_jepa: f32,  // JEPA weight
    pub w_nca: f32,   // NCA weight
}

pub fn compute_multi_loss(
    ce_loss: f32,
    jepa_loss: Option<f32>,
    nca_loss: Option<f32>,
    cfg: &MultiObjectiveConfig,
) -> f32 {
    let mut total = cfg.w_ce * ce_loss;

    if let Some(jl) = jepa_loss {
        total += cfg.w_jepa * jl;
    }

    if let Some(nl) = nca_loss {
        total += cfg.w_nca * nl;
    }

    total
}
```

**Estimated Effort**: 2 hours

---

## Track 4: Infrastructure & Tooling (Priority: MEDIUM)

### T4.1 Checkpoint Management
**File**: `src/checkpoint.rs` (NEW)

**Required Changes**:
```rust
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub step: usize,
    pub model_params: Vec<f32>,
    pub optimizer_state: OptimizerState,
    pub best_bpb: f32,
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerState {
    pub step: usize,
    pub m: Vec<f64>,
    pub v: Vec<f64>,
}

pub fn save_checkpoint(
    model: &MinimalTransformer,
    optimizer: &AdamWCpu,
    best_bpb: f32,
    step: usize,
    path: &Path,
) -> anyhow::Result<()> {
    let checkpoint = Checkpoint {
        step,
        model_params: model.parameters(),
        optimizer_state: OptimizerState {
            step: optimizer.step_count(),
            m: optimizer.m.clone(),
            v: optimizer.v.clone(),
        },
        best_bpb,
        config_hash: "TODO".to_string(),
    };

    let bytes = bincode::serialize(&checkpoint)?;
    std::fs::write(path, bytes)?;

    Ok(())
}

pub fn load_checkpoint(path: &Path) -> anyhow::Result<Checkpoint> {
    let bytes = std::fs::read(path)?;
    Ok(bincode::deserialize(&bytes)?)
}
```

**Estimated Effort**: 2 hours

---

### T4.2 Metrics Logging
**File**: `src/metrics.rs` (NEW)

**Required Changes**:
```rust
pub struct MetricsLogger {
    pub log_path: PathBuf,
    pub events: Vec<MetricsEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsEvent {
    pub step: usize,
    pub timestamp: String,
    pub train_loss: f32,
    pub val_bpb: f32,
    pub lr: f64,
    pub throughput_tokens_per_sec: f32,
}

impl MetricsLogger {
    pub fn log(&mut self, event: MetricsEvent) {
        self.events.push(event.clone());
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
        {
            use std::io::Write;
            writeln!(file, "{}", serde_json::to_string(&event).unwrap()).ok();
        }
    }
}
```

**Estimated Effort**: 1 hour

---

### T4.3 CLI Improvements
**File**: `src/bin/trios-train.rs`

**Required Changes**:
```rust
#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    config: PathBuf,

    #[arg(long)]
    seed: Option<u64>,

    #[arg(long)]
    steps: Option<usize>,

    #[arg(long)]
    resume_from: Option<PathBuf>,  // NEW: Resume from checkpoint

    #[arg(long)]
    checkpoint_dir: Option<PathBuf>,  // NEW: Checkpoint directory

    #[arg(long)]
    dry_run: bool,

    #[arg(long)]
    verbose: bool,  // NEW: Verbose logging
}
```

**Estimated Effort**: 1 hour

---

## Track 5: Validation & Testing (Priority: HIGH)

### T5.1 Unit Tests for Training Loop
**File**: `src/train_loop.rs` (tests module)

**Required Changes**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_training_step_reduces_loss() {
        let mut state = setup_test_state();
        let batch = dummy_batch();

        let initial_loss = training_step(&mut state, &batch).unwrap();
        let final_loss = training_step(&mut state, &batch).unwrap();

        assert!(final_loss < initial_loss, "Training should reduce loss");
    }

    #[test]
    fn test_lr_schedule_monotonic() {
        for step in 0..100 {
            let lr = phi_lr_schedule(step, 0.01, 10);
            if step > 10 {
                let prev_lr = phi_lr_schedule(step - 1, 0.01, 10);
                assert!(lr <= prev_lr, "LR should decay after warmup");
            }
        }
    }

    #[test]
    fn test_checkpoint_roundtrip() {
        let model = MinimalTransformer::new(16, 64, 256, 4, 1);
        let optimizer = AdamWCpu::with_phi_defaults(model.param_count());

        let path = PathBuf::from("/tmp/test_checkpoint.bin");
        save_checkpoint(&model, &optimizer, 2.5, 100, &path).unwrap();

        let loaded = load_checkpoint(&path).unwrap();
        assert_eq!(loaded.step, 100);
        assert_eq!(loaded.best_bpb, 2.5);
    }
}
```

**Estimated Effort**: 3 hours

---

### T5.2 Integration Test: Champion Config
**File**: `tests/champion_reproduction.rs` (NEW)

**Required Changes**:
```rust
#[test]
fn test_champion_config_trains() {
    let config = Config::load("crates/trios-trainer/configs/champion.toml").unwrap();

    // Run 100 steps
    let result = run(&config);

    assert!(result.final_bpb.is_finite());
    assert!(result.best_bpb < 10.0);  // Should converge from random
}

#[test]
fn test_inv8_lr_validation() {
    // Valid LR
    assert!(validate_lr_phi_band(0.004));

    // Invalid LR
    assert!(!validate_lr_phi_band(0.02));
    assert!(!validate_lr_phi_band(0.0001));
}
```

**Estimated Effort**: 2 hours

---

### T5.3 Gate-2 Benchmark Test
**File**: `tests/gate2_benchmark.rs` (NEW)

**Required Changes**:
```rust
#[test]
#[ignore]  // Run only when explicitly requested
fn test_gate2_victory() {
    let config = Config::load("crates/trios-trainer/configs/gate2-attempt.toml").unwrap();

    let result = run(&config);

    // Gate-2 requirement: BPB ≤ 1.50
    assert!(
        result.best_bpb <= 1.50,
        "Gate-2 failed: BPB {} > 1.50",
        result.best_bpb
    );
}
```

**Estimated Effort**: 1 hour

---

## Execution Order

### Week 1: Core Training Loop
- Day 1-2: T1.1 (Integrate Forward/Backward)
- Day 3: T1.2 (Wire Up Optimizer)
- Day 4: T1.3 (Batch Sampling)
- Day 5: T1.4 (Evaluation Loop)

### Week 2: Model & Validation
- Day 1: T2.1 (Parameter Access)
- Day 2: T5.1 (Unit Tests)
- Day 3-4: T2.2 (Gradient Accumulation)
- Day 5: T5.2 (Integration Tests)

### Week 3: JEPA & NCA
- Day 1-2: T3.1 (JEPA Objective)
- Day 3: T3.2 (NCA Objective)
- Day 4: T3.3 (Multi-Objective)
- Day 5: T2.3 (Tied Embeddings)

### Week 4: Infrastructure
- Day 1: T4.1 (Checkpoints)
- Day 2: T4.2 (Metrics)
- Day 3: T4.3 (CLI)
- Day 4: T5.3 (Gate-2 Test)
- Day 5: Buffer & Review

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Model doesn't converge | HIGH | Start with champion config (proven) |
| BPB plateau | MEDIUM | Try tied embeddings (Issue #67) |
| OOM on small GPU | MEDIUM | Gradient accumulation |
| Slow training | LOW | Profile + optimize hotspots |

---

## Success Criteria

- [x] PR-0: Skeleton compiles
- [x] PR-1: Model/Optimizer migrated
- [ ] PR-2: Real training works
- [ ] PR-3: Champion reproduced (BPB ≤ 2.24 @ 27K)
- [ ] Gate-2: BPB ≤ 1.50
- [ ] Gate-final: Production deployment

---

## Dependencies

| Crate | Purpose | Status |
|-------|---------|--------|
| `trios-phi-schedule` | LR schedules | ⚠️ Needs add |
| `bincode` | Checkpoint serialization | ⚠️ Needs add |
| `trios-igla-race` | Invariants (optional) | ✅ Listed |

**Add to `Cargo.toml`**:
```toml
[dependencies]
trios-phi-schedule = { path = "../trios-phi-schedule" }
bincode = "2.0"
```
