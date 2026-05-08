# Matrix Runner Topology — Path to 351/351

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

This runbook describes the parallel Railway-service deployment plan for closing
the 351-cell Format×Algorithm matrix on `gHashTag/trios#446`. Origin lane:
issue `L-MATRIX-FILL-351`. Companion docs: `matrix-runner-retrieval.md`
(commit-back orchestrator, lane #598).

## State today (baseline)

- Total cells: **351** (39 formats × 9 algos)
- Measured R7-honest: **38** (only `adamw` + `muon` historical runs)
- Synthetic placeholders: **313**
- Coverage: **10.83%**

R7-honest is defined by `.github/scripts/closure_gate.py`:

- `COUNT(*)` ≥ 3 sample rows per cell
- `COUNT(DISTINCT seed_phi)` ≥ 2
- `MAX(step)` ≥ 3000

## Tier partition

The 351 cells are partitioned into three disjoint tiers with strict R5-honest
priorities:

| Tier | Count | CSV | Coverage rationale |
|------|------:|-----|--------------------|
| 1 | 50  | `assertions/matrix_priority_50.csv` | Frozen subset from L-MR-MATRIX-PRIORITY (PR #589). Concentrates `gf16` × all 8 pre-SOAP algos + 21 fmts × `{adamw, muon}`. |
| 2 | 100 | `assertions/matrix_priority_tier2.csv` | The 17 missing formats × `{adamw, muon}` (34) + `gf16 × soap` (1, post-KAPPA fill) + `{bf16, fp16}` × 7 non-`{adamw,muon}` algos (14) + remaining `gf*` × 7 non-`{adamw,muon}` algos (49) + 2 `tf32` filler cells (`sgdm`, `lion`). |
| 3 | 201 | `assertions/matrix_priority_tier3.csv` | Long-tail: every cell not assigned to tiers 1 or 2. |
| **Total** | **351** | — | — |

Math: 50 + 100 + 201 = 351. No cell is double-counted. The 38 already-measured
historic cells are *inside* tier 1 by design (re-measured under the canonical
seed pool to satisfy R7).

> The user-facing brief stated `351 - 50 - 100 - 38 = 163` for tier 3.
> R5-honest accounting puts the 38 historical measurements *inside* tier 1
> (because they re-run anyway under the canonical seed pool), so tier 3 is the
> remainder = 201, not 163. The discrepancy is documented here rather than
> resolved by fabricating a 38-row "already_measured" tier.

## Canonical seeds (L-SEED-CANON #600)

Every tier runs **4 seeds**: `{47, 89, 144, 123}`. Earlier runs used a
3-seed pool `{47, 89, 144}`; the 4th seed `123` was added by L-SEED-CANON
ruling on issue #600 to firm up the Welch-t test (4 seeds → 3 distinct pairs
for variance estimation).

`scripts/run_priority_matrix.ts` (frozen tier-1 runner) defaults to
`SEEDS=47,89,144`. The new dispatcher `scripts/run_matrix_tier.ts` overrides
this with the canonical 4-seed list whenever the env `SEEDS` is unset.

## Parallel Railway topology

Four Railway services, all in the IGLA project, each pinned to the same
`trios-trainer-igla:latest` image SHA (current pin: `1d3632ba`):

| Service | Tier | Cells | Est. CPU wall-clock | Image | TIER env |
|---------|-----:|------:|---------------------|-------|---------:|
| `trios-mr-tier1-runner` (existing, id `71f5aac2-d4d5-4640-8895-90ced5d4ea63`) | 1 | 50  | ~12.5 h | `1d3632ba` | `TIER=1` |
| `trios-mr-tier2a-runner` (new) | 2 (first half, 50)   | 50  | ~12.5 h | `1d3632ba` | `TIER=2`, `MATRIX_RANGE=51..100` |
| `trios-mr-tier2b-runner` (new) | 2 (second half, 50)  | 50  | ~12.5 h | `1d3632ba` | `TIER=2`, `MATRIX_RANGE=101..150` |
| `trios-mr-tier3-runner`  (new) | 3 | 201 | ~50 h *or* split by 2 | `1d3632ba` | `TIER=3` |

Estimates assume the same per-run cost as the priority-50 runner: 50 cells × 4
seeds × 3000 steps ≈ 12.5 h on the standard Railway CPU instance.

### Wall-clock to 351/351

- **Tier 1 + Tier 2a + Tier 2b** running in parallel: ~12.5 h (bottleneck is any
  one of the three).
- **Tier 3** running alone: ~50 h on a single CPU service, or split into
  `tier3a/tier3b` services (~25 h each, parallel).
- **Total parallel wall-clock**: ~25 h (tier 3 split) — matches the user's
  4-services × 5–6 h GPU goal *only* if each tier 3 half runs on GPU.

R5-honest: on **CPU only**, even with 4 parallel services the bottleneck is the
201-cell tier 3, which takes ~50 h serial or ~25 h parallel-split. The user's
"4–6 h" target is achievable only with GPU acceleration.

### Cost note

CPU services on Railway are billed by service-hour. Estimated total:

- 3 services × 12.5 h (tiers 1, 2a, 2b) = 37.5 service-hours
- 2 services × 25 h (tiers 3a, 3b)      = 50 service-hours
- **Total ≈ 87.5 service-hours**

This is the canonical price for closing the matrix once. Re-runs are
hash-deduplicated by the postrun orchestrator (lane #598), so a partial
re-deploy does not re-burn the budget.

## Postrun retrieval (shared)

A single sidecar service drains `assertions/matrix_samples.jsonl` from any
tier:

- Service: `trios-mr-postrun-sidecar` (one shared instance).
- Image: same repo, `scripts/postrun_sidecar.ts` start command.
- Volume: must mount **all four runner volumes** read-only (or each runner
  pushes its JSONL to a shared volume hosted on Railway storage). The simpler
  pattern is the latter: every runner writes to a shared
  `assertions/matrix_samples.jsonl` on a Railway-managed shared volume.
- Interval: 30 min (env `INTERVAL_MIN=30`).
- The sidecar opens PRs in batches of 25 (env `BATCH_SIZE=25`).

The sidecar is the only service authorised to push to `gHashTag/trios`; the
runners never push.

## Coverage CI gate

`scripts/matrix_coverage_report.ts` is the canonical reporter. CI runs it
hourly (or on every postrun-PR merge) and compares against
`assertions/matrix_coverage_baseline.json`. A regression — fewer R7-honest
cells than the baseline — is grounds for failing the lane.

The reporter outputs JSON to stdout and Markdown via `--markdown-out=<path>`.
For the matrix-bot's hourly cron, the JSON output is uploaded to the
Railway-hosted dashboard at `trios-production.up.railway.app`.

## Deployment checklist

For each new Railway service (`tier2a`, `tier2b`, `tier3`):

1. Source: `gHashTag/trios`, branch `main` (after this PR merges).
2. Build command: `cargo build --release -p trios-trainer-igla` *or* pull the
   image SHA `1d3632ba` from `ghcr.io/ghashtag/trios-trainer-igla:latest`.
3. Start command: `npx tsx scripts/run_matrix_tier.ts`.
4. Volume mount: shared volume with `assertions/matrix_samples.jsonl`.
5. Required env:
   - `TIER` — `1`, `2`, or `3`.
   - `TRIOS_TRAINER_BIN` — absolute path to the prebuilt trainer.
6. Optional env:
   - `STEPS` — default `3000`.
   - `SEEDS` — default `47,89,144,123` (L-SEED-CANON #600).
   - `DRY_RUN=1` — print plan without invoking the trainer.

The sidecar service is identical to the one documented in
`matrix-runner-retrieval.md` and does not change for this lane.

## Anchor

`phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
