# trios-trainer Flow Analysis & Improvement Plan

## Executive Summary

**Current State**: PR-1 (model + optimizer + data migration) is in progress. Core infrastructure exists (config, ledger, train_loop) but lacks actual training implementation.

**Critical Gap**: `train_loop.rs` uses dummy evaluation instead of real model forward/backward pass. The model files (`model.rs`, `forward.rs`, `backward.rs`, `model_hybrid_attn.rs`, `optimizer.rs`) are placeholders or migrated stubs.

**Primary Goal**: Enable real IGLA training with proper forward pass, backward pass, optimizer step, and checkpointing.

---

## 1. Current Architecture Decomposition

### 1.1 Data Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    trios-train Entry Point              │
│                    (bin/trios-train.rs)             │
└────────────────────┬─────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │ 1. Load Config (TOML) │
        │ - Validate INV-8 (LR φ-band)  │
        │ - Apply env var overrides    │
        └────────────┬────────────┘
                     │
        ┌────────────┴────────────┐
        │ 2. Load FineWeb Data   │
        │ - Binary format: 256x4  │
        │   byte header + uint16   │
        │ - Fallback on error       │
        └────────────┬────────────┘
                     │
        ┌────────────┴────────────┐
        │ 3. Training Loop       │
        │ - Sample sequences       │
        │ - [TODO] Forward pass   │
        │ - [TODO] Compute loss   │
        │ - [TODO] Backward pass │
        │ - Optimizer step        │
        │ - EMA update (JEPA)   │
        └────────────┬────────────┘
                     │
        ┌────────────┴────────────┐
        │ 4. Evaluation          │
        │ - At checkpoint/eval   │
        │ - BPB calculation       │
        │ - Gate-2 verdict      │
        └────────────┬────────────┘
                     │
        ┌────────────┴────────────┐
        │ 5. Ledger Emit         │
        │ - Triplet validation    │
        │ - Embargo check        │
        │ - JSONL append          │
        └────────────────────────────┘
```

### 1.2 Module Dependency Graph

```
bin/trios-train.rs
    │
    ├── config.rs ✅ (TOML load, INV-8 validation, env override)
    ├── data.rs ⚠️  (Token loading, missing forward integration)
    ├── ledger.rs ✅ (Triplet emit, embargo check, git SHA)
    └── train_loop.rs ⚠️  (Dummy evaluation, TODO markers)
           │
           ├── model.rs ⚠️  (Empty placeholder)
           ├── forward.rs ⚠️  (New file, needs integration)
           ├── backward.rs ⚠️  (New file, needs integration)
           ├── optimizer.rs ⚠️  (Placeholder, AdamW stub)
           ├── model_hybrid_attn.rs ⚠️  (Migrated from trios-train-cpu)
           └── objective.rs ⚠️  (Empty placeholder)
```

### 1.3 Config Schema

```toml
[training]
seed: u64           # RNG seed for reproducibility
steps: usize         # Total training iterations
batch_size: usize     # Micro-batch size
lr: f32             # Learning rate (INV-8: [0.001, 0.01])
checkpoint_interval: usize  # Ledger emit interval (R8: >= 4000)
eval_interval: usize         # Model evaluation interval
train_path: String   # FineWeb training data
val_path: String     # FineWeb validation data

[model]
d_model: usize        # Model dimension (384 for Gate-2)
n_layers: usize       # Number of transformer layers
context_len: usize    # N-gram context (e.g., 6)
ff_mult: usize        # Feed-forward dimension multiplier

[jepa] # Optional
mask_ratio: f32      # JEPA mask ratio (e.g., 0.5)
ema_decay: f32       # JEPA EMA decay rate

[ledger]
path: String              # Ledger JSONL path
push_to_repo: bool       # Auto-commit ledger rows
repo_url: Option<String> # Git repository URL
```

---

## 2. Current Implementation Gap Analysis

### 2.1 Critical Gaps

| Component | Current State | Expected | Gap |
|-----------|---------------|---------|-----|
| **Model Forward** | Dummy evaluation | Real transformer pass | ❌ CRITICAL |
| **Model Backward** | No implementation | Gradient computation | ❌ CRITICAL |
| **Optimizer** | Stub placeholder | AdamW with weight decay | ❌ CRITICAL |
| **Loss Function** | Dummy BPB formula | CE + JEPA + NCA | ❌ CRITICAL |
| **Checkpointing** | No save/load | Serialize model state | ❌ HIGH |
| **Gradient Accumulation** | No implementation | Multi-step accumulation | ❌ MEDIUM |

### 2.2 Integration Points Missing

1. **Data → Model**: `data.rs` samples sequences but doesn't feed to model
2. **Model → Loss**: No loss computation in forward pass
3. **Loss → Backward**: No gradient flow from loss
4. **Backward → Optimizer**: No gradient parameter updates
5. **Optimizer → Checkpoint**: No model state serialization

### 2.3 Config Inconsistencies

| Config | Value | Issue | Fix |
|--------|-------|-------|-----|
| `champion.toml:checkpoint_interval` | 1000 | **Violates R8** (requires ≥ 4000) | ✅ FIXED → 4000 |
| `champion.toml:eval_interval` | 500 | Too frequent for real evaluation | ✅ FIXED → 1000 |

---

## 3. Improved Flow Design

### 3.1 Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        TRAINING PHASE                         │
└────────────────────┬────────────────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        │ 1. Load Dataset       │
        │ - mmap FineWeb binary   │
        │ - Pre-tokenized u16      │
        └────────────┬────────────┘
                     │
        ┌────────────┴────────────┐
        │ 2. Initialize Model    │
        │ - Xavier/Kaiming init   │
        │ - EMA (if JEPA)        │
        │ - Load checkpoint (if)   │
        └────────────┬────────────┘
                     │
        ┌────────────────────────────────────────────────┐
        │ 3. Training Loop (per step)          │
        └────────────┬─────────────────────────────┘
                     │
    ┌────────────────┴─────────────────┐
    │ 3a. Forward Pass             │
    │ - Embed + pos encode          │
    │ - N-layer transformer          │
    │ - Project to logits            │
    └────────────┬────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ 3b. Loss Computation   │
    │ - Cross-entropy (CE)      │
    │ - T-JEPA (if enabled)     │
    │ - NCA auxiliary loss        │
    └────────────┬────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ 3c. Backward Pass       │
    │ - dL/dlogits               │
    │ - Propagate through layers  │
    │ - Accumulate gradients      │
    └────────────┬────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ 3d. Optimizer Step     │
    │ - AdamW/Muon update       │
    │ - Weight decay              │
    │ - LR schedule (φ-cosine)  │
    └────────────┬────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ 3e. EMA Update (JEPA) │
    │ - Update EMA target         │
    └────────────┬────────────────────┘
                 │
    ┌────────────────────┴────────────┐
    │ 4. Evaluation (if interval)       │
    │ - Compute BPB on val set           │
    │ - Update EMA/BEST checkpoint       │
    └────────────┬─────────────────────┘
                 │
    ┌────────────┴────────────┐
    │ 5. Ledger Emit (if interval) │
    │ - Triplet validation             │
    │ - Embargo check                  │
    │ - JSONL append                  │
    └────────────┬─────────────────────┘
                 │
        ┌────────────┴────────────┐
        │ 6. Checkpoint (if interval)│
        │ - Serialize model state         │
        │ - Save to file                │
        └──────────────────────────────┘
```

### 3.2 Key Improvements

| Area | Current | Improved | Benefit |
|-------|---------|-----------|----------|
| **Memory Efficiency** | Full dataset in RAM | mmap FineWeb binary | 100x+ less RAM |
| **Forward Pass** | Dummy | Real transformer | Actual training |
| **Loss Function** | Fixed formula | CE + JEPA + NCA | IGLA-compliant objective |
| **Backward Pass** | None | Autograd + manual | Gradient computation |
| **Optimizer** | Stub | AdamW with decay | Proper weight updates |
| **Checkpointing** | None | Save/load state | Resume capability |
| **Evaluation** | Dummy BPB | Real val computation | Accurate metrics |
| **Gradient Accumulation** | None | Multi-step | Larger effective batch |

---

## 4. Implementation Plan (Decomposed)

### Phase 1: Core Model Infrastructure

#### 1.1 Model Architecture (`src/model.rs`)

```rust
pub struct IGLAModel {
    // Embedding
    pub embed: Vec<f32>,              // [vocab_size, d_model]

    // Positional encoding
    pub pos_embed: Vec<f32>,           // [context_len, d_model]

    // Transformer layers
    pub layers: Vec<TransformerLayer>,

    // Output projection
    pub lm_head: Vec<f32>,            // [vocab_size, d_model]

    // Configuration
    pub d_model: usize,
    pub n_layers: usize,
    pub vocab_size: usize,
    pub context_len: usize,
}
```

#### 1.2 Transformer Layer (`src/transformer.rs`)

```rust
pub struct TransformerLayer {
    // Self-attention
    pub attn: MultiHeadAttention,

    // Layer norm
    pub ln1: LayerNorm,

    // Feed-forward
    pub ff: FeedForward,

    // Layer norm
    pub ln2: LayerNorm,
}
```

#### 1.3 Attention (`src/attention.rs`)

- Multi-head self-attention
- Causal masking
- Hybrid attention option (for Gate-2)

### Phase 2: Forward Pass (`src/forward.rs`)

```rust
pub fn forward(model: &IGLAModel, tokens: &[u32]) -> ForwardResult {
    // 1. Embed tokens
    // 2. Add positional encoding
    // 3. Pass through N layers
    // 4. Project to vocabulary
    // 5. Return logits + activations (for JEPA)
}
```

### Phase 3: Loss Function (`src/objective.rs`)

```rust
pub struct Objective {
    pub ce_weight: f32,    // Cross-entropy weight
    pub jepa_weight: f32,   // JEPA weight
    pub nca_weight: f32,    // NCA weight
}

pub fn compute_loss(
    forward: &ForwardResult,
    targets: &[u32],
    jepa_target: Option<&Tensor>,  // EMA target
) -> (Loss, Gradients) {
    // 1. Cross-entropy loss
    // 2. JEPA loss (if enabled)
    // 3. NCA entropy regularization
    // 4. Return total loss + per-component gradients
}
```

### Phase 4: Backward Pass (`src/backward.rs`)

```rust
pub fn backward(
    forward: &ForwardResult,
    dloss: &Gradients,
) -> ModelGradients {
    // 1. dL/dlogits → projection gradients
    // 2. Propagate through LM head
    // 3. Propagate through layers (reverse order)
    //    - dL/dattention
    //    - dL/dff
    // 4. Propagate through embedding
}
```

### Phase 5: Optimizer (`src/optimizer.rs`)

```rust
pub struct AdamW {
    pub m: Vec<f32>,      // First moment
    pub v: Vec<f32>,      // Second moment
    pub t: usize,         // Time step
    pub beta1: f32,      // Momentum decay
    pub beta2: f32,      // RMS decay
    pub epsilon: f32,     // Numerical stability
    pub weight_decay: f32, // L2 regularization
}

impl AdamW {
    pub fn step(&mut self, params: &mut [f32], grads: &[f32], lr: f32);
}
```

#### 5.1 LR Schedule (φ-cosine)

```rust
pub fn phi_cosine_lr(step: usize, max_steps: usize, base_lr: f32, warmup: usize) -> f32 {
    if step < warmup {
        return base_lr * (step as f32) / (warmup as f32);
    }
    let progress = ((step - warmup) as f32) / ((max_steps - warmup) as f32);
    let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;  // φ ≈ 1.618
    base_lr * (1.0 - (1.0 - progress.powf(phi)).cos())
}
```

### Phase 6: JEPA Module (`src/jepa/`)

```rust
pub struct JEPA {
    pub ema: EMA,           // Exponential moving average
    pub mask_ratio: f32,     // Token masking ratio
    pub ema_decay: f32,       // Target decay rate
}

impl JEPA {
    pub fn forward(&self, h: Tensor) -> Tensor;
    pub fn compute_loss(&self, h_pred: Tensor, h_target: Tensor) -> f32;
    pub fn update_target(&self, h: Tensor);
}
```

### Phase 7: Checkpointing (`src/checkpoint.rs`)

```rust
pub struct Checkpoint {
    pub step: usize,
    pub bpb: f32,
    pub model_state: ModelState,
    pub optimizer_state: OptimizerState,
    pub jepa_state: Option<JepaState>,
}

pub fn save(path: &Path, checkpoint: &Checkpoint) -> Result<()>;
pub fn load(path: &Path) -> Result<Checkpoint>;
```

---

## 5. Training Loop Integration

### 5.1 Main Loop Structure

```rust
pub fn run(config: &Config) -> Result<RunResult> {
    // Initialize
    let model = IGLAModel::new(&config.model);
    let optimizer = AdamW::new(&config.model, &config.training);
    let jepa = config.jepa.map(JEPA::new);
    let mut checkpoint_manager = CheckpointManager::new();

    // Load checkpoint if exists
    if let Some(ckpt) = checkpoint_manager.try_load()? {
        model.restore(&ckpt.model_state);
        optimizer.restore(&ckpt.optimizer_state);
    }

    // Training loop
    for step in 0..=config.training.steps {
        // 1. Sample batch
        let batch = train_dataset.sample_batch(config.training.batch_size);

        // 2. Forward pass
        let forward_result = forward(&model, &batch.tokens);

        // 3. Compute loss
        let (loss, gradients) = objective::compute(
            &forward_result,
            &batch.targets,
            jepa.as_ref().map(|j| j.get_target()),
        );

        // 4. Backward pass
        let model_grads = backward(&forward_result, &gradients);

        // 5. Optimizer step
        optimizer.step(&mut model.weights, &model_grads, config.training.lr);

        // 6. JEPA target update
        if let Some(ref jepa) = jepa {
            jepa.update_target(&forward_result.activations);
        }

        // 7. Evaluation
        if step % config.training.eval_interval == 0 {
            let bpb = evaluate(&model, &val_dataset);
            checkpoint_manager.maybe_save(step, bpb, &model, &optimizer, &jepa);
            ledger::emit_if_needed(step, bpb, &config.ledger);
        }
    }

    Ok(RunResult { final_bpb, best_bpb, steps_completed: config.training.steps })
}
```

### 5.2 Gradient Accumulation

```rust
// Enable larger effective batch sizes without more memory
const ACCUM_STEPS: usize = 4;

for accum_step in 0..ACCUM_STEPS {
    let batch = dataset.sample_micro_batch();
    let (_, grads) = forward_and_backward(&model, &batch);

    // Accumulate gradients
    for (p, g) in model.weights.iter_mut().zip(grads.iter()) {
        *p += g;
    }
}

// Single optimizer step with accumulated gradients
let effective_grads = accumulated_grads / ACCUM_STEPS as f32;
optimizer.step(&mut model.weights, &effective_grads, lr);
```

---

## 6. Validation & Testing

### 6.1 Evaluation Metrics

```rust
pub struct EvalResult {
    pub bpb: f32,           // Bits per byte (main metric)
    pub ce_loss: f32,         // Cross-entropy loss
    pub nca_entropy: f32,    // NCA entropy (regularization)
    pub samples: usize,       // Number of samples
}

pub fn evaluate(model: &IGLAModel, dataset: &FineWebDataset) -> EvalResult {
    // Compute BPB on full or sampled validation set
}
```

### 6.2 Invariant Enforcement

```rust
// INV-8: LR φ-band validation
fn validate_lr_band(lr: f32) -> bool {
    const PHI: f32 = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let min_lr = 1e-3;
    let max_lr = 1e-2;
    (min_lr..=max_lr).contains(&lr)
}

// R8: Gate-2 floor (step ≥ 4000 for ledger emit)
fn should_emit_ledger(step: usize) -> bool {
    step >= 4000
}

// Embargo: SHA block
fn check_embargo(sha: &str, embargo: &EmbargoBlock) -> bool {
    !embargo.is_blocked(sha)
}
```

---

## 7. Performance Optimizations

### 7.1 Memory

| Technique | Description | Impact |
|-----------|-------------|--------|
| **FineWeb mmap** | Memory-map binary data | No loading time, minimal RAM |
| **Gradient Checkpointing** | Save gradients only | Faster resume |
| **Activation Checkpointing** | Offload to CPU during eval | Save GPU memory |

### 7.2 Computation

| Technique | Description | Impact |
|-----------|-------------|--------|
| **Flash Attention** | O(N²) → O(N) for long contexts | Scale to longer sequences |
| **Mixed Precision** | BF16 for compute, FP32 for reduction | 2x faster, same accuracy |
| **Kernel Fusion** | Combine ops into single kernel | Reduce kernel launches |

### 7.3 I/O

| Technique | Description | Impact |
|-----------|-------------|--------|
| **Async Data Loading** | Prefetch next batch | Hide I/O latency |
| **Async Checkpointing** | Write while computing next step | No training stall |
| **Compression** | LZ4 checkpoint compression | 10-100x smaller files |

---

## 8. Migration Checklist

### PR-1: Model + Optimizer + Data

- [ ] `src/model.rs`: Complete IGLAModel struct
- [ ] `src/transformer.rs`: Migrate from trios-train-cpu
- [ ] `src/attention.rs`: Multi-head + hybrid support
- [ ] `src/forward.rs`: Complete forward pass
- [ ] `src/backward.rs`: Complete backward pass
- [ ] `src/optimizer.rs`: AdamW with φ-cosine
- [ ] `src/data.rs`: Integrate with forward pass
- [ ] `src/data/tokenizer.rs`: Migrate from trios-train-cpu
- [ ] Tests: Unit tests for each module

### PR-2: JEPA + Objective

- [ ] `src/jepa/`: Complete T-JEPA implementation
- [ ] `src/objective.rs`: CE + JEPA + NCA combination
- [ ] `src/invariants.rs`: INV-8 + R8 + embargo enforcement
- [ ] Tests: JEPA loss correctness
- [ ] Tests: Invariant validation

### PR-3: Champion Reproduction

- [ ] Run `champion.toml` for 27K steps
- [ ] Validate BPB ≈ 2.2393 ± 0.01
- [ ] Validate INV-8 at runtime
- [ ] Validate R8 ledger emission timing
- [ ] Validate triplet format in ledger
- [ ] Benchmark: Step time, throughput

---

## 9. Success Criteria

| Criterion | Target | Verification |
|------------|--------|--------------|
| **Champion BPB** | 2.2393 ± 0.01 | Run champion config 3x (seeds 43, 44, 45) |
| **Gate-2 Target** | BPB < 1.85 | Run gate2-attempt.toml 3x |
| **Training Speed** | > 1K steps/sec | Benchmark on laptop |
| **Memory** | < 8GB for champion | Profile RAM usage |
| **Checkpointing** | < 1s save/load | Benchmark I/O |
| **Invariants** | All enforced | Assert at runtime |
