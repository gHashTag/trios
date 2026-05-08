# Format × Algorithm Coordinate Matrix

Closes the matrix-coverage strand of [#446](https://github.com/gHashTag/trios/issues/446)
that was opened in [issuecomment-4370442020](https://github.com/gHashTag/trios/issues/446#issuecomment-4370442020).

## Pipeline

```
issuecomment-4370442020  (frozen 2026-05-04, 38 cells)
        ↓
assertions/matrix_legacy_snapshot_2026-05-04.jsonl   ← legacy ledger (R5-honest, frozen)
        ↓                                            ↓ (overlay)
scripts/render_matrix_446.ts                        bin/matrix_bot/main.ts
        ↓                                            ↓
docs/matrix/coordinate_matrix_<date>.md             #446 body block <!-- matrix_bot:auto:begin --> ... <!-- matrix_bot:auto:end -->
```

## R5-honest supersession rule

For every `(format, algo)` cell:
1. If `ssot.bpb_samples` has `COUNT(*) ≥ 3 AND COUNT(DISTINCT seed) ≥ 2 AND MAX(step) ≥ 3000`
   for that cell → live value wins, rendered as **bold**.
2. Else, fall back to the legacy snapshot value, rendered as `val ⓛ`.
3. Else, render `🔲` (TODO).

Live cells permanently supersede legacy cells; the legacy ledger is **never edited** in place
(R10 atomic). New columns of evidence are added via fresh `assertions/matrix_legacy_snapshot_<date>.jsonl`
files plus a PR.

## Files

| Path | Role |
|---|---|
| `assertions/matrix_legacy_snapshot_2026-05-04.jsonl` | 38-cell frozen legacy snapshot (R7-honest per legacy ledger) |
| `assertions/matrix_priority_50.csv` | tier-1 priority cells (50) |
| `assertions/matrix_priority_tier2.csv` | tier-2 (100) |
| `assertions/matrix_priority_tier3.csv` | tier-3 long tail (201) |
| `assertions/matrix_per_cell_audit.csv` | full 351-cell R7 audit |
| `scripts/render_matrix_446.ts` | ad-hoc renderer (one-shot, writes Markdown) |
| `bin/matrix_bot/main.ts` | hourly cron regenerator (writes #446 body) |
| `docs/matrix/coordinate_matrix_2026-05-09.md` | first regenerated artefact (legacy-only at first run) |

## Rendering manually

```bash
# legacy-only render (no DB)
npx tsx scripts/render_matrix_446.ts --output=docs/matrix/coordinate_matrix_$(date -u +%F).md

# with live overlay
RAILWAY_POSTGRES_URL=postgres://... \
  npx tsx scripts/render_matrix_446.ts --output=docs/matrix/coordinate_matrix_$(date -u +%F).md
```

## Status as of 2026-05-09

- **Frozen baseline**: 38 / 312 cells (12.2%) from the 2026-05-04 ledger
- **Live overlay**: bootstrapping via `trios-postrun-sidecar` (Wave 10) into `ssot.bpb_samples`
- **Topology**: 5 runners (tier1 + tier2a + tier2b + tier3a + tier3b) × 4 seeds {47, 89, 144, 123}
- **Target**: 312 / 312 cells live (R7-honest) by Wave 12 close

φ² + φ⁻² = 3 · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
