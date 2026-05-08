# Matrix Coverage Snapshot — 2026-05-09 (Wave 12.G2 first manual report)

**Anchor:** `phi^2 + phi^-2 = 3`
**DOI:** [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
**Frozen ledger:** [`assertions/matrix_legacy_snapshot_2026-05-04.jsonl`](../assertions/matrix_legacy_snapshot_2026-05-04.jsonl) (38 cells × {`adamw`, `muon`}, ts `2026-05-04T10:52:46Z`)
**Supersession contract:** `live (ssot.bpb_samples) > legacy-frozen (jsonl) > 🔲 (no observation)`
**SHA pin:** `9715415a` · **Seed canon:** `{47, 89, 144, 123}` (per L-SEED-CANON #600) · **Steps:** `3000`
**Parent issue:** [#446](https://github.com/gHashTag/trios/issues/446) (mega-issue; no separate sub-issue for this snapshot)

> ## R5-honest disclaimer
> 
> This is a **deploy-moment snapshot**, not a samples-landed snapshot. The Wave-16 matrix-runner fleet (16 services across IGLA + acc2 + acc4 + acc6) finished deploying minutes before this report; **no new BPB samples have landed yet**. Live cells therefore show 🔲 across the board.
> 
> The 38 legacy entries from the frozen ledger of 2026-05-04 are carried forward verbatim under their canonical format keys (`fp32`→`f32`, `fp64`→`f64` mapping documented in §2.1). The legacy entries record `best_bpb` only — they do **not** carry per-seed splits — so per-seed cells under legacy formats remain 🔲 with the legacy-best value displayed in a dedicated column.
> 
> The DB query path is **confirmed broken**: `worker_status` and `experiment_queue_status` MCP tools return `error connecting to server` (issue #126), and the Neon SoT quota is exhausted. Full per-seed counts will be available in the next snapshot (T+60 min) once the matrix-runner-postrun sidecar drains the runners' JSONL into `ssot.bpb_samples` on the Railway `phd-postgres-ssot` instance.

## 1. Coverage summary

- Total cells: **351** (39 formats × 9 algos)
- Total samples target: **1404** (351 cells × 4 seeds)
- Legacy-frozen cells (carried forward): **38** (10.83% of cells, only `adamw` + `muon` columns)
- Live samples: **0** (deploy-moment snapshot; runners are armed but no data drained yet)
- Combined coverage at this instant: **38/351 cells** with at least one ledger entry; **0/1404 per-seed samples** observed.

## 2. Coverage matrix (per-algo tables)

Cell legend:
- ⬛ `<value>` — legacy-frozen `best_bpb` from `matrix_legacy_snapshot_2026-05-04.jsonl` (no per-seed split available; the value is the historical best across the legacy run set).
- 🔲 — no observation: cell not yet in legacy ledger AND not yet in `ssot.bpb_samples`.
- *(no live cells yet for this snapshot — see R5 disclaimer above)*

### 2.1 Format-key mapping (legacy → canonical)

| Legacy ledger key | Canonical key (matrix_bot.py / runner CLI) |
|---|---|
| `fp32` | `f32` |
| `fp64` | `f64` |
| (all others) | (identical) |

### 2.2.1 `adamw`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7478` |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.8010` |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.6719` |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7566` |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7103` |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.2933` |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7566` |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.3263` |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7424` |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7103` |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.6864` |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7003` |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `adamw`: legacy cells = 19/39 (49%); live samples = 0/156._

### 2.2.2 `muon`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.6568` |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.6808` |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.6568` |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.8383` |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `4.1152` |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.8645` |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `4.1152` |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.5655` |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `3.3263` |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `4.1152` |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7424` |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `4.1152` |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.8982` |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.8070` |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.6864` |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | ⬛ `2.7566` |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `muon`: legacy cells = 19/39 (49%); live samples = 0/156._

### 2.2.3 `sgdm`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `sgdm`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.4 `lion`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `lion`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.5 `adafactor`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `adafactor`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.6 `lamb`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `lamb`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.7 `schedulefree`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `schedulefree`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.8 `rmsprop`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `rmsprop`: legacy cells = 0/39 (0%); live samples = 0/156._

### 2.2.9 `soap`

| format | seed=47 | seed=89 | seed=144 | seed=123 | legacy_best |
|---|---|---|---|---|---|
| `f32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `f64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `bf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `tf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e4m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp8_e5m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e2m3` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp6_e3m2` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp4_e2m1` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf12` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf20` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf24` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `gf64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `int32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `uint8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `nf8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit16` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `posit64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `lns8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp4` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp6` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `mxfp8` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal32` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal64` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `decimal128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary128` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `binary256` | 🔲 | 🔲 | 🔲 | 🔲 | — |
| `fp80` | 🔲 | 🔲 | 🔲 | 🔲 | — |

_Algo `soap`: legacy cells = 0/39 (0%); live samples = 0/156._

## 3. Active fleet inventory (Wave 16, 16 matrix-runners)

### 3.1 IGLA project (`e4fe33bb-3b09-4842-9782-7d2dea1abc9b`) — 13 matrix runners

| Service | ID | Cell range / seed | Notes |
|---------|-----|-------------------|-------|
| `trios-mr-priority-runner` | `71f5aac2` | priority cells | tier-1 frozen (50 cells from `matrix_priority_50.csv`) |
| `trios-mr-tier1a-runner` | `b20cf052` | tier1 0:25 | first half of priority-50 |
| `trios-mr-tier1b-runner` | `32cee577` | tier1 25:50 | second half of priority-50 |
| `trios-mr-tier2a-runner` | `e2c7447c` | tier2 50:75 | first half of `matrix_priority_tier2.csv` |
| `trios-mr-tier2b-runner` | `683c4ece` | tier2 75:100 | second half of `matrix_priority_tier2.csv` |
| `trios-mr-tier3-runner` | `b0bd370e` | tier3 0:100 (legacy wide) | legacy wide range — superseded once `tier3b/c/d` saturate |
| `trios-mr-tier3b-runner` | `22d82210` | tier3 100:201 | wide back-fill |
| `trios-mr-tier3c-runner` | `b03133c2` | tier3 100:150 | mid-band of `matrix_priority_tier3.csv` |
| `trios-mr-tier3d-runner` | `5003c8e0` | tier3 150:201 | tail of `matrix_priority_tier3.csv` |
| `trios-mr-seed47-runner` | `19c85634` | seed=47, all cells | fan-out by seed |
| `trios-mr-seed89-runner` | `c3e27a4f` | seed=89, all cells | fan-out by seed |
| `trios-mr-seed144-runner` | `fe9a7801` | seed=144, all cells | fan-out by seed |
| `trios-mr-seed123-runner` | `43afaa6d` | seed=123, all cells | fan-out by seed (4th canon seed) |

### 3.2 Cross-account runners (3)

| Service | Project | ID | Cell range |
|---------|---------|----|------------|
| `trios-mr-acc2-runner` | acc2 reasonable-perception (`12c508c7`) | `3f10fc42` | cells 87:175 |
| `trios-mr-acc4-runner` | acc4 believable-connection (`0247abaa`) | `8a15af8d` | cells 175:263 |
| `trios-mr-acc6-runner` | acc6 robust-radiance (`475a2290`) | `422b1ffd` | cells 263:351 |

> The `acc1` slot for a fourth cross-account runner is **blocked** — trial expired (see §5).

### 3.3 Coordination + infrastructure (sibling services, not runners)

| Role | Service | ID |
|------|---------|----|
| Hourly issue-body regenerator (#446) | `trios-matrix-bot` | `91ef9d5c` (900s tick) |
| JSONL → SoT drain | `trios-postrun-sidecar` | `066163be` |
| Canonical seed-1597 bridge | `trios-train-ONE-v2-acc1-s1597` | `94a833e9` |
| Postgres SoT (Railway) | `phd-postgres-ssot` | `c5f37b42` |
| trios-railway gateway | `trios-railway` | `b84f7b81` |
| MCP gateway | `trios-railway-mcp` | `db786a4b` |
| Public MCP | `trios-mcp-public` | `3abc18da` |

## 4. Throughput projection

Assumptions:

- Each runner processes one `(format, algo, seed_phi)` cell at a time at `STEPS=3000` per cell.
- Per-cell wall-clock on the standard Railway CPU instance: ≈15 min (calibrated against the pre-Wave-16 priority runner).
- Disjoint cell windows across the 16 runners (the 13 IGLA runners + 3 cross-account runners) → no double-work; the postrun sidecar dedups by canonical row hash regardless.

Projected throughput at saturation:

- 16 runners × (60 min / 15 min per cell) ≈ **64 cell-completions / hour**.
- With 4 canonical seeds, each unique `(format, algo)` cell needs 4 completions → **64 / 4 = 16 unique cells / hour** by the 4-seed quorum metric.
- 351 unique cells / 16 unique-cells-per-hour ≈ **22 hours** to full 1404-sample coverage at saturation.

Caveats (R5-honest):

- The 15-min figure is from the pre-Wave-16 single-runner profile; cross-account runners on different physical Railway hosts may run faster or slower.
- Several runners share cell windows (e.g. `tier3-runner` overlaps `tier3b/c/d`); the postrun sidecar dedups but the wall-clock budget of the redundant runners is not additive.
- Neon SoT quota exhaustion (see §5) **blocks observation** of cells the runners do produce until the SoT is restored or runners switch to JSONL-only mode and the sidecar re-drains.

A realistic ETA, with caveats, is **22–48 h to full saturation**, with the next snapshot (T+60 min) confirming whether real throughput tracks the 64-completion-per-hour projection.

## 5. Honest gaps (R5)

- **`worker_status` MCP tool broken** (issue [#126](https://github.com/gHashTag/trios/issues/126)). Calls return `error connecting to server`. Per-runner liveness can only be inferred from Railway logs, not queried directly. **Impact**: this report cannot show "runner X is currently processing cell Y" — only the static fleet inventory.
- **`experiment_queue_status` MCP tool broken** (same root cause as `worker_status`). The pull-queue depth is not observable through the MCP gateway right now.
- **Neon SoT quota exhausted.** The legacy Neon `ssot.bpb_samples` instance hit its monthly read quota; queries return `quota exceeded`. Per L-NEON-RENAME (PR #125 on `gHashTag/trios-railway`), the runtime backing store has moved to Railway service `phd-postgres-ssot` (`c5f37b42`), but the historical Neon ledger is still the source for cells under that legacy URL. The matrix-runner-postrun sidecar (`066163be`) writes to the Railway SoT going forward; legacy data remains read-blocked until quota resets.
- **acc1 trial expired.** The `acc1` Railway account ran out of trial credits, blocking the 4th cross-account slot. The 16-runner total holds because the 13 IGLA runners + 3 cross-account (acc2 / acc4 / acc6) cover the full 351-cell space without acc1.

## 6. Next snapshot

- **Schedule**: T+60 min from this report (`docs/matrix_coverage_2026-05-09T22-20.md` or similar).
- **Trigger**: first runner samples expected to land in `assertions/matrix_samples.jsonl` and be drained into the Railway SoT by then.
- **Expected delta**: a handful of `tier1a/b` and `tier2a/b` cells in `adamw` / `muon` should flip from 🔲 to live values across at least 1–2 of the 4 seeds.
- **Bot dependency**: the `trios-matrix-bot` (service `91ef9d5c`) ticks every 900 s and PATCHes #446 — this manual report does not replace the bot; it is a **first-snapshot reference** that captures the deploy moment in version control.

## Anchor

`phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
