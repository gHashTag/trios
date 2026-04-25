# TASK-5A: T-JEPA Greenfield Design Spec v2

> **Status:** BLOCKED until greenfield implementation
> **Decision:** 2026-04-24T15:59Z — Switched to TASK-8, defer JEPA
> **Updated:** 2026-04-24T16:30Z — Detailed API and module structure
> **Reason:** Referenced branch `feat/physics-migration-phase-a` and JEPA modules do not exist

---

## Why TASK-5 Blocked

Original TASK-5 assumed integration from existing code:
- Merge `tjepa.rs` and `objective.rs` from branch
- Wire into existing trainer CLI
- Add ASHA guard for 3000-step minimum

**Reality:**
- Branch does not exist
- `jepa.rs` was 0-byte placeholder
- No predictor architecture, EMA, or masking code

**Conclusion:** TASK-5 is not integration — it's greenfield R&D.
**This spec makes it implementable.**

---

## Module Structure

```
crates/trios-train-cpu/src/
├── jepa/
│   ├── mod.rs           # Public API
│   ├── masking.rs       # Span masking
│   ├── predictor.rs     # Prediction head
│   ├── ema.rs           # Exponential Moving Average
│   └── loss.rs          # JEPA loss computation
└── objective.rs         # Multi-objective (NTP + JEPA + NCA)
```

---

## Public API (jepa/mod.rs)

```rust
//! T-JEPA (Ternary Joint Embedding Predictive Architecture)
//!
//! Implements masked prediction with EMA target encoder.
//! Based on LeJEPA/LeWorldModel principles.

pub use masking::{MaskConfig, mask_spans, MaskResult};
pub use predictor::{Predictor, PredictorConfig, PredictionOutput};
pub use ema::{EmaTarget, EmaConfig, ema_update};
pub use loss::{JepaLoss, JepaLossConfig, compute_jepa_loss};

/// JEPA training configuration
#[derive(Debug, Clone)]
pub struct JepaConfig {
    pub seed: u64,
    pub d_model: usize,
    pub mask_ratio: f64,
    pub min_span: usize,
    pub max_span: usize,
    pub num_spans: usize,
    pub ema_start: f64,
    pub ema_end: f64,
    pub predictor_lr_mult: f64,
}

impl Default for JepaConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            d_model: 384,
            mask_ratio: 0.3,
            min_span: 3,
            max_span: 9,
            num_spans: 2,
            ema_start: 0.996,
            ema_end: 1.0,
            predictor_lr_mult: 0.1,
        }
    }
}

/// JEPA training result
#[derive(Debug, Clone)]
pub struct JepaResult {
    pub steps_completed: usize,
    pub final_loss: f64,
    pub final_variance: f64,
    pub loss_monotone: bool,
    pub ema_verified: bool,
    pub converged: bool,
}
```

---

## 1. Span Masking (jepa/masking.rs)

```rust
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Mask configuration
#[derive(Debug, Clone, Copy)]
pub struct MaskConfig {
    pub ratio: f64,       // 0.3
    pub min_span: usize,  // 3
    pub max_span: usize,  // 7, 9, 11
    pub num_spans: usize, // 2
}

impl Default for MaskConfig {
    fn default() -> Self {
        Self {
            ratio: 0.3,
            min_span: 3,
            max_span: 9,
            num_spans: 2,
        }
    }
}

/// Mask result
#[derive(Debug, Clone)]
pub struct MaskResult {
    /// Boolean mask: true = masked position
    pub mask: Vec<bool>,
    /// List of (start, end) for each masked span
    pub spans: Vec<(usize, usize)>,
}

/// Generate random span masks
///
/// # Arguments
/// * `seq_len` - Sequence length
/// * `config` - Mask configuration
/// * `rng` - Random number generator
///
/// # Returns
/// MaskResult with boolean mask and span boundaries
pub fn mask_spans(
    seq_len: usize,
    config: MaskConfig,
    rng: &mut impl Rng,
) -> MaskResult {
    let mut mask = vec![false; seq_len];
    let mut spans = Vec::new();

    for _ in 0..config.num_spans {
        let span_len = rng.gen_range(config.min_span..=config.max_span);
        let start = rng.gen_range(0..seq_len.saturating_sub(span_len));
        let end = (start + span_len).min(seq_len);

        for i in start..end {
            mask[i] = true;
        }
        spans.push((start, end));
    }

    MaskResult { mask, spans }
}

/// Get unmasked positions
pub fn get_unmasked(mask: &[bool]) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter_map(|(i, &m)| if !m { Some(i) } else { None })
        .collect()
}

/// Get masked positions
pub fn get_masked(mask: &[bool]) -> Vec<usize> {
    mask.iter()
        .enumerate()
        .filter_map(|(i, &m)| if m { Some(i) } else { None })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_ratio_approximate() {
        let mut rng = StdRng::seed_from_u64(42);
        let result = mask_spans(100, MaskConfig::default(), &mut rng);

        let masked_count = result.mask.iter().filter(|&&m| m).count();
        let ratio = masked_count as f64 / 100.0;

        assert!((ratio - 0.3).abs() < 0.2); // Allow variance
    }

    #[test]
    fn test_span_bounds() {
        let mut rng = StdRng::seed_from_u64(42);
        let result = mask_spans(100, MaskConfig::default(), &mut rng);

        for (start, end) in result.spans {
            assert!(start < end);
            assert!(end <= 100);
            assert!(end - start >= 3);
            assert!(end - start <= 11);
        }
    }
}
```

---

## 2. EMA Target Encoder (jepa/ema.rs)

```rust
/// EMA configuration
#[derive(Debug, Clone, Copy)]
pub struct EmaConfig {
    pub start: f64,  // 0.996
    pub end: f64,    // 1.0
    pub ramp_steps: usize,  // Steps to reach end
}

impl Default for EmaConfig {
    fn default() -> Self {
        Self {
            start: 0.996,
            end: 1.0,
            ramp_steps: 30000,
        }
    }
}

/// EMA target encoder
pub struct EmaTarget {
    config: EmaConfig,
    step: usize,
}

impl EmaTarget {
    pub fn new(config: EmaConfig) -> Self {
        Self { config, step: 0 }
    }

    /// Get current decay rate
    pub fn decay(&self) -> f64 {
        if self.step >= self.config.ramp_steps {
            self.config.end
        } else {
            let progress = self.step as f64 / self.config.ramp_steps as f64;
            self.config.start + (self.config.end - self.config.start) * progress
        }
    }

    /// Update target parameters using EMA
    ///
    /// # Arguments
    /// * `target` - Target parameters to update (in-place)
    /// * `online` - Online encoder parameters
    pub fn update(&mut self, target: &mut [f32], online: &[f32]) {
        let decay = self.decay();
        ema_update(target, online, decay);
        self.step += 1;
    }

    /// Reset step counter
    pub fn reset(&mut self) {
        self.step = 0;
    }
}

/// EMA update function (pure)
///
/// theta_target = decay * theta_target + (1 - decay) * theta_online
pub fn ema_update(target: &mut [f32], online: &[f32], decay: f64) {
    assert_eq!(target.len(), online.len());

    for (t, o) in target.iter_mut().zip(online.iter()) {
        *t = decay * *t + (1.0 - decay) * *o;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_decay_schedule() {
        let config = EmaConfig {
            start: 0.5,
            end: 1.0,
            ramp_steps: 100,
        };
        let mut ema = EmaTarget::new(config);

        assert_eq!(ema.decay(), 0.5);

        for _ in 0..50 {
            ema.step += 1;
        }
        assert!((ema.decay() - 0.75).abs() < 0.01);

        for _ in 50..100 {
            ema.step += 1;
        }
        assert_eq!(ema.decay(), 1.0);
    }

    #[test]
    fn test_ema_update() {
        let mut target = vec![1.0, 1.0];
        let online = vec![2.0, 0.0];

        ema_update(&mut target, &online, 0.9);

        assert!((target[0] - 1.1).abs() < 1e-6);  // 0.9*1 + 0.1*2
        assert!((target[1] - 0.9).abs() < 1e-6);  // 0.9*1 + 0.1*0
    }
}
```

---

## 3. Predictor (jepa/predictor.rs)

```rust
/// Predictor configuration
#[derive(Debug, Clone)]
pub struct PredictorConfig {
    pub d_model: usize,
    pub hidden_dim: usize,
    pub num_layers: usize,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self {
            d_model: 384,
            hidden_dim: 256,
            num_layers: 2,
        }
    }
}

/// Prediction output
#[derive(Debug, Clone)]
pub struct PredictionOutput {
    pub predicted: Vec<f32>,
    pub target: Vec<f32>,
    pub loss: f64,
}

/// JEPA predictor head
pub struct Predictor {
    config: PredictorConfig,
    weights: Vec<f32>,
}

impl Predictor {
    pub fn new(config: PredictorConfig) -> Self {
        let total_params = config.d_model * config.hidden_dim
            + config.hidden_dim * config.num_layers
            + config.hidden_dim * config.d_model;

        Self {
            config,
            weights: vec![0.0; total_params],
        }
    }

    /// Forward pass: predict target embeddings from context embeddings
    ///
    /// # Arguments
    /// * `context` - Unmasked context embeddings
    /// * `target_positions` - Positions to predict
    pub fn forward(
        &self,
        context: &[f32],
        target_positions: &[usize],
    ) -> Vec<f32> {
        // Placeholder: implement projection
        // For now, return context as prediction
        let d_model = self.config.d_model;
        let mut predicted = Vec::with_capacity(target_positions.len() * d_model);

        for &pos in target_positions {
            if pos < context.len() / d_model {
                let start = pos * d_model;
                let end = start + d_model;
                predicted.extend_from_slice(&context[start..end]);
            } else {
                predicted.extend(vec![0.0; d_model]);
            }
        }

        predicted
    }

    /// Compute prediction loss (L2 norm)
    pub fn compute_loss(&self, predicted: &[f32], target: &[f32]) -> f64 {
        predicted
            .iter()
            .zip(target.iter())
            .map(|(p, t)| (p - t).powi(2) as f64)
            .sum::<f64>() / predicted.len() as f64
    }
}
```

---

## 4. JEPA Loss (jepa/loss.rs)

```rust
/// JEPA loss configuration
#[derive(Debug, Clone, Copy)]
pub struct JepaLossConfig {
    pub use_l2_normalization: bool,
    pub stop_gradient: bool,
}

impl Default for JepaLossConfig {
    fn default() -> Self {
        Self {
            use_l2_normalization: true,
            stop_gradient: true,
        }
    }
}

/// JEPA loss result
#[derive(Debug, Clone)]
pub struct JepaLoss {
    pub total: f64,
    pub prediction: f64,
    pub variance: f64,
}

/// Compute JEPA loss with optional L2 normalization
pub fn compute_jepa_loss(
    predicted: &[f32],
    target: &[f32],
    config: JepaLossConfig,
) -> JepaLoss {
    let (pred, tgt) = if config.use_l2_normalization {
        (l2_normalize(predict), l2_normalize(target))
    } else {
        (predicted.to_vec(), target.to_vec())
    };

    // Prediction loss (L2)
    let prediction_loss = pred
        .iter()
        .zip(tgt.iter())
        .map(|(p, t)| (p - t).powi(2) as f64)
        .sum::<f64>() / pred.len() as f64;

    // Variance (anti-collapse)
    let mean = tgt.iter().sum::<f32>() as f64 / tgt.len() as f64;
    let variance = tgt
        .iter()
        .map(|t| (*t as f64 - mean).powi(2))
        .sum::<f64>() / tgt.len() as f64;

    JepaLoss {
        total: prediction_loss - variance * 0.01, // Anti-collapse weight
        prediction: prediction_loss,
        variance,
    }
}

/// L2 normalize a vector
fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
    if norm < 1e-8 {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}
```

---

## 5. Multi-Objective (objective.rs)

```rust
/// Multi-objective configuration
#[derive(Debug, Clone, Copy)]
pub struct ObjectiveConfig {
    pub ntp_weight: f64,
    pub jepa_weight: f64,
    pub nca_weight: f64,
}

impl Default for ObjectiveConfig {
    fn default() -> Self {
        Self {
            ntp_weight: 0.5,
            jepa_weight: 0.25,
            nca_weight: 0.25,
        }
    }
}

/// Component losses
#[derive(Debug, Clone)]
pub struct ComponentLosses {
    pub ntp: f64,
    pub jepa: f64,
    pub nca: f64,
}

/// Combined loss result
#[derive(Debug, Clone)]
pub struct CombinedLoss {
    pub total: f64,
    pub components: ComponentLosses,
}

/// Compute weighted combined loss
pub fn compute_combined_loss(
    components: ComponentLosses,
    config: ObjectiveConfig,
) -> CombinedLoss {
    let total = components.ntp * config.ntp_weight
        + components.jepa * config.jepa_weight
        + components.nca * config.nca_weight;

    CombinedLoss { total, components }
}

/// NCA entropy constraint
///
/// Enforces entropy in band [1.5, 2.8] as hard constraint
pub fn nca_entropy_constraint(entropy: f64) -> f64 {
    const MIN_ENTROPY: f64 = 1.5;
    const MAX_ENTROPY: f64 = 2.8;

    if entropy < MIN_ENTROPY {
        (MIN_ENTROPY - entropy).powi(2) * 100.0
    } else if entropy > MAX_ENTROPY {
        (entropy - MAX_ENTROPY).powi(2) * 100.0
    } else {
        0.0
    }
}
```

---

## 6. ASHA Guard Integration (asha.rs)

```rust
/// Get rung schedule for a given architecture
///
/// JEPA requires 3000+ steps for first rung due to slower convergence
pub fn get_rung_schedule(arch: &str) -> Vec<i32> {
    match arch {
        "jepa" => vec![3000, 9000, 27000],
        _ => vec![1000, 3000, 9000, 27000],
    }
}

/// Check if trial should skip a rung
pub fn should_skip_rung(arch: &str, rung: i32) -> bool {
    arch == "jepa" && rung < 3000
}
```

---

## Integration Points

### trios-igla-trainer/src/main.rs

```rust
#[arg(long, default_value = "ngram")]
arch: String,  // Add: ngram | jepa | attn | hybrid

// In training loop:
match args.arch.as_str() {
    "jepa" => {
        let jepa_config = JepaConfig {
            seed: args.seed,
            d_model: args.hidden,
            ..Default::default()
        };
        // ... JEPA training
    }
    _ => {
        // ... N-gram training
    }
}
```

### trios-igla-race/src/asha.rs

```rust
// Update rung loop:
let rungs = get_rung_schedule(&config.arch);

for rung in &rungs {
    if should_skip_rung(&config.arch, *rung) {
        continue;
    }
    // ... training step
}
```

---

## Implementation Order (Revised)

1. **TASK-5A.1:** `jepa/masking.rs` — Pure functions, fully testable
2. **TASK-5A.2:** `jepa/ema.rs` — Math only, fully testable
3. **TASK-5A.3:** `jepa/predictor.rs` — Skeleton with forward pass
4. **TASK-5A.4:** `jepa/loss.rs` — Loss computation
5. **TASK-5A.5:** `jepa/mod.rs` — Public API, JepaConfig, JepaResult
6. **TASK-5A.6:** `objective.rs` — Multi-objective wrapper
7. **TASK-5A.7:** ASHA guard in `asha.rs`
8. **TASK-5A.8:** Trainer integration in `igla-trainer`

---

## Success Criteria

- ✅ `cargo test -p trios-train-cpu` passes for all JEPA modules
- ✅ `--arch jepa` compiles and runs
- ✅ `BPB=X.XXXX` output maintained (stdout last line only)
- ✅ JEPA rung schedule: [3000, 9000, 27000] (skip 1000)
- ✅ EMA decay verified in tests (0.996 → 1.0)
- ✅ Masking produces valid spans (min=3, max=11)
- ✅ L2 normalization tested
- ✅ Variance > 0.01 (no collapse) in mock runs

---

## Test Coverage Requirements

### masking.rs
- `test_mask_ratio_approximate` - Ratio ~0.3
- `test_span_bounds` - Spans within min/max
- `test_get_unmasked` - Correct unmasked positions
- `test_get_masked` - Correct masked positions

### ema.rs
- `test_ema_decay_schedule` - Linear ramp
- `test_ema_update` - Correct formula
- `test_ema_reset` - Step counter reset

### loss.rs
- `test_l2_normalize` - Unit norm
- `test_jepa_loss_computation` - L2 + variance
- `test_anti_collapse` - Variance > 0.01

### Integration
- `test_full_jepa_forward` - Mask → Predict → Loss
- `test_ema_in_training` - EMA updates each step

---

## References

- Issue #143: https://github.com/gHashTag/trios/issues/143
- LeWorldModel: https://linkedin.com/posts/yann-lecun_boom-a-clean-recipe-to-train-jepa-world-7441886063993847808-aUCH
- IGLA-GF16 3-Model Synthesis: `.trinity/specs/igla-gf16-3model-synthesis.md`
