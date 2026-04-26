# Issue #143 — IGLA RACE Master Status (2026-04-26)

> **Last Updated:** 2026-04-26T08:30Z
> **Agent:** EPSILON

---

## Autonomous Hunt Summary

### BATCH 1 (60K steps, 6 configs)
| Exp ID | Config | Best BPB @ 60K | Notes |
|--------|---------|------------------|-------|
| E1 | LR=0.004, JEPA_W=1.0, NCA_W=0.25 | 2.1697 | Champion config baseline |
| E2 | LR=0.005, JEPA_W=1.0, NCA_W=0.25 | 2.1689 | **CHAMPION** |
| E3 | LR=0.003, JEPA_W=1.0, NCA_W=0.25 | 2.1793 | Lower LR |
| E4 | LR=0.004, JEPA_W=1.0, NCA_W=0.3 | 2.1697 | Higher NCA |
| E5 | LR=0.004, JEPA_W=1.25, NCA_W=0.25 | 2.1697 | Higher JEPA |
| E6 | LR=0.004, JEPA_W=1.0, NCA_W=0.25, warmup=2500 | 2.1697 | Higher warmup |

### BATCH 2 (80-100K steps, 5 configs)
| Exp ID | Config | Best BPB | Notes |
|--------|---------|----------|-------|
| E7 | LR=0.006 @ 80K | 2.1591 | Very high LR |
| E8 | LR=0.008 @ 80K | 2.1798 | Extreme LR |
| E9 | LR=0.005, NCA=0.5 @ 80K | 2.1476 | **CHAMPION** |
| E10 | LR=0.005, JEPA=0.75 @ 80K | 2.1476 | **TIED** |
| E11 | LR=0.005 @ 100K | 2.1387 | **NEW CHAMPION** |

### BATCH 3 (150K steps, 4 configs) — IN PROGRESS
| Exp ID | Config | Best BPB @ 43K | Notes |
|--------|---------|-----------------|-------|
| E12 | LR=0.005, JEPA=0.75, NCA=0.5 @ 150K | 2.3587 | Best combo |
| E13 | LR=0.0045, JEPA=0.75, NCA=0.5 @ 150K | 2.3408 | Lower LR |
| E14 | LR=0.005, JEPA=0.75, NCA=0.6 @ 150K | 2.3587 | Higher NCA |
| E15 | LR=0.005, JEPA=0.5, NCA=0.5 @ 150K | 2.3587 | Lower JEPA |

---

## Champion Progression

| Date | BPB | Steps | Config |
|------|-----|-------|--------|
| 2026-04-26T04:30Z | 2.1763 | 42K | LR=0.004, JEPA_W=1.0, NCA_W=0.25 |
| 2026-04-26T07:00Z | 2.1689 | 60K | LR=0.005, JEPA_W=1.0, NCA_W=0.25 |
| 2026-04-26T07:30Z | 2.1476 | 67K | LR=0.005, JEPA_W=0.75, NCA_W=0.5 |
| 2026-04-26T08:00Z | 2.1387 | 100K | LR=0.005, JEPA_W=0.75, NCA_W=0.5 |

**Total Improvement:** 2.1763 → 2.1387 = **0.0376 BPB** (~1.7%)

---

## Gate Status

| Gate | Target | Current | Status |
|------|--------|---------|--------|
| Gate-1 | ≤2.22 | 2.1387 | ✅ **PASSED** |
| Gate-2 | ≤2.03 | 2.1387 | 🔴 NOT REACHED (need ~0.11 BPB) |
| Gate-2 (pre-reg) | ≤1.85 | N/A | 🔴 NOT STARTED (requires hybrid architecture) |
| Gate-final | <1.50 | N/A | 🔴 NOT PRE-REGISTERED |

---

## Pre-Registered Gate-2 Plan (#143:4320342032)

**Architecture:** Hybrid ngram(dim=64, hidden=512, num_ctx=8) + 1-layer causal self-attention (d_model=64, 4 heads, RoPE, qk_gain=φ²=2.618) + JEPA predictor

**Key Parameters:**
- lr ∈ [α_φ/φ⁴, α_φ] where α_φ = 0.0072
- Cosine schedule 54K steps
- seed=43 for initial falsifier

**Falsifier:** If BPB > 2.00 at 54K OR divergence (Δval_BPB ≥ 0.5) → hypothesis burned (R5 Popper)

**Current Status:** Architecture NOT YET IMPLEMENTED in codebase

---

## Next Actions

1. **Implement Gate-2 hybrid architecture** (ngram + 1-layer causal SA + JEPA)
   - Expand n-gram to hidden=512, num_ctx=8
   - Add RoPE positional encoding
   - Add QK-Gain = φ² (INV-9)
   - Implement gradient computation for attention layer

2. **Launch L-h1/L-h3 experiments** on Gate-2 architecture (seed=43)

3. **Write Gate-final pre-registration** after Gate-2 results are available

---

**Comment URL:** https://github.com/gHashTag/trios/issues/143#issuecomment-4314616372
