# Training-Flow V2 — Gate-2 Push Plan

> **Target:** BPB < 1.85 on 3 seeds (quorum 3/3) before deadline
> **Champion baseline:** BPB = 2.2393 @ 27K steps, seed=43 (commit 2446855)
> **Gap to close:** 0.39 BPB points

## Phase P0 — Audit (Champion Reproduction)

**Hypothesis:** Champion config reproduces BPB=2.2393 ± 0.01
**Exit criterion:** 3/3 seeds within 0.01 of baseline
**Status:** Done

## Phase P1 — Optimizer Lab (Muon vs AdamW)

**Hypothesis:** Muon optimizer achieves >0.05 BPB improvement over AdamW
**Exit criterion:** Paired t-test p < 0.05, n >= 3 seeds
**Status:** In progress

Configs:
- `config_muon105.toml` — Muon with lr=1e-3
- AdamW baseline with same lr schedule

## Phase P2 — μP Transfer (8M → 70M Scaling)

**Hypothesis:** μP transfer scales loss monotonically from 8M to 70M params
**Exit criterion:** 70M loss < 8M loss, delta > 0.1 BPB
**Status:** Planned

## Phase P3 — Schedule-Free + WSD

**Hypothesis:** Schedule-Free or WSD scheduler beats cosine by >0.03 BPB
**Exit criterion:** Best scheduler wins paired comparison
**Status:** Planned

## Phase P4 — Multi-Objective + EMA (JEPA + NCA)

**Hypothesis:** Joint JEPA+NCA objective beats single-task CE
**Exit criterion:** Multi-obj BPB < single-task BPB by >0.02
**Status:** Planned

## Phase P5 — Gate-2 Push (3 seeds < 1.85)

**Hypothesis:** Best config from P1-P4 achieves BPB < 1.85 on seeds {42,43,44}
**Exit criterion:** `victory::check_victory()` returns Ok(VictoryReport) for 3/3 seeds
**Status:** Planned

## Pre-Registered Decision Matrix

| Phase | Config | Seed(s) | Steps | BPB | Verdict | PR |
|-------|--------|---------|-------|-----|---------|-----|
| P0 | champion | 43 | 27K | 2.2393 | Reproduced | — |
| P1 | pending | pending | pending | pending | pending | pending |

Only merged PRs fill this matrix. No retroactive entries.
