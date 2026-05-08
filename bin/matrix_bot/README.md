# `bin/matrix_bot`

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

Hourly regenerator for the coverage matrix table inside `gHashTag/trios` issue
[#446](https://github.com/gHashTag/trios/issues/446). Reads aggregated row counts
from `ssot.bpb_samples` on the Railway `phd-postgres-ssot` Postgres SoT and
PATCHes the issue body, idempotently replacing the auto-block.

## Behaviour

- Polls Postgres every `POLL_INTERVAL_S` seconds (default 3600 = hourly).
- Aggregates `ssot.bpb_samples` by `tier`: row count, distinct cells, distinct
  seeds, min/avg/max BPB.
- Renders a Markdown table with totals and the per-tier breakdown.
- Wraps the block in `<!-- matrix_bot:auto:begin -->` / `<!-- matrix_bot:auto:end -->`
  markers and PATCHes the issue body. If the markers exist, the block is
  replaced in place; otherwise it is appended once.

## Environment variables

| Var | Default | Purpose |
|-----|---------|---------|
| `RAILWAY_POSTGRES_URL` | (required, with legacy fallback) | Postgres SoT connection string. Per L-NEON-RENAME, falls back to `NEON_DATABASE_URL` if unset. |
| `NEON_DATABASE_URL` | — | Legacy fallback for `RAILWAY_POSTGRES_URL`. |
| `GITHUB_TOKEN` | (required) | PAT or fine-grained token with `issues:write` on `GH_OWNER/GH_REPO`. |
| `ISSUE` | `446` | Issue number to keep in sync. |
| `GH_OWNER` | `gHashTag` | Issue owner. |
| `GH_REPO` | `trios` | Issue repo. |
| `POLL_INTERVAL_S` | `3600` | Seconds between ticks. |
| `SEEDS` | `47,89,144,123` | Canonical seed list (per L-SEED-CANON #600); used for `total_samples = TOTAL_CELLS × seeds.length`. |
| `TOTAL_CELLS` | `351` | Format×algo grid size (39 × 9 per L-MATRIX-FILL-351). |

## Markers (idempotent replace)

Block boundary on the issue body:

```
<!-- matrix_bot:auto:begin -->
… auto-generated table …
<!-- matrix_bot:auto:end -->
```

Re-running is safe: the regex replace targets exactly this block; everything
outside it is preserved.

## Deploy

The container is the same image used by every other Railway service in the
IGLA project (`ghcr.io/ghashtag/trios-trainer-igla:latest`) — `pg` and
`@octokit/rest` are already present from the trainer-igla base.

Use TRI MCP `railway_service_deploy` with `name="trios-matrix-bot"`, project
`e4fe33bb-3b09-4842-9782-7d2dea1abc9b` (IGLA), env from `MANIFEST.json` in
this directory. The `ENTRYPOINT_OVERRIDE` is `node bin/matrix_bot/main.js`
(after the image's TS build step).

## R5-honest scope notes

- `matrix_bot` writes ONLY inside the marker pair on issue #446. Hand-written
  prose around the auto-block is preserved.
- The bot is read-only against `ssot.bpb_samples`; no inserts, no mutations.
- The bot is non-fatal: a single tick error is logged and the loop continues.

## Anchor

`phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
