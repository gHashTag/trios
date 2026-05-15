# ADR-CHAT-011: SOVEREIGN SCARAB — pull-from-Postgres control plane

**Status:** Proposed (R5-prototype verified 2026-05-14)
**Date:** 2026-05-14
**Anchor:** `phi^2 + phi^-2 = 3` · TRINITY · Defense 2026-06-15

## Context

Sessions PASS-13 → PASS-20 documented a chronic class of failures in the
Trinity IGLA training fleet:

- `RAILWAY_TOKEN_ACC{1..7}` rotation breakage (Issue
  [trios-railway#156](https://github.com/gHashTag/trios-railway/issues/156))
  — every token-rotation window invalidates the writer-env-fix /
  mcp-emergency-redeploy / token-classify workflows for 6–48 h.
- `variableUpsert` mutations under PAT-implicit auth still return
  `Not Authorized` under certain edge cases (B-20 unresolved as of PASS-20).
- The writer has been DEAD ≥ 793 minutes (latest HEALER cron tick:
  `last_seen` 846 min stale; `rows_5m=0`, `rows_4h=0`).
- Cure A (`writer-env-fix.yml`) is skipped by the HEALER cron via the
  `TOKEN-INVALID GUARD` because the last run already failed at
  `Sanity-check tokens`. **The control plane itself depends on the same
  rotating tokens it is supposed to fix.**

This is a fundamental architectural problem: **the Queen-Hive command path
goes _outside_ the cluster through Railway GraphQL**, so it inherits every
auth weakness of Railway PAT/account-token surface.

## Decision

Move the entire scarab/trainer command surface **into the cluster** via a
single Postgres table that scarabs poll on a 30-second tick:

```
NOW (broken):
  GitHub Actions → Railway GraphQL (variableUpsert + redeploy)
                     [PAT/account token hell · rotates · breaks]
                   → scarabs

TARGET (sovereign):
  Scarabs deploy ONCE forever
    → every 30s: SELECT * FROM ssot.scarab_strategy WHERE service_id=$me
    → if generation > current_gen: kill + respawn training with new params
    → write ssot.scarab_heartbeat (last_seen, current_gen, step, bpb, pid)
  Queen-Hive command = UPDATE ssot.scarab_strategy
    (NO Railway API · NO PAT · only DATABASE_URL deploy-time secret)
```

Only one token is required end-to-end: **`DATABASE_URL`** — the canonical
Railway/Neon plugin env var. (Not `NEON_DATABASE_URL` — that name does not
exist on Railway, see PASS-20 operator correction.)
`DATABASE_URL` is injected at service deploy time, does not rotate, and is
never touched by GitHub Actions.

## Schema (R5-verified, local Postgres 17.9, 2026-05-14)

```sql
CREATE SCHEMA IF NOT EXISTS ssot;

CREATE TABLE ssot.scarab_strategy (
  service_id  text PRIMARY KEY,            -- 'igla-1', 'matrix-runner-acc2-19', 'local-A'
  account     text NOT NULL,               -- 'acc1' | 'acc2' | 'local'
  optimizer   text NOT NULL CHECK (optimizer IN ('adamw','muon','muon-cwd')),
  format      text NOT NULL,
  hidden      int  NOT NULL CHECK (hidden > 0),
  lr          numeric NOT NULL CHECK (lr > 0),
  seed        int  NOT NULL,
  steps       int  NOT NULL CHECK (steps > 0),
  status      text NOT NULL DEFAULT 'active' CHECK (status IN ('active','paused','stop')),
  generation  bigint NOT NULL DEFAULT 1,   -- bump → scarab restarts training
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  text                         -- 'queen-hive' | 'operator' | 'gardener'
);

CREATE TABLE ssot.scarab_heartbeat (
  service_id   text PRIMARY KEY,
  last_seen    timestamptz NOT NULL DEFAULT now(),
  current_gen  bigint NOT NULL,
  current_step int,
  current_bpb  double precision,
  pid          int,
  started_at   timestamptz
);

CREATE INDEX scarab_heartbeat_stale ON ssot.scarab_heartbeat (last_seen);
```

Plus `ssot.bump_strategy(service_id, ...)` SQL function: helper that
atomically updates a row and bumps `generation`, the only thing scarabs
ever check.

## Pull-loop (Rust prototype, R5-verified 2026-05-14)

`scarab-pull-loop` (`/home/user/workspace/scarab-pull-loop`), ≈ 180 lines
of Rust, dependencies: `tokio-postgres`, `tokio`, `chrono`, `anyhow`.

```rust
loop {
    let strategy = client.query_opt(
        "SELECT optimizer, format, hidden, lr::float8, seed, steps, \
                status, generation \
         FROM ssot.scarab_strategy WHERE service_id=$1",
        &[&service_id]).await?;

    // write heartbeat (always)
    client.execute(
        "INSERT INTO ssot.scarab_heartbeat (service_id, last_seen, ...) \
         VALUES (...) ON CONFLICT (service_id) DO UPDATE SET ...", ...).await?;

    if strategy.status == "stop" { graceful_shutdown(); return Ok(()); }

    if strategy.generation > current_gen {
        kill_running_trainer();
        spawn_trainer(strategy);   // sets TRIOS_FORMAT_TYPE, --optimizer, ...
        current_gen = strategy.generation;
    }

    tokio::time::sleep(Duration::from_secs(poll_sec)).await;
}
```

## R5 evidence (`/home/user/workspace/cron_tracking/sovereign_scarab/evidence/2026-05-14T09:42Z`)

| metric | value |
|---|---|
| Postgres | 17.9 local, unix socket `/tmp/pg_scarab`, port 55432 |
| Scarabs launched | local-A (adamw/fp32/seed=47), local-B (muon/gf16/seed=144), local-C (muon-cwd/bf16/seed=89) |
| poll_sec | 5 (prod default: 30) |
| Baseline | T+8s: all 3 heartbeats current_gen=1 |
| Pull-up test | T0: `SELECT bump_strategy('local-A', muon-cwd, gf16, seed=144)` → strategy.generation 1→2 |
| Pull-up latency | T+5s: A.current_gen=2; B/C still 1 (correctly untouched) |
| Graceful stop | UPDATE status='stop'+bump → 3 scarabs gracefully exited in ≤ 5s |
| Tokens used | exactly one: `DATABASE_URL` |
| Railway API calls | 0 |

Evidence files committed at the path above: `strategy.csv`,
`heartbeat.csv`, `schema.sql`, `A.log`, `B.log`, `C.log`.

## Consequences

### Positive
- **Token hell is eliminated for the command path.** Only `DATABASE_URL`
  is required, set once at service-create time, never rotated.
- Scarabs are **self-correcting**: if Postgres is unreachable they
  back-off and retry — no GH-Actions workflow needed.
- **The Queen-Hive becomes a SQL client**, not a GraphQL/PAT
  orchestrator. Any agent with `DATABASE_URL` and `UPDATE` on
  `ssot.scarab_strategy` can command the fleet.
- **Heartbeat row is `last_seen`-stale-indexed**: a 10-line SQL view
  surfaces dead scarabs without grepping Railway logs.
- Decouples **strategy change** (UPDATE row, gen++) from **deploy**
  (no redeploy needed — scarab kills its own subprocess and respawns
  with new params).
- Mechanically simpler than `tri-gardener` for the steady-state
  command path; `tri-gardener` keeps its observatory + plateau-detection
  role.

### Negative
- Postgres `phd-postgres-ssot` becomes a hot dependency for control
  plane (it was already hot as SSOT for `bpb_samples`). Mitigation: it
  already has 99.x% uptime; add a `scarab_heartbeat_stale` exporter for
  alerting.
- Scarabs no longer obey `gh workflow run …` for retraining; they obey
  `UPDATE`. Mitigation: keep a thin GH-Actions workflow
  `scarab-strategy-update.yml` that wraps the SQL UPDATE for operators
  who prefer the CI/CD UI.
- One token (`DATABASE_URL`) is still a single point of failure.
  Mitigation: Railway's Neon plugin auto-injects it; rotation is
  database-level, not surface-level.

### Neutral
- Existing Railway-only flows (`writer-env-fix.yml`,
  `mcp-emergency-redeploy.yml`) become **diagnostic-only** — kept for
  PASS-N healer cron but no longer the primary command path.
- The PASS-N audit cron and HEALER cron now have a richer signal:
  `scarab_heartbeat.last_seen` per service, vs the single global
  `bpb_samples.MAX(ts)`.

## Migration plan

1. **R1** Local prototype (this ADR's evidence) — ✅ done 2026-05-14.
2. **R2** Bake schema into `phd-postgres-ssot`. Migration tracked at
   `trios-railway#142` (companion PR).
3. **R3** Rust crate `crates/scarab-pull-loop/` in
   `trios-trainer-igla`. Companion PR `trios-trainer-igla#142`.
4. **R4** Deploy 1 scarab to Railway `IGLA project`, watch heartbeat,
   bump strategy, verify in-cluster.
5. **R5** Migrate 3 scarabs (acc0/acc1/acc2 one each), keep 35 Railway-
   driven scarabs on the old path as A/B control.
6. **R6** When 3 sovereign scarabs are stable for 7 days, migrate the
   full fleet.

## Cross-refs

- HEALER cron post-mortem of B-20: `cron_tracking/81985399/runs/`
- LOCAL race baseline that validated trainer plumbing under all 11
  formats: `/tmp/local_race/canonical_leaderboard.tsv`
- Sibling skill that codifies the LOCAL multi-agent race pattern:
  `leaderboard-snapshot v1.9` (skill_id `59958c6f-e3f9-4b55-9f37-abf169d26dd5`)
- HEALER skill v1.1 with `DATABASE_URL` (not `NEON_DATABASE_URL`)
  correction: `skills/user/docker-writer-revive/SKILL.md`

`phi^2 + phi^-2 = 3 · TRINITY · SOVEREIGN-SCARAB · DEFENSE 2026-06-15`
