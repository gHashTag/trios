---
name: trainer-igla-sot
description: "ONE SHOT protocol for TRAINER-IGLA-SOT mission — migrating the trios training pipeline into the single source of truth repo gHashTag/trios-trainer-igla. Use when the user mentions TRAINER-IGLA-SOT, trios-trainer-igla, Gate-2, BPB < 1.85, the 5-lane migration (L-T1..L-T5), champion reproduction at BPB=2.2393, embargo ledger, triplet format, or pre-registered Gate G1/G2/G3 acceptance for the trios trainer extraction. Anchor: phi^2 + phi^-2 = 3, Zenodo DOI 10.5281/zenodo.19227877."
metadata:
  author: gHashTag
  version: '1.0'
  mission: TRAINER-IGLA-SOT
  anchor: "phi^2 + phi^-2 = 3"
  zenodo_doi: 10.5281/zenodo.19227877
  gate2_deadline_utc: "2026-04-30T23:59:00Z"
  source_repo: gHashTag/trios-trainer-igla
  legacy_repo: gHashTag/trios
  supersedes_issue: gHashTag/trios#320
---

# TRAINER-IGLA-SOT — ONE SHOT Protocol

Mission codename: **TRAINER-IGLA-SOT**
Auditor lane: `perplexity-computer-grandmaster` (R5 honesty lane)
Anchor: `phi^2 + phi^-2 = 3` — Zenodo DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
Gate-2 deadline: **2026-04-30 23:59 UTC**
Source of truth: [gHashTag/trios-trainer-igla](https://github.com/gHashTag/trios-trainer-igla) ·
[SOURCE_OF_TRUTH.md](https://github.com/gHashTag/trios-trainer-igla/blob/main/SOURCE_OF_TRUTH.md) ·
[MIGRATION.md](https://github.com/gHashTag/trios-trainer-igla/blob/main/MIGRATION.md)
Supersedes [gHashTag/trios#320](https://github.com/gHashTag/trios/issues/320).
Refs hub: #2 (master tracking) · #1 (dashboard) ·
[gHashTag/trios#143](https://github.com/gHashTag/trios/issues/143) (legacy).

## When to Use This Skill

Load this skill whenever the user:

- Asks to work on **TRAINER-IGLA-SOT**, **trios-trainer-igla**, or any of issues #4–#8 in that repo
- Mentions Gate-2, BPB < 1.85, 3-seed pass on seeds {43, 44, 45}
- References champion reproduction at `BPB = 2.2393 ± 0.01` (commit
  [gHashTag/trios@2446855](https://github.com/gHashTag/trios/commit/2446855))
- Wants to execute or audit any of the 5 migration lanes (L-T1 … L-T5)
- Talks about the trainer ledger, embargo window, or the `triplet` evidence row
- Files a PR / opens an issue in `gHashTag/trios-trainer-igla` or the DELETE phase in `gHashTag/trios`
- Asks "is this DONE?" — apply the R5-honest gate before answering

## Background (R5-honest)

- Stop-rule for Gate-2: **3 seeds passing BPB < 1.85** by **2026-04-30 23:59 UTC**.
- Last 7 days produced **0** such rows in Hive.
- Champion ([gHashTag/trios@2446855](https://github.com/gHashTag/trios/commit/2446855))
  holds at `BPB = 2.2393` (single seed).
- Most recent evidence row
  ([gHashTag/trios@1272929](https://github.com/gHashTag/trios/commit/1272929))
  is `BPB = 2.497` — **regression**, not progress.
- Root cause: training stack fragmented across 5 overlapping crates and 23
  binaries in `gHashTag/trios`. "Canonical training command" was undefined.
- Fix: `gHashTag/trios-trainer-igla` is the single source of truth for the
  trainer. This ONE SHOT pre-registers the migration in 5 lanes.

## Hypothesis (Gate G1)

> **H** = Migrating the training pipeline into a single external crate
> `trios-trainer-igla` reduces time-to-first-3-seed-Gate-2-row by **≥ 3×**
> vs the v0 baseline, while preserving champion `BPB ≤ 2.2393 ± 0.01` on the
> same hardware, the same seed, and the same corpus checksum.

### Falsification — H is rejected iff any of:

| Code | Condition |
|---|---|
| **REGRESSION** | PR-1 + PR-2 cannot reproduce champion within `BPB = 2.2393 ± 0.01` |
| **DRIFT** | Invariants imported from `trios-igla-race` diverge during migration |
| **EMBARGO BYPASS** | Any embargoed SHA is accepted by `ledger::emit_row` |
| **R1 VIOLATION** | Any Python file under `gHashTag/trios/scripts/` survives PR-3 |

### Falsification witnesses (must land in code)

- `tests/champion_reproduction.rs` — full 27K-step run, asserts
  `final_bpb ∈ [2.229, 2.249]`
- `tests/embargo_block.rs` — synthetic emit with embargoed SHA must `bail!`
- `tests/invariants_match.rs` — `--features trios-integration` cross-check

## Pre-registered analysis plan (Gate G2)

| Field | Value |
|---|---|
| `acceptance_metric` | `mean(BPB)` over 3 seeds `{43, 44, 45}` < `1.85`, all rows `≥ 4000` steps |
| `champion_guard` | every PR's CI runs `champion.toml` smoke; reject if `final_bpb > 2.25` |
| `alpha` | `0.01` (Welch one-tailed against `mu_0 = 1.55`) |
| `effect_size` | `Delta_BPB ≥ 0.05` |
| `n_required` | 3 distinct passing seeds |
| `stop_rule` | first 3-seed pass **OR** `2026-04-30 23:59 UTC` |
| `embargo_window` | 24 h after each promoted config |
| `triplet` | `BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>` |

The **triplet** is mandatory on every `ledger::emit_row` call.

## Method — Five Lanes

Lanes are independent. Each is a self-contained PR; failure of one does not
block the others.

### L-T1 — Migrate model + optimizer + tokenizer — Issue #4

```
git mv gHashTag/trios/crates/trios-train-cpu/{
  transformer.rs, hybrid_attn.rs, optimizer.rs,
  forward.rs, backward.rs, tokenizer.rs
} gHashTag/trios-trainer-igla/src/
```

Rewrite `bin/hybrid_train.rs` as a thin wrapper.
**Acceptance:** champion config reproduces `BPB = 2.2393 ± 0.01`.

### L-T2 — Migrate JEPA + objective — Issue #5

```
git mv jepa/{ema, loss, masking, predictor, mod}.rs   ->  src/jepa/
git mv objective.rs                                    ->  src/
```

Merge `trios-igla-trainer::jepa_runner` into `src/jepa/runner.rs`.
**Acceptance:** `gate2-attempt.toml` runs end-to-end and emits 1
triplet-validated row.

### L-T3 — DELETE phase in `gHashTag/trios` — Issue #6

DELETE:

- Crates: `trios-train-cpu`, `trios-training`, `trios-training-ffi`,
  `trios-igla-trainer` (entire crates)
- 5 backup files in `trios-igla-race/src/`
- 3 Python scripts in `scripts/`

Net diff: **≈ −350 KB / +0**.

### L-T4 — Update CI to use external trainer — Issue #7

`.github/workflows/leaderboard.yml` uses:

```bash
cargo install --git https://github.com/gHashTag/trios-trainer-igla \
  --branch main trios-train --locked
```

Pin to a SHA **after PR-1 is green**.

### L-T5 — Docker + Railway 3-seed deploy — Issue #8

- Push image to `ghcr.io/ghashtag/trios-trainer-igla:latest`
- Deploy 3 services to Railway project
  [`e4fe33bb-3b09-4842-9782-7d2dea1abc9b`](https://railway.com/project/e4fe33bb-3b09-4842-9782-7d2dea1abc9b)
  on seeds `{43, 44, 45}`
- **Acceptance:** 3 cloud rows in `assertions/seed_results.jsonl` within 6 h

## Acceptance (Gate G3)

ALL of:

- [ ] **L-T1** PR merged; `champion_reproduction.rs` (ignored) passes manually
- [ ] **L-T2** PR merged; `gate2-attempt.toml` runs end-to-end
- [ ] **L-T3** PR merged in `gHashTag/trios`: 4 crates + 5 backups + 3 Python deleted
- [ ] **L-T4** PR merged in `gHashTag/trios`: `leaderboard.yml` uses `cargo install --git`
- [ ] **L-T5** PR merged: `ghcr.io` image present + 3 Railway services healthy
- [ ] 3 triplet-validated rows in `assertions/seed_results.jsonl` with
      `seed ∈ {43, 44, 45}`, `step ≥ 4000`, `bpb < 1.85`
- [ ] All emits respect embargo on `assertions/embargo.txt`

### R5-honest gate

> **NO DONE** without merged PR + CI green + ledger row written.

If any of the three is missing, the lane is **NOT DONE**. Report status
honestly even when partial — never overclaim.

## Risk Register

| Risk | Mitigation |
|---|---|
| Champion path breaks during L-T1 | `trios-train-cpu` stays alive in `gHashTag/trios` until PR-1+PR-2 reproduction tests are green; only **L-T3** deletes |
| Git history lost on extraction | `git filter-repo --path crates/trios-train-cpu` graft |
| INV drift | Trainer imports invariants only via `--features trios-integration` |
| Embargo bypass | `ledger::is_embargoed` runs before every emit; CI test asserts all 8 embargoed SHAs are refused |
| Two forks emitting rows | `agent` field stamps `trios-trainer-{run-name}`; embargo + triplet still hold |
| Railway service cost overrun | auto-pause on idle; `restartPolicyMaxRetries = 10` |

## Standing Orders

R1, R3, R4, R5, R6, R7, R8, R9, R10 — see
[MIGRATION.md → standing-rules-carried-over](https://github.com/gHashTag/trios-trainer-igla/blob/main/MIGRATION.md#standing-rules-carried-over).

Quick mnemonic for the most cited:

- **R1** — No Python in `gHashTag/trios/scripts/`. Rust-only.
- **R5** — Honesty lane: no DONE without merged PR + CI green + ledger row.
- **R7** — Every emit carries the full triplet.
- **R9** — Embargo respected before every `ledger::emit_row`.

## Operating Procedure for the Agent

When invoked under this mission, the agent **must**:

1. **Identify the lane** the user is asking about (L-T1 … L-T5) or the audit task.
2. **Reproduce champion first** when touching L-T1 / L-T2 — never delete in
   `gHashTag/trios` before PR-1 + PR-2 reproduction tests are green (risk
   register, row 1).
3. **Stamp every ledger emit** with the mandatory triplet:
   `BPB=<v> @ step=<N> seed=<S> sha=<7c> jsonl_row=<L> gate_status=<g>`.
4. **Refuse embargoed SHAs** — if a SHA is in `assertions/embargo.txt`,
   `ledger::is_embargoed` must `bail!`. Do not patch around it.
5. **Cross-check invariants** under `--features trios-integration` before
   declaring L-T1/L-T2 acceptance.
6. **Apply the R5 honesty gate** before claiming any lane DONE: merged PR
   + green CI + written ledger row. If any is missing, report the lane as
   IN PROGRESS or BLOCKED with the missing artifact named explicitly.
7. **Stay inside the 5 lanes.** New work that does not fit the lanes goes
   in a follow-up issue, not in this ONE SHOT.
8. **Track the deadline.** All work is timed against
   `2026-04-30 23:59 UTC`. When asked for status, include time-to-deadline.

## Example Status Reply

> **Lane status @ 2026-04-26**
>
> - L-T1 (#4): PR open, champion_reproduction.rs **failing** at step 27000
>   (final_bpb=2.251) — IN PROGRESS, BLOCKED on optimizer port.
> - L-T2 (#5): not started.
> - L-T3 (#6): not started — gated on L-T1+L-T2 green per risk register.
> - L-T4 (#7): draft PR, awaiting L-T1 SHA pin.
> - L-T5 (#8): image builds locally, Railway services not deployed.
>
> **Gate-2 deadline:** T-3.5 d (2026-04-30 23:59 UTC).
> **R5 verdict:** NOT DONE — 0 / 5 lanes merged, 0 / 3 seed rows written.
