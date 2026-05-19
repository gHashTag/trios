# Training Flow v2 — Gate-2 Decomposed Plan

> Status: **draft proposal** — aligned with trios-trainer-igla #24/#25
> Anchor: `phi^2 + phi^-2 = 3` ([Zenodo 10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877))
> Companion CLI: `tri railway` ONE SHOT

## TL;DR

Champion sits at **BPB=2.2393** (sha `2446855`, seed 43, step 27000). Gate-2 demands **BPB<1.85 on 3 seeds (43, 44, 45) with step >= 4000** before deadline `2026-04-30 23:59 UTC`.

Current gap: **0.39 BPB** to close.

This plan decomposes the chase into **6 phases** (P0..P5), each with one falsifiable hypothesis, one exit criterion, and one owner.

```
P0 Audit -> P1 OptLab -> P2 muP Transfer -> P3 SF+WSD -> P4 MultiObj+EMA -> P5 Gate-2 Push
audit        muon vs adam    8M -> 24M HPs     schedule-free    JEPA+NCA+EMA    3 seeds, ledger
```

## Standing Rules

| Rule | Description |
|------|-------------|
| **R5** | No DONE without merged PR + green CI + ledger row |
| **R7** | Every emit carries `BPB= @ step= seed= ~sha=<7c> jsonl_row= gate_status=` |
| **R8** | Ledger row only valid for `step >= 4000` |
| **R9** | Embargo (`assertions/embargo.txt`) checked before any `ledger::emit_row` |

## Why these levers (2025 evidence)

| Lever | Reported gain | Source |
|-------|---------------|--------|
| **Muon** (orthogonalized momentum) | -2.88% vs AdamW; ~1/3 fewer steps to 1B | [Shah et al. 2025 (IMU-1)](https://arxiv.org/abs/2602.02522) |
| **muP** (Maximal Update Parametrization) | Optimal LR at 8M transfers to >=10x width | [Cerebras muP Guide](https://www.cerebras.ai/blog/the-practitioners-guide-to-the-maximal-update-parameterization) |
| **Schedule-Free AdamW** | MLCommons 2024 AlgoPerf winner | [Defazio et al. 2024 (Meta AI)](https://ai.meta.com/research/publications/the-road-less-scheduled/) |
| **WSD** (Warmup-Stable-Decay) | Better anytime curves | [Wen et al. 2024](https://arxiv.org/abs/2404.06395) |
| **Post-hoc EMA** | Free generalization gain | [Sanyal et al. 2024](https://arxiv.org/abs/2411.18583) |

---

## P0 — Audit and Reproduce Champion

**Pre-conditions**: Clean checkout, FineWeb mirrors, `cargo test --release` green

**Hypothesis**: `configs/champion.toml --seed 43` reproduces `BPB = 2.2393 +/- 0.01 @ step 27000`

**Tasks**:
1. Re-run `tests/champion_reproduction.rs` with `--ignored`
2. Capture wall-clock + memory profile in `assertions/baseline_profile.json`
3. Snapshot HEAD SHA into `docs/audit/P0_seed43.md`
4. Diff `src/train_loop.rs` against `gHashTag/trios@2446855`
5. Lock the floor: append `champion@` to `assertions/champion_lock.txt`

**Exit criterion**: Ledger emits `BPB=2.2393 +/- 0.01 @ step=27000 seed=43 sha=<7c> gate_status=below_target_evidence`

**Falsification**: BPB drift > 0.05 -> bisect before any other phase

**Owner**: `repro-auditor`

**Margin**: 0 (floor)

---

## P1 — Optimizer Lab (AdamW vs Muon vs Muon+CWD)

**Pre-conditions**: P0 ledger row exists

**Hypothesis**: At champion architecture (256 d, 2L, 4H), Muon with `eta_2D=0.0235, eta_1D=0.007, momentum=0.95` reduces final BPB by **>=0.05** vs AdamW

**Tasks**:
1. Add `src/optimizer/muon.rs` — Newton-Schulz orthogonalization, 7 NS steps
2. Extend `OptimizerKind::Muon { eta_2d, eta_1d, momentum, ns_steps }`
3. New configs:
   - `configs/lab/p1-adamw.toml` (control)
   - `configs/lab/p1-muon.toml`
   - `configs/lab/p1-muon-cwd.toml`
4. Each config: 12K steps, seed 43 only (lab phase, NOT Gate-2 row)
5. CI gate: `cargo test --release optimizer::muon::ortho_invariant`

**Exit criterion**: `assertions/lab/p1_leaderboard.jsonl` with >=3 rows; winner by `argmin(bpb_final)` with margin >= 0.05

**Falsification**: Muon does not beat AdamW by >=0.05 -> proceed with AdamW, document null in `docs/audit/P1_null.md`

**Owner**: `optim-lab`

**Margin**: >=0.05 BPB

---

## P2 — muP Transfer (8M -> 24M -> Gate-2 Width)

**Pre-conditions**: P1 winner pinned

**Hypothesis**: At muP-anchored LR from 8M proxy, same scalar LR transfers to 24M and Gate-2 candidate (~70M) with **<=5% degradation** vs LR-swept baseline

**Tasks**:
1. Add `src/mup.rs`:
   - Input/output multiplier scaling
   - Attention QK 1/d_head scaling
   - Per-parameter-group LR scaling
2. Configs: `configs/lab/p2-proxy-8m.toml`, `p2-proxy-24m.toml`, `p2-target-70m.toml`
3. LR sweep on 8M: `{1e-3, 2e-3, 4e-3, 8e-3, 16e-3}` -> pick `lr_star`
4. Apply `lr_star` to 24M/70M with NO further sweep
5. Validate INV-8 at every sweep point

**Exit criterion**: `assertions/lab/p2_transfer.jsonl` shows 70M within 5% of swept baseline

**Falsification**: >10% degradation -> debug muP scaling factors

**Owner**: `mup-prover`

**Margin**: <5% degradation

---

## P3 — Schedule-Free AdamW + WSD

**Pre-conditions**: P1 + P2 winners frozen

**Hypothesis**: Replacing cosine `phi-schedule` with **Schedule-Free** (or WSD) yields **>=0.04 BPB** improvement AND strictly better anytime curve

**Tasks**:
1. Implement Schedule-Free in `src/optimizer.rs::schedule_free`:
   - `y_t = (1 - beta1) * z_t + beta1 * x_t`
   - Mixing coeff `c_{t+1} = 1/(t+1)`
2. Implement WSD: warmup (1K), stable (24K), decay (5K cosine)
3. Configs:
   - `configs/lab/p3-cosine.toml` (control)
   - `configs/lab/p3-sf.toml`
   - `configs/lab/p3-wsd.toml`
4. Eval every 500 steps, dump curve to `assertions/lab/p3_curves.jsonl`
5. Report anytime metric: `area_under_bpb_curve`

**Exit criterion**: Winner beats cosine by >=0.04 BPB AND anytime AUC drop >=5%

**Falsification**: Neither SF nor WSD dominates cosine -> stick with cosine, document null

**Owner**: `schedule-bench`

**Margin**: >=0.04 BPB + anytime dominance

---

## P4 — Multi-Objective + Post-hoc EMA

**Pre-conditions**: P3 winner frozen; `gate2-attempt.toml` weights as floor

**Hypothesis**: Weighted CE + JEPA + NCA with `(w_ce, w_jepa, w_nca)` sweep + post-hoc EMA(N=10) removes **>=0.03 BPB** at zero training cost

**Tasks**:
1. `src/objective.rs` — add per-loss gradient scaling
2. Sweep `(w_jepa, w_nca)` on `{(0.0,0.0), (0.5,0.0), (0.5,0.1), (0.7,0.15)}`
3. Post-hoc EMA in `src/checkpoint.rs::ema_average`
4. Config: `configs/lab/p4-objective.toml` + `p4-ema.toml`
5. Exit if BPB delta > +0.02 (EMA may not regress)

**Exit criterion**: `assertions/lab/p4_objective.jsonl` shows >=0.03 BPB drop, no row below champion floor

**Falsification**: EMA regresses on >=2 of 4 settings -> drop EMA from Gate-2 plan

**Owner**: `objective-jeweller`

**Margin**: >=0.03 BPB

---

## P5 — Gate-2 Push (3-Seed ONE SHOT)

**Pre-conditions**: P0..P4 merged; `configs/gate2-final.toml` baked from winners

**Hypothesis**: With P1..P4 winners stacked, all seeds in `{43,44,45}` yield **BPB < 1.85** at `step >= 4000` before `2026-04-30 23:59 UTC`

**Tasks**:
1. Pin `configs/gate2-final.toml`
2. Run `tri railway` ONE SHOT (`up --confirm`)
3. Operator POSTs to Railway; three services: `trainer-seed-43/44/45`
4. Each service emits R7 triplets every 500 steps
5. `assertions/seed_results.jsonl` accumulates; `tri railway gate2` reports verdict
6. Stop: 3 distinct seeds with `BPB < 1.85 AND step >= 4000` OR deadline

**Exit criterion**: 3 ledger rows with `gate_status="victory_candidate"` AND merged `feat: Gate-2 victory` PR

**Falsification**: Deadline hit without quorum -> publish `docs/audit/P5_postmortem.md`

**Owner**: `gate2-pilot`

**Margin**: merged victory PR

---

## Decision Matrix (pre-registered)

Filled only by merged PRs:

| Phase | Hypothesis margin | Outcome (BPB delta) | Decision | PR |
|-------|-------------------|---------------------|----------|-----|
| P0 | reproduce 2.2393 +/- 0.01 | _pending_ | _pending_ | _pending_ |
| P1 | Muon - AdamW <= -0.05 | _pending_ | _pending_ | _pending_ |
| P2 | muP transfer < 5% deg | _pending_ | _pending_ | _pending_ |
| P3 | SF/WSD - cosine <= -0.04 | _pending_ | _pending_ | _pending_ |
| P4 | objective+EMA <= -0.03 | _pending_ | _pending_ | _pending_ |
| P5 | 3 seeds < 1.85 | _pending_ | _pending_ | _pending_ |

---

## Lab vs Ledger Discipline (R7/R8 Hygiene)

**Lab rows** (`assertions/lab/*.jsonl`):
- NOT R7-validated triplets
- MAY have step < 4000
- For local decisions only
- Never roll up to Gate-2

**Ledger rows** (`assertions/seed_results.jsonl`):
- MUST satisfy R7 + R8 + R9
- Only P0 and P5 allowed to write here

To "promote" a lab row to ledger row MUST run full P5-style 3-seed verification.

---

## Code Touchpoints

| Phase | New files | Modified |
|-------|-----------|----------|
| P0 | `docs/audit/P0_seed43.md`, `assertions/baseline_profile.json`, `assertions/champion_lock.txt` | `tests/champion_reproduction.rs` |
| P1 | `src/optimizer/muon.rs`, `configs/lab/p1-*.toml` | `src/optimizer.rs`, `src/config.rs` |
| P2 | `src/mup.rs`, `configs/lab/p2-*.toml` | `src/model.rs`, `src/optimizer.rs` |
| P3 | _none_ | `src/optimizer.rs::schedule_free`, `src/optimizer.rs::wsd_lr` |
| P4 | `configs/lab/p4-*.toml` | `src/objective.rs`, `src/checkpoint.rs::ema_average` |
| P5 | `configs/gate2-final.toml`, `docs/audit/P5_*.md` | _none, by design_ |

---

## How to Start P0 Today

```bash
git checkout -b feat/p0-audit-25 main
cargo test --release reproduce_champion -- --ignored
git diff --no-index gHashTag/trios@2446855::trios-igla-trainer/src/train_loop.rs src/train_loop.rs > docs/audit/P0_drift.md
# run, capture, commit, R5-honest report
```

Submit PR titled `feat(p0): audit + champion reproduction (closes #N)`.

---

## Anchor

Mathematical foundation: `phi^2 + phi^-2 = 3` ([Zenodo 10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)).

Every phase MUST preserve this invariant in any modified numeric or scheduling code.
