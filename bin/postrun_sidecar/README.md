# trios-postrun-sidecar

Anchor: `phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)

## Purpose

Ingest BPB rows produced by tier runners (`trios-mr-tier{1,2a,2b,3}-runner`) into
`ssot.bpb_samples` on the Railway-hosted postgres SSOT
(`phd-postgres-ssot`). One row per `(cell_id, seed, sha_pin)` triple,
deduplicated by the table's UNIQUE constraint. The sidecar is the single
service authorised to write to `ssot.bpb_samples`; runners never write
directly.

## Self-bootstrap

`migrate.ts` runs first on every sidecar boot. It reads
`migrations/2026-05-09_bpb_samples.sql` and applies it against the
configured database. The migration is idempotent (every statement is
`CREATE … IF NOT EXISTS`), so re-running is always safe.

## Environment variables

| Var | Default | Purpose |
|-----|---------|---------|
| `RAILWAY_POSTGRES_URL` | — (required) | Primary postgres URL; points to `phd-postgres-ssot`. |
| `NEON_DATABASE_URL` | — | Legacy fallback (sunset pending L-NEON-RENAME). Used only if `RAILWAY_POSTGRES_URL` is unset. |
| `POLL_INTERVAL_S` | `60` | Seconds between polling iterations. |

## Deploy

Via the TRI MCP tool:

```
railway_service_deploy({
  name: "trios-postrun-sidecar",
  image: "ghcr.io/ghashtag/trios-trainer-igla:latest",
  env: {
    RAILWAY_POSTGRES_URL: "${{phd-postgres-ssot.DATABASE_URL}}",
    POLL_INTERVAL_S: "60",
    ENTRYPOINT_OVERRIDE: "node bin/postrun_sidecar/migrate.ts && node bin/postrun_sidecar/index.js"
  },
  project: "e4fe33bb-3b09-4842-9782-7d2dea1abc9b"
})
```

The `ENTRYPOINT_OVERRIDE` chains `migrate.ts` (synchronous, exits 0 on
success) and the long-running poll loop (`index.js`, not in scope for this
PR). Failure of `migrate.ts` aborts the chain so the sidecar never polls
against an un-migrated schema.

## Anchor

`phi^2 + phi^-2 = 3` · DOI [10.5281/zenodo.19227877](https://doi.org/10.5281/zenodo.19227877)
