# TASK-5A: T-JEPA Greenfield Design Spec

> **Status:** BLOCKED until greenfield implementation
> **Decision:** 2026-04-24T15:59Z — Switched to TASK-8, defer JEPA
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

---

## Required JEPA Components

### 1. Predictor Architecture
```rust
pub struct JepaPredictor {
    // Projector from embedding to prediction space
    projector: DenseLayer,
    // Span attention (if needed)
    span_attn: Option<SpanAttention>,
}
```

### 2. Span Masking
```rust
pub fn mask_spans(
    seq_len: usize,
    mask_ratio: f64,     // 0.3 from issue
    min_span: usize,     // 3
    max_span: usize,     // 7, 9, 11
    num_spans: usize,    // 2
    rng: &mut impl Rng,
) -> Vec<bool>  // true = masked
```

### 3. EMA Target Encoder
```rust
pub struct EmaTarget {
    decay_schedule: impl Fn(usize) -> f64,  // 0.996 → 1.0
    online_encoder: Encoder,
    target_encoder: Encoder,
}

pub fn ema_update(
    target: &mut [f32],
    online: &[f32],
    decay: f64,
) {
    // theta_target = decay * theta_target + (1 - decay) * theta_online
}
```

### 4. Multi-Objective Loss
```rust
pub struct HybridLoss {
    ntp_weight: f64,   // 0.5
    jepa_weight: f64,  // 0.25
    nca_weight: f64,   // 0.25
}

pub fn compute_loss(
    ntp_loss: f64,
    jepa_loss: f64,
    nca_loss: f64,
    config: &HybridLoss,
) -> f64
```

### 5. ASHA Guard
```rust
pub fn jepa_minimum_rung(arch: &str, rung: u64) -> bool {
    if arch == "jepa" { rung >= 3000 } else { true }
}
```

---

## Implementation Order

1. **TASK-5A.1:** Masking API (pure function, testable)
2. **TASK-5A.2:** EMA update (math, testable)
3. **TASK-5A.3:** Predictor skeleton (no training yet)
4. **TASK-5A.4:** JEPA loss computation
5. **TASK-5A.5:** ASHA guard in `asha.rs`
6. **TASK-5A.6:** Full trainer integration

---

## Success Criteria

- `--arch jepa` compiles and runs
- `BPB=X.XXXX` output maintained
- JEPA rung schedule: [3000, 9000, 27000]
- EMA decay schedule verified in tests
- Masking produces valid spans

---

## Dependencies

- `crates/trios-train-cpu/src/jepa.rs` (to create)
- `crates/trios-train-cpu/src/objective.rs` (to create)
- `crates/trios-igla-trainer/src/main.rs` (modify)
- `crates/trios-igla-race/src/asha.rs` (modify)
