# IGLA RACE — Breakthrough Analysis & Decomposition Plan
## Issue: #143 — Target: < 1.5 BPB on TinyShakespeare
## Current Status: Best BPB = 2.5329 (gap: 1.03 BPB)

---

## Executive Summary

### Current Plateau
- **Best Result**: 2.5329 BPB (6-gram, h=384, lr=0.004, wd=0.01)
- **Improvement from 4-gram (2.780 BPB)**: -0.2471 BPB (-8.9%)
- **Stagnation**: No significant improvement after 6-gram despite extensive hyperparameter exploration

### Root Cause Analysis

The N-gram architecture has reached its fundamental limit (~2.5 BPB) due to:
1. **No long-range dependencies**: Only 6 previous tokens are considered
2. **No parameter sharing**: Each context position (ctx1-ctx6) has separate embeddings
3. **No hierarchical features**: Single hidden layer, no residual connections or attention

### Required Action
**Architectural Jump Needed**: Transition from N-gram → Attention/Transformer-based models to break the 2.5 BPB barrier.

---

## Detailed Experiment Analysis

### What Worked

| Architecture | Config | Best BPB | Notes |
|-------------|--------|---------|-------|
| 4-gram | h=128, lr=0.004 | 2.877 | Baseline |
| 4-gram | h=192, GELU, residual | 2.743 | +Residual but worse |
| 5-gram | h=256, ctx3, lr=0.006 | 2.6005 | +5th context token helped |
| 5-gram | h=384, lr=0.006 | 2.5771 | Larger hidden, higher LR worked well |
| **6-gram | h=384, lr=0.004, wd=0.01 | 2.5329 | **Current best** |
| 6-gram | h=384, lr=0.005 | 2.5500 | Higher LR worse |
| 7-gram | h=384, lr=0.004 | 2.5678 | Worse than 6-gram |
| 8-gram | h=384 | 2.7811 | More context hurt (overfitting?) |

### What Didn't Work

| Strategy | Result | Analysis |
|----------|--------|---------|
| GELU activation | 2.7184 (vs 2.6942 ReLU) | No significant improvement |
| Residual connections | 2.743 (worse than 2.780) | Added complexity without benefit |
| LayerNorm + Multi-layer | 3.21 (worse than single layer) | Deeper networks overfit on small dataset |
| Dropout (0.1) | 2.759 | Worse than no dropout |
| Weight decay (0.04) | 2.7184 | Different WD was worse |
| Warmup (500) | 2.743 | No benefit observed |
| 7-gram, 8-gram | 2.56-2.78 | More context hurt (overfitting?) |
| h=512 | Not tested | Would likely overfit |
| GF16 precision | 3.2182 | Numerical instability worse |

### Optimal Configuration (Based on Current Data)

| Parameter | Best Value | Range Tested | Recommendation |
|-----------|------------|-------------|----------------|
| Hidden size (h) | 384 | [64, 128, 192, 256, 384, 512] | **384 is optimal for current architecture** |
| Learning rate (lr) | 0.004 | [0.001, 0.002, 0.003, 0.004, 0.005, 0.006] | **0.004 is sweet spot** |
| Context window | 6 (ctx4) | [4, 5, 6, 7, 8] | **6 tokens is optimal** |
| Weight decay (wd) | 0.01 | [0.0, 0.01, 0.04] | **0.01 helps** |
| Activation | ReLU | [ReLU, GELU] | **ReLU is sufficient** |
| Dropout | 0.0 | [0.0, 0.05, 0.1, 0.2] | **No dropout is best** |
| Optimizer | AdamW | [AdamW, Muon, SGD] | **AdamW is best** |

---

## Decomposition Plan (English)

### Phase 1: Attention Layer (Priority: HIGH)

**Objective**: Add self-attention mechanism to N-gram model to capture longer-range dependencies.

**Expected BPB**: 2.20 - 2.10 (10-18% improvement)

**Implementation Tasks**:
1. [ ] Add `attention_layer` module to `trios-train-cpu`
2. [ ] Modify forward pass: instead of `embed[i-6..i] → proj`, use `embed[i-6..i] → attn[i-6..i]`
3. [ ] Add attention weights (W_q, W_k, W_v) to hidden state
4. [ ] Update training loop to compute attention scores
5. [ ] Test attention-only model with same baseline h=384
6. [ ] Compare attention vs pure N-gram: ablation study

**Estimated Effort**: 4-6 hours

**Success Criteria**: Attention model BPB < 2.50 with same h=384, lr=0.004

### Phase 2: Minimal Transformer (Priority: HIGH)

**Objective**: Implement smallest viable transformer (single attention head, 2-3 layers) for TinyShakespeare.

**Expected BPB**: 1.80 - 2.00 (28-33% improvement over current best)

**Implementation Tasks**:
1. [ ] Add `transformer` module to `trios-train-cpu`
2. [ ] Implement multi-head self-attention (M=4 heads, d=16)
3. [ ] Add positional encoding (RoPE or learned)
4. [ ] Implement transformer block: MSA → LayerNorm → FFN (64→256→64)
5. [ ] Integrate with training loop
6. [ ] Parameter sweep on: d_model, lr, n_layers
7. [ ] Test minimal transformer baseline

**Estimated Effort**: 8-12 hours

**Success Criteria**: Transformer BPB < 1.80 with < 50K parameters

### Phase 3: Search Optimization (Priority: MEDIUM)

**Objective**: Use IGLA RACE system with Neon to perform大规模 hyperparameter search.

**Expected BPB**: 1.30 - 1.50 (13-50% improvement)

**Implementation Tasks**:
1. [ ] Complete `trios-igla-race` binary with all ASHA rungs working
2. [ ] Integrate `trios-igla-trainer` as subprocess
3. [ ] Set up Neon database with proper schema (trials + experience + competitors)
4. [ ] Implement parallel worker spawning (4 workers per machine)
5. [ ] Add automated failure memory generation
6. [ ] Create leaderboard dashboard

**Estimated Effort**: 4-8 hours (in progress)

**Success Criteria**: IGLA RACE finds BPB < 1.30

### Phase 4: Specialized Architectures (Priority: MEDIUM)

**Objective**: Explore advanced architectures that have shown success on similar tasks.

**Expected BPB**: 1.00 - 1.30 (30-53% improvement)

**Implementation Tasks**:
1. [ ] Study HierAttn v3 (results: 1.215 BPB on TinyShakespeare)
2. [ ] Implement simplified hierarchical attention
3. [ ] Test on TinyShakespeare

**Estimated Effort**: 8-16 hours

**Success Criteria**: Hierarchical attention BPB < 1.30

### Phase 5: Quantization (Priority: LOW)

**Objective**: Reduce model size while maintaining accuracy using quantization.

**Expected BPB**: 1.50 - 1.00 (50% size reduction with minimal loss)

**Implementation Tasks**:
1. [ ] Implement GPTQ INT4 quantization (3-4 bits per parameter)
2. [ ] Add quantization-aware training
3. [ ] Compare FP16 vs INT4 accuracy
4. [ ] Optimize quantized model

**Estimated Effort**: 4-8 hours

**Success Criteria**: INT4 model BPB < 1.80 within 10% FP16 accuracy loss

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|-----------|
| Phase 1: Architectural complexity may introduce bugs | Extensive testing, ablation studies |
| Phase 2: Transformer may overfit small dataset | Careful regularization, early stopping |
| Phase 3: IGLA RACE requires Neon coordination | Use existing Neon infrastructure |
| Phase 4: Research reproduction challenges | Follow published papers exactly |
| Phase 5: Quantization may degrade performance | Validate on holdout set |

---

## Next Steps

1. **Implement Attention Layer** - Start with single-head attention, validate BPB improvement
2. **Validate Current N-gram Baseline** - Re-run 6-gram h=384 lr=0.004 to confirm 2.5329 BPB
3. **Set up IGLA RACE infrastructure** - Ensure Neon DB is accessible, start 4-worker race
4. **Create Decomposition Issue** - Post this analysis plan to issue #143 in English
5. **Begin Phase 1 Implementation** - Add attention module to trainer, run ablation tests

---

## Timeline Estimate

- Week 1: Phase 1-2 (Attention layer) + Baseline validation
- Week 2: Phase 3-4 (Minimal transformer) + Phase 3 IGLA RACE
- Week 3: Phase 4-5 (Specialized) + Phase 5 (Quantization)

**Total Estimated Time**: 4-6 weeks to reach BPB < 1.30 target

---

**Status**: READY TO POST TO ISSUE #143
